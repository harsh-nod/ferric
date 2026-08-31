//! Closed M1 prepublication join from scheduler roster through fixed batch.
//!
//! This layer binds the scheduler-issued live prefix to the exact validated
//! workspace inputs, generated plan identities, pending KV reservations, and
//! physical fixed-batch custody before a queue can publish any packet. It does
//! not create or submit a queue and makes no hardware-execution claim.

use core::fmt;

use ferric_spec::{
    Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, RequestId,
    StepPlan, ValidatedM1StepInputs, M1_MAX_ACTIVE_SEQUENCES,
};

use crate::{
    bind_m1_physical_buffer_ranges_v1, build_m1_physical_fixed_batch_v1,
    compose_m1_step_workspace_image_v1, AddresslessM1PhysicalBufferRecipeV1,
    BoundM1CompletionOutputV1, BoundM1KvWorkspaceTableV1,
    BoundM1SpeculativeDraftKvRoundWorkspaceTableV1, ComposedM1FullStepWorkspaceSetV1,
    ComposedM1StepWorkspaceImageV1, ContentBoundM1ProgramCatalogV1, Gfx942DeviceBinding,
    InitializedM1FullStepWorkspaceAllocationFailureV1,
    InitializedM1FullStepWorkspacePreflightErrorV1, LogicalRunnerDeclaration,
    M1FullStepWorkspaceImagesV1, M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans,
    M1FullStepWorkspaceSubleaseOwners, M1KvWorkspaceReservationCustodyV1,
    M1PartitionedModelMemoryKvPoolV1, M1PhysicalBufferBindingErrorV1,
    M1PhysicalBufferBindingFailureV1, M1PhysicalFixedBatchBuildErrorV1,
    M1PhysicalFixedBatchBuildFailureV1, M1PhysicalFixedBatchShapeV1, M1PhysicalFixedBatchV1,
    M1ScheduledDispatchV1, M1SpeculativeDraftKvRoundReservationCustodyV1,
    M1StepWorkspaceImageCompositionFailureV1, M1StepWorkspaceImageCompositionOutcomeV1,
    PendingDeviceKvStepWrite,
};

const MAX_LANES: usize = M1_MAX_ACTIVE_SEQUENCES as usize;

/// Closed full-step KV workspace-table shape supplied to prepublication.
///
/// ```compile_fail
/// use ferric_engine::M1FullStepKvWorkspaceTablesV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1FullStepKvWorkspaceTablesV1>();
/// ```
#[must_use = "validated workspace tables and pending reservations remain linear"]
#[derive(Debug, Eq, PartialEq)]
pub enum M1FullStepKvWorkspaceTablesV1 {
    /// One target-only workspace table.
    TargetOnly { target: BoundM1KvWorkspaceTableV1 },
    /// Draft and target prefill workspace tables.
    PairedPrefill {
        draft: BoundM1KvWorkspaceTableV1,
        target: BoundM1KvWorkspaceTableV1,
    },
    /// Aggregate draft-round and target-verification workspace tables.
    SpeculativeRound {
        draft_decode: BoundM1SpeculativeDraftKvRoundWorkspaceTableV1,
        target_speculative: BoundM1KvWorkspaceTableV1,
    },
}

impl M1FullStepKvWorkspaceTablesV1 {
    /// Returns the exact closed table shape.
    #[must_use]
    pub const fn kind(&self) -> M1FullStepWorkspaceInputKind {
        match self {
            Self::TargetOnly { .. } => M1FullStepWorkspaceInputKind::TargetOnly,
            Self::PairedPrefill { .. } => M1FullStepWorkspaceInputKind::PairedPrefill,
            Self::SpeculativeRound { .. } => M1FullStepWorkspaceInputKind::SpeculativeRound,
        }
    }
}

/// Pending KV reservation custody retained across allocation, queue, and readback.
///
/// ```compile_fail
/// use ferric_engine::M1FullStepKvReservationCustodyV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1FullStepKvReservationCustodyV1>();
/// ```
#[must_use = "pending KV reservations must remain retained until explicitly settled"]
#[derive(Debug, Eq, PartialEq)]
pub enum M1FullStepKvReservationCustodyV1 {
    /// Target-only pending reservations.
    TargetOnly {
        target: M1KvWorkspaceReservationCustodyV1,
    },
    /// Draft and target prefill pending reservations.
    PairedPrefill {
        draft: M1KvWorkspaceReservationCustodyV1,
        target: M1KvWorkspaceReservationCustodyV1,
    },
    /// Aggregate draft-round and target-verification pending reservations.
    SpeculativeRound {
        draft_decode: M1SpeculativeDraftKvRoundReservationCustodyV1,
        target_speculative: M1KvWorkspaceReservationCustodyV1,
    },
}

impl M1FullStepKvReservationCustodyV1 {
    /// Exact target selection retained by the reservation set.
    #[must_use]
    pub const fn target_selection(&self) -> Qwen3PlanSelection {
        match self {
            Self::TargetOnly { target } | Self::PairedPrefill { target, .. } => target.selection(),
            Self::SpeculativeRound {
                target_speculative, ..
            } => target_speculative.selection(),
        }
    }

    /// Exact target KV-arena allocation identity.
    #[must_use]
    pub const fn target_allocation_id(&self) -> Identity {
        match self {
            Self::TargetOnly { target } | Self::PairedPrefill { target, .. } => {
                target.allocation_id()
            }
            Self::SpeculativeRound {
                target_speculative, ..
            } => target_speculative.allocation_id(),
        }
    }

    /// Exact draft KV-arena allocation identity when the step uses draft KV.
    #[must_use]
    pub const fn draft_allocation_id(&self) -> Option<Identity> {
        match self {
            Self::TargetOnly { .. } => None,
            Self::PairedPrefill { draft, .. } => Some(draft.allocation_id()),
            Self::SpeculativeRound { draft_decode, .. } => Some(draft_decode.allocation_id()),
        }
    }
}

/// Stable rejection reason before workspace composition consumes any input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PrepublicationJoinErrorV1 {
    /// Workspace plans and KV tables use different closed shapes.
    InputKind {
        plans: M1FullStepWorkspaceInputKind,
        tables: M1FullStepWorkspaceInputKind,
    },
    /// A workspace position retained a selection outside its closed shape.
    Selection {
        role: Qwen3ModelRole,
        expected: Qwen3PlanSelection,
        actual: Qwen3PlanSelection,
    },
    /// The validated input prefix differs from the scheduler count.
    MemberCount { expected: usize, actual: usize },
    /// A live validated lane was unexpectedly absent.
    MissingLane { role: Qwen3ModelRole, lane: usize },
    /// A validated lane changed scheduler order, slot, or generation.
    RequestOrder {
        role: Qwen3ModelRole,
        lane: usize,
        expected: RequestId,
        actual: RequestId,
    },
    /// A validated lane changed the scheduler-issued epoch.
    CompletionEpoch { role: Qwen3ModelRole, lane: usize },
    /// A validated lane retained another finite selection.
    LaneSelection { role: Qwen3ModelRole, lane: usize },
    /// A validated lane retained a plan identity outside the published runner.
    PlanIdentity { role: Qwen3ModelRole, lane: usize },
    /// Draft and target live lanes retained different pre-step contexts.
    ContextLength {
        lane: usize,
        draft: u32,
        target: u32,
    },
    /// Draft and target prefill lanes retained different active widths.
    ActiveLength {
        lane: usize,
        draft: u32,
        target: u32,
    },
    /// Draft and target inputs retained different token values.
    Token { lane: usize, column: usize },
    /// Draft and target inputs retained different position values.
    Position { lane: usize, column: usize },
    /// A speculative draft-round table changed target selection or K.
    SpeculativeWidth,
}

impl fmt::Display for M1PrepublicationJoinErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 prepublication join rejected: {self:?}")
    }
}

impl std::error::Error for M1PrepublicationJoinErrorV1 {}

/// Pure rejection retaining the exact scheduler, plans, and KV tables unchanged.
#[must_use = "all unchanged prepublication inputs remain recoverable"]
#[derive(Debug)]
pub struct M1PrepublicationJoinFailureV1 {
    error: M1PrepublicationJoinErrorV1,
    scheduled: Box<M1ScheduledDispatchV1>,
    plans: Box<M1FullStepWorkspacePlans>,
    tables: Box<M1FullStepKvWorkspaceTablesV1>,
}

impl M1PrepublicationJoinFailureV1 {
    /// Returns the stable rejection reason.
    #[must_use]
    pub const fn error(&self) -> M1PrepublicationJoinErrorV1 {
        self.error
    }

    /// Recovers every unchanged input exactly once.
    #[must_use = "every exact rejected input remains retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1PrepublicationJoinErrorV1,
        M1ScheduledDispatchV1,
        M1FullStepWorkspacePlans,
        M1FullStepKvWorkspaceTablesV1,
    ) {
        (self.error, *self.scheduled, *self.plans, *self.tables)
    }
}

/// One workspace slot after an all-slots composition attempt.
#[must_use = "composed or rejected workspace inputs remain retained"]
#[derive(Debug)]
pub enum M1WorkspaceImageResidueV1 {
    /// Composition succeeded and retains the exact plan and image.
    Composed(ComposedM1StepWorkspaceImageV1),
    /// Composition rejected and retains the exact unchanged inputs.
    Rejected(M1StepWorkspaceImageCompositionFailureV1),
}

/// Composition failure retaining every slot, scheduler authority, and KV reservation.
#[must_use = "closed composition residue and pending authority require explicit handling"]
#[derive(Debug)]
pub struct M1PrepublicationCompositionFailureV1 {
    scheduled: Box<M1ScheduledDispatchV1>,
    target_plans: Box<[Option<StepPlan>; MAX_LANES]>,
    kv: Box<M1FullStepKvReservationCustodyV1>,
    residue: Box<[M1WorkspaceImageResidueV1]>,
}

impl M1PrepublicationCompositionFailureV1 {
    /// Scheduler authority retained unchanged.
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    /// All slot results in deterministic draft-then-target order.
    pub fn residue(&self) -> &[M1WorkspaceImageResidueV1] {
        &self.residue
    }

    /// Recovers every retained owner exactly once.
    #[must_use = "every composition residue and authority owner remains retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1ScheduledDispatchV1,
        [Option<StepPlan>; MAX_LANES],
        M1FullStepKvReservationCustodyV1,
        Box<[M1WorkspaceImageResidueV1]>,
    ) {
        (*self.scheduled, *self.target_plans, *self.kv, self.residue)
    }
}

/// Scheduler and request-specific authority retained through physical execution.
///
/// ```compile_fail
/// use ferric_engine::M1PrepublicationStepCustodyV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1PrepublicationStepCustodyV1>();
/// ```
#[must_use = "scheduler, exact target plans, and pending KV reservations remain linear"]
#[derive(Debug)]
pub struct M1PrepublicationStepCustodyV1 {
    scheduled: M1ScheduledDispatchV1,
    target_plans: [Option<StepPlan>; MAX_LANES],
    kv: M1FullStepKvReservationCustodyV1,
}

impl M1PrepublicationStepCustodyV1 {
    /// Exact scheduler-issued roster.
    pub const fn scheduled_dispatch(&self) -> &M1ScheduledDispatchV1 {
        &self.scheduled
    }

    /// Exact target `StepPlan` prefix with canonical `None` padding.
    #[must_use]
    pub const fn target_plans(&self) -> &[Option<StepPlan>; MAX_LANES] {
        &self.target_plans
    }

    pub(crate) fn target_active_lengths(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        let target = match &self.kv {
            M1FullStepKvReservationCustodyV1::TargetOnly { target }
            | M1FullStepKvReservationCustodyV1::PairedPrefill { target, .. } => target,
            M1FullStepKvReservationCustodyV1::SpeculativeRound {
                target_speculative, ..
            } => target_speculative,
        };
        target
            .reservations()
            .iter()
            .map(PendingDeviceKvStepWrite::active_tokens)
    }

    /// Pending KV reservation custody.
    pub const fn kv_reservations(&self) -> &M1FullStepKvReservationCustodyV1 {
        &self.kv
    }

    /// Recovers all three exact owners.
    #[must_use = "scheduler, target-plan, and KV custody all remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1ScheduledDispatchV1,
        [Option<StepPlan>; MAX_LANES],
        M1FullStepKvReservationCustodyV1,
    ) {
        (self.scheduled, self.target_plans, self.kv)
    }
}

/// Composed workspace images joined to scheduler and KV authority.
#[must_use = "prepared images must proceed to allocation or be retained"]
#[derive(Debug)]
pub struct M1PreparedScheduledWorkspaceImagesV1 {
    plans: M1FullStepWorkspacePlans,
    images: M1FullStepWorkspaceImagesV1,
    step: M1PrepublicationStepCustodyV1,
}

impl M1PreparedScheduledWorkspaceImagesV1 {
    /// Exact prepared workspace shape.
    #[must_use]
    pub const fn kind(&self) -> M1FullStepWorkspaceInputKind {
        self.plans.kind()
    }

    /// Scheduler/KV authority retained beside the images.
    pub const fn step(&self) -> &M1PrepublicationStepCustodyV1 {
        &self.step
    }

    pub(crate) const fn plans(&self) -> &M1FullStepWorkspacePlans {
        &self.plans
    }

    pub(crate) fn into_rearm_parts(
        self,
    ) -> (
        M1FullStepWorkspacePlans,
        M1FullStepWorkspaceImagesV1,
        M1PrepublicationStepCustodyV1,
    ) {
        (self.plans, self.images, self.step)
    }
}

/// Allocated workspace owners joined to scheduler and KV authority.
///
/// Completion host ranges are intentionally available only from this phase,
/// after the final device workspace has fixed the generic allocation ordering.
///
/// ```compile_fail
/// use ferric_engine::M1PartitionedModelMemoryKvPoolV1;
/// use ferric_spec::Qwen3PlanSelection;
///
/// fn allocate_too_early(
///     memory: &mut M1PartitionedModelMemoryKvPoolV1,
///     selection: Qwen3PlanSelection,
/// ) {
///     let _ = memory.allocate_completion_output(selection);
/// }
/// ```
#[must_use = "allocated prepublication custody must enter physical batch construction"]
#[derive(Debug)]
pub struct M1AllocatedScheduledStepV1 {
    workspace_owners: M1FullStepWorkspaceSubleaseOwners,
    partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    step: M1PrepublicationStepCustodyV1,
}

impl M1AllocatedScheduledStepV1 {
    /// Checked physical-device receipt retained with allocated step custody.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.partitioned_memory.device()
    }

    /// Exact workspace allocation owners.
    pub const fn workspace_owners(&self) -> &M1FullStepWorkspaceSubleaseOwners {
        &self.workspace_owners
    }

    /// Exact scheduler/KV authority.
    pub const fn step(&self) -> &M1PrepublicationStepCustodyV1 {
        &self.step
    }

    /// Exact closed model-memory, allocation, partition, and page-ledger owner.
    #[must_use = "partitioned memory custody remains retained through prepublication"]
    pub const fn partitioned_memory(&self) -> &M1PartitionedModelMemoryKvPoolV1 {
        &self.partitioned_memory
    }

    /// Allocates the ordinary compact output after every device workspace.
    ///
    /// This ordering keeps the generic host-range data index stable through
    /// queue construction because this allocated-step phase cannot append
    /// another device allocation.
    ///
    /// # Errors
    ///
    /// Returns the exact compact-output allocation diagnostic while retaining
    /// this allocated step unchanged apart from the allocation session's
    /// documented failure state.
    pub fn allocate_completion_output(
        &mut self,
        selection: Qwen3PlanSelection,
    ) -> Result<BoundM1CompletionOutputV1, crate::M1CompletionOutputErrorV1> {
        self.partitioned_memory
            .allocate_completion_output(selection)
    }

    /// Allocates the guarded compact output after every device workspace.
    ///
    /// # Errors
    ///
    /// Returns the exact guarded-output allocation diagnostic while retaining
    /// allocated-step custody.
    pub fn allocate_guarded_completion_output(
        &mut self,
        selection: Qwen3PlanSelection,
    ) -> Result<BoundM1CompletionOutputV1, crate::M1CompletionOutputErrorV1> {
        self.partitioned_memory
            .allocate_guarded_completion_output(selection)
    }

    /// Preallocates the inactive compact output and independent diagnostic
    /// choices required by a future exact S1/K4 queue rollover.
    ///
    /// This must complete before first queue construction because the generic
    /// detached-queue API can replace, but cannot insert, host-visible
    /// allocations.
    ///
    /// # Errors
    ///
    /// Rejects repeated reservation or returns the exact host allocation
    /// failure while this allocated step retains the model/allocation pool.
    pub fn reserve_s1_k4_rollover_output(
        &mut self,
    ) -> Result<(), crate::M1S1K4RolloverOutputReserveErrorV1> {
        self.partitioned_memory.reserve_s1_k4_rollover_output()
    }

    /// Preallocates inactive outputs for every finite speculative successor.
    ///
    /// This catalog must be complete before first queue construction because
    /// the detached-queue rollover path may replace host allocations but
    /// cannot add a successor output after publication.
    ///
    /// # Errors
    ///
    /// Rejects repeated reservation or returns the exact host allocation
    /// failure while this allocated step retains the model/allocation pool.
    pub fn reserve_finite_speculative_rollover_outputs(
        &mut self,
    ) -> Result<(), crate::device_cache::M1FiniteSpeculativeRolloverOutputReserveErrorV1> {
        self.partitioned_memory
            .reserve_finite_speculative_rollover_outputs()
    }

    /// Attaches qualification logits without permitting another device allocation.
    ///
    /// # Errors
    ///
    /// Returns the exact attachment failure with compact-output custody.
    pub fn enable_qualification_logits_capture(
        &mut self,
        completion: BoundM1CompletionOutputV1,
    ) -> Result<BoundM1CompletionOutputV1, Box<crate::M1QualificationLogitsAllocationFailureV1>>
    {
        self.partitioned_memory
            .enable_qualification_logits_capture(completion)
    }

    /// Attaches finite speculative diagnostic choices after every device allocation.
    ///
    /// # Errors
    ///
    /// Returns the exact attachment failure with compact-output custody.
    pub fn enable_speculative_diagnostic_choices_capture(
        &mut self,
        completion: BoundM1CompletionOutputV1,
    ) -> Result<
        BoundM1CompletionOutputV1,
        Box<crate::M1SpeculativeDiagnosticChoicesAllocationFailureV1>,
    > {
        self.partitioned_memory
            .enable_speculative_diagnostic_choices_capture(completion)
    }

    /// Source-compatible S1/K4 entry point for diagnostic choice capture.
    ///
    /// # Errors
    ///
    /// Returns the exact attachment failure with compact-output custody.
    pub fn enable_speculative_k4_diagnostic_choices_capture(
        &mut self,
        completion: BoundM1CompletionOutputV1,
    ) -> Result<
        BoundM1CompletionOutputV1,
        Box<crate::M1SpeculativeDiagnosticChoicesAllocationFailureV1>,
    > {
        self.partitioned_memory
            .enable_speculative_k4_diagnostic_choices_capture(completion)
    }

    /// Attaches direct target-choice capture after every device allocation.
    ///
    /// # Errors
    ///
    /// Returns the exact attachment failure with compact-output custody.
    pub fn enable_direct_diagnostic_choices_capture(
        &mut self,
        completion: BoundM1CompletionOutputV1,
    ) -> Result<BoundM1CompletionOutputV1, Box<crate::M1DirectDiagnosticChoicesAllocationFailureV1>>
    {
        self.partitioned_memory
            .enable_direct_diagnostic_choices_capture(completion)
    }

    fn into_parts(
        self,
    ) -> (
        M1FullStepWorkspaceSubleaseOwners,
        M1PartitionedModelMemoryKvPoolV1,
        M1PrepublicationStepCustodyV1,
    ) {
        (self.workspace_owners, self.partitioned_memory, self.step)
    }
}

/// Allocation rejection retaining all authority that remains recoverable.
#[must_use = "allocation failure custody requires explicit recovery or teardown"]
#[derive(Debug)]
pub struct M1PrepublicationAllocationFailureV1 {
    failure: InitializedM1FullStepWorkspaceAllocationFailureV1,
    partitioned_memory: Box<M1PartitionedModelMemoryKvPoolV1>,
    step: Box<M1PrepublicationStepCustodyV1>,
}

impl M1PrepublicationAllocationFailureV1 {
    /// Checked physical-device receipt retained after allocation rejection.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.partitioned_memory.device()
    }

    /// Existing exact initialized-workspace failure.
    pub const fn source(&self) -> &InitializedM1FullStepWorkspaceAllocationFailureV1 {
        &self.failure
    }

    /// Scheduler and KV authority retained even after a terminal service failure.
    pub const fn step(&self) -> &M1PrepublicationStepCustodyV1 {
        &self.step
    }

    /// Closed allocation and partition custody retained after rejection.
    #[must_use = "partitioned memory custody remains retained by the failure"]
    pub const fn partitioned_memory(&self) -> &M1PartitionedModelMemoryKvPoolV1 {
        &self.partitioned_memory
    }

    /// Recovers the existing failure and retained step authority.
    #[must_use = "allocation failure and retained step authority both require handling"]
    pub fn into_parts(
        self,
    ) -> (
        InitializedM1FullStepWorkspaceAllocationFailureV1,
        M1PartitionedModelMemoryKvPoolV1,
        M1PrepublicationStepCustodyV1,
    ) {
        (self.failure, *self.partitioned_memory, *self.step)
    }

    /// Recovers the exact prepared input after pure allocation preflight rejection.
    ///
    /// Runtime failure returns this failure unchanged because one or more host
    /// images may already have been consumed by the service allocation path.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure after any runtime allocation or binding attempt.
    pub fn into_preflight_prepared(
        self,
    ) -> Result<
        (
            InitializedM1FullStepWorkspacePreflightErrorV1,
            M1PartitionedModelMemoryKvPoolV1,
            M1PreparedScheduledWorkspaceImagesV1,
        ),
        Self,
    > {
        let Self {
            failure,
            partitioned_memory,
            step,
        } = self;
        match failure.into_preflight_parts() {
            Ok((error, plans, images)) => Ok((
                error,
                *partitioned_memory,
                M1PreparedScheduledWorkspaceImagesV1 {
                    plans,
                    images,
                    step: *step,
                },
            )),
            Err(failure) => Err(Self {
                failure,
                partitioned_memory,
                step,
            }),
        }
    }
}

/// Builds and validates the complete scheduler-bound workspace images.
///
/// All roster, selection, epoch, context, K, and generated-plan checks run
/// before any linear input is dismantled. Composition then attempts every slot
/// so a rejection retains closed residue for the complete shape.
///
/// # Errors
///
/// Returns [`M1PrepareFailureV1`] with unchanged inputs for pure join rejection
/// or closed all-slot residue when byte-image composition rejects.
pub fn prepare_m1_scheduled_workspace_images_v1(
    scheduled: M1ScheduledDispatchV1,
    runner: &LogicalRunnerDeclaration,
    plans: M1FullStepWorkspacePlans,
    tables: M1FullStepKvWorkspaceTablesV1,
) -> Result<M1PreparedScheduledWorkspaceImagesV1, M1PrepareFailureV1> {
    let target_plans = match validate_join(&scheduled, runner, &plans, &tables) {
        Ok(plans) => plans,
        Err(error) => {
            return Err(M1PrepareFailureV1::Join(M1PrepublicationJoinFailureV1 {
                error,
                scheduled: Box::new(scheduled),
                plans: Box::new(plans),
                tables: Box::new(tables),
            }))
        }
    };
    let (outcomes, kv) = compose_all(plans, tables);
    match collect_composed(outcomes) {
        Ok(images) => {
            let (plans, images) = images.into_allocation_inputs();
            Ok(M1PreparedScheduledWorkspaceImagesV1 {
                plans,
                images,
                step: M1PrepublicationStepCustodyV1 {
                    scheduled,
                    target_plans,
                    kv,
                },
            })
        }
        Err(residue) => Err(M1PrepareFailureV1::Composition(
            M1PrepublicationCompositionFailureV1 {
                scheduled: Box::new(scheduled),
                target_plans: Box::new(target_plans),
                kv: Box::new(kv),
                residue,
            },
        )),
    }
}

/// Pure-join or composition failure from prepublication preparation.
#[must_use]
#[derive(Debug)]
pub enum M1PrepareFailureV1 {
    /// Pure validation rejection with every input unchanged.
    Join(M1PrepublicationJoinFailureV1),
    /// Composition rejection with closed all-slot residue.
    Composition(M1PrepublicationCompositionFailureV1),
}

/// Allocates the complete prepared workspace set while retaining step authority.
///
/// # Errors
///
/// Returns [`M1PrepublicationAllocationFailureV1`] with the exact existing
/// allocation failure and scheduler/KV custody.
pub fn allocate_m1_prepublication_workspaces_v1(
    mut partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    prepared: M1PreparedScheduledWorkspaceImagesV1,
) -> Result<M1AllocatedScheduledStepV1, M1PrepublicationAllocationFailureV1> {
    let M1PreparedScheduledWorkspaceImagesV1 {
        plans,
        images,
        step,
    } = prepared;
    match partitioned_memory.allocate_full_step_workspaces(plans, images) {
        Ok(workspace_owners) => Ok(M1AllocatedScheduledStepV1 {
            workspace_owners,
            partitioned_memory,
            step,
        }),
        Err(failure) => Err(M1PrepublicationAllocationFailureV1 {
            failure,
            partitioned_memory: Box::new(partitioned_memory),
            step: Box::new(step),
        }),
    }
}
/// Batch-construction failure retaining exact step authority and build inputs.
///
/// The `'static` catalog representation in the error enum is intentionally not
/// used by the public builder; see [`M1PrepublicationBatchBuildFailureV1`].
#[must_use = "batch construction failure retains step authority and exact lower inputs"]
#[derive(Debug)]
pub enum M1PrepublicationBatchBuildFailureV1<'a> {
    /// Pure validation rejection; all inputs are unchanged.
    Rejected {
        error: M1PrepublicationBatchBuildErrorKindV1,
        allocated: Box<M1AllocatedScheduledStepV1>,
        recipe: Box<AddresslessM1PhysicalBufferRecipeV1>,
        completion_output: Box<BoundM1CompletionOutputV1>,
        catalog: Box<ContentBoundM1ProgramCatalogV1<'a>>,
    },
    /// Existing owner-checked binding rejection retains its exact inputs.
    Binding {
        failure: Box<M1PhysicalBufferBindingFailureV1>,
        step: Box<M1PrepublicationStepCustodyV1>,
        catalog: Box<ContentBoundM1ProgramCatalogV1<'a>>,
    },
    /// Existing fixed-batch rejection retains its exact bindings and catalog.
    FixedBatch {
        failure: Box<M1PhysicalFixedBatchBuildFailureV1<'a>>,
        step: Box<M1PrepublicationStepCustodyV1>,
    },
}

/// Stable diagnostic recovered with exact retry-capable batch inputs.
#[derive(Debug)]
pub enum M1PrepublicationBatchBuildDiagnosticV1 {
    /// Pure prepublication batch preflight rejected.
    Preflight(M1PrepublicationBatchBuildErrorKindV1),
    /// Existing owner-checked physical-buffer binding rejected.
    Binding(M1PhysicalBufferBindingErrorV1),
    /// Existing physical fixed-batch construction rejected.
    FixedBatch(M1PhysicalFixedBatchBuildErrorV1),
}

impl<'a> M1PrepublicationBatchBuildFailureV1<'a> {
    /// Recovers one normalized set of exact inputs for a corrected retry.
    ///
    /// Both lower builders are transactional: their failures retain the
    /// original recipe and allocation owners. Derived bound rows are discarded
    /// while reconstructing the original prepublication inputs.
    #[must_use = "the diagnostic and every exact retry input remain retained"]
    #[allow(clippy::type_complexity)]
    pub fn into_retry_inputs(
        self,
    ) -> (
        M1PrepublicationBatchBuildDiagnosticV1,
        M1AllocatedScheduledStepV1,
        AddresslessM1PhysicalBufferRecipeV1,
        BoundM1CompletionOutputV1,
        ContentBoundM1ProgramCatalogV1<'a>,
    ) {
        match self {
            Self::Rejected {
                error,
                allocated,
                recipe,
                completion_output,
                catalog,
            } => (
                M1PrepublicationBatchBuildDiagnosticV1::Preflight(error),
                *allocated,
                *recipe,
                *completion_output,
                *catalog,
            ),
            Self::Binding {
                failure,
                step,
                catalog,
            } => {
                let (error, recipe, workspace_owners, partitioned_memory, completion_output) =
                    failure.into_parts();
                (
                    M1PrepublicationBatchBuildDiagnosticV1::Binding(error),
                    M1AllocatedScheduledStepV1 {
                        workspace_owners,
                        partitioned_memory,
                        step: *step,
                    },
                    recipe,
                    completion_output,
                    *catalog,
                )
            }
            Self::FixedBatch { failure, step } => {
                let (error, catalog, bindings) = failure.into_parts();
                let (recipe, workspace_owners, partitioned_memory, completion_output, _bound_rows) =
                    bindings.into_parts();
                (
                    M1PrepublicationBatchBuildDiagnosticV1::FixedBatch(error),
                    M1AllocatedScheduledStepV1 {
                        workspace_owners,
                        partitioned_memory,
                        step: *step,
                    },
                    recipe,
                    completion_output,
                    catalog,
                )
            }
        }
    }
}

/// Copyable pure preflight diagnostic for fixed-batch construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PrepublicationBatchBuildErrorKindV1 {
    /// Physical recipe and scheduler-bound target selection differ.
    Selection {
        expected: Qwen3PlanSelection,
        actual: Qwen3PlanSelection,
    },
    /// Completion-output capacity does not cover exactly the scheduler prefix.
    CompletionCapacity { members: usize, capacity: usize },
    /// Pending target KV reservations name another model-memory arena.
    TargetKvArena {
        expected: Identity,
        actual: Identity,
    },
    /// Pending draft KV reservations name another model-memory arena.
    DraftKvArena {
        expected: Identity,
        actual: Identity,
    },
    /// One target reservation page is not backed by the retained lease ledger.
    TargetKvPage { lane: usize, logical_page: u32 },
    /// One draft reservation page is not backed by the retained lease ledger.
    DraftKvPage { lane: usize, logical_page: u32 },
}

/// Opaque queue-creation input proving the prepublication join completed.
///
/// ```compile_fail
/// use ferric_engine::M1PrepublicationBatchV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1PrepublicationBatchV1<'static>>();
/// ```
#[must_use = "the only publication-capable input must enter queue custody"]
#[derive(Debug)]
pub struct M1PrepublicationBatchV1<'a> {
    pub(crate) batch: M1PhysicalFixedBatchV1<'a>,
    pub(crate) step: M1PrepublicationStepCustodyV1,
}

impl M1PrepublicationBatchV1<'_> {
    /// Checked physical-device receipt retained before queue creation.
    #[must_use]
    pub const fn device(&self) -> Gfx942DeviceBinding {
        self.batch.device()
    }

    /// Exact closed physical batch shape.
    #[must_use]
    pub const fn shape(&self) -> M1PhysicalFixedBatchShapeV1 {
        self.batch.shape()
    }

    /// Exact scheduler and KV authority joined to the batch.
    pub const fn step(&self) -> &M1PrepublicationStepCustodyV1 {
        &self.step
    }
}

/// Owner-checks all physical ranges and constructs the sole queue input.
///
/// The allocated input already owns the closed model-memory/KV pool. Every
/// retained reservation page is revalidated against that pool before physical
/// binding consumes it into fixed-batch custody.
///
/// # Errors
///
/// Returns [`M1PrepublicationBatchBuildFailureV1`] with unchanged inputs for
/// pure preflight rejection or the exact existing binding/build failure plus
/// retained step authority.
pub fn build_m1_prepublication_batch_v1(
    allocated: M1AllocatedScheduledStepV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    completion_output: BoundM1CompletionOutputV1,
    catalog: ContentBoundM1ProgramCatalogV1<'_>,
) -> Result<M1PrepublicationBatchV1<'_>, M1PrepublicationBatchBuildFailureV1<'_>> {
    if let Err(error) = validate_batch_inputs(&allocated, &recipe, &completion_output) {
        return Err(M1PrepublicationBatchBuildFailureV1::Rejected {
            error,
            allocated: Box::new(allocated),
            recipe: Box::new(recipe),
            completion_output: Box::new(completion_output),
            catalog: Box::new(catalog),
        });
    }
    let (workspace_owners, partitioned_memory, step) = allocated.into_parts();
    let bindings = match bind_m1_physical_buffer_ranges_v1(
        recipe,
        workspace_owners,
        partitioned_memory,
        completion_output,
    ) {
        Ok(bindings) => bindings,
        Err(failure) => {
            return Err(M1PrepublicationBatchBuildFailureV1::Binding {
                failure: Box::new(failure),
                step: Box::new(step),
                catalog: Box::new(catalog),
            })
        }
    };
    let batch = match build_m1_physical_fixed_batch_v1(catalog, bindings) {
        Ok(batch) => batch,
        Err(failure) => {
            return Err(M1PrepublicationBatchBuildFailureV1::FixedBatch {
                failure: Box::new(failure),
                step: Box::new(step),
            })
        }
    };
    Ok(M1PrepublicationBatchV1 { batch, step })
}

fn validate_batch_inputs(
    allocated: &M1AllocatedScheduledStepV1,
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    completion_output: &BoundM1CompletionOutputV1,
) -> Result<(), M1PrepublicationBatchBuildErrorKindV1> {
    let expected = allocated.step.kv.target_selection();
    let actual = recipe
        .workspace_composition()
        .dispatch_plan()
        .intent()
        .target_selection();
    if expected != actual {
        return Err(M1PrepublicationBatchBuildErrorKindV1::Selection { expected, actual });
    }
    validate_completion_capacity(
        allocated.step.scheduled.member_count(),
        completion_output.shape().sequences() as usize,
    )?;
    let expected_target = allocated
        .partitioned_memory
        .allocation_id(Qwen3ModelRole::Target8B);
    let actual_target = allocated.step.kv.target_allocation_id();
    validate_kv_arena_identity(Qwen3ModelRole::Target8B, expected_target, actual_target)?;
    if let Some(actual_draft) = allocated.step.kv.draft_allocation_id() {
        let expected_draft = allocated
            .partitioned_memory
            .allocation_id(Qwen3ModelRole::Draft06B);
        validate_kv_arena_identity(Qwen3ModelRole::Draft06B, expected_draft, actual_draft)?;
    }
    validate_partitioned_page_custody(&allocated.partitioned_memory, &allocated.step.kv)?;
    Ok(())
}

fn validate_completion_capacity(
    members: usize,
    capacity: usize,
) -> Result<(), M1PrepublicationBatchBuildErrorKindV1> {
    if members == 0 || members > capacity {
        Err(M1PrepublicationBatchBuildErrorKindV1::CompletionCapacity { members, capacity })
    } else {
        Ok(())
    }
}

fn validate_kv_arena_identity(
    role: Qwen3ModelRole,
    expected: Identity,
    actual: Identity,
) -> Result<(), M1PrepublicationBatchBuildErrorKindV1> {
    if expected == actual {
        Ok(())
    } else if role == Qwen3ModelRole::Target8B {
        Err(M1PrepublicationBatchBuildErrorKindV1::TargetKvArena { expected, actual })
    } else {
        Err(M1PrepublicationBatchBuildErrorKindV1::DraftKvArena { expected, actual })
    }
}

fn validate_partitioned_page_custody(
    partitioned_memory: &M1PartitionedModelMemoryKvPoolV1,
    custody: &M1FullStepKvReservationCustodyV1,
) -> Result<(), M1PrepublicationBatchBuildErrorKindV1> {
    match custody {
        M1FullStepKvReservationCustodyV1::TargetOnly { target } => {
            validate_regular_reservations(partitioned_memory, target, true)
        }
        M1FullStepKvReservationCustodyV1::PairedPrefill { draft, target } => {
            validate_regular_reservations(partitioned_memory, draft, false)?;
            validate_regular_reservations(partitioned_memory, target, true)
        }
        M1FullStepKvReservationCustodyV1::SpeculativeRound {
            draft_decode,
            target_speculative,
        } => {
            for (lane, aggregate) in draft_decode.reservations().iter().enumerate() {
                validate_pending_pages(
                    partitioned_memory,
                    aggregate.pending_step_write(),
                    lane,
                    false,
                )?;
            }
            validate_regular_reservations(partitioned_memory, target_speculative, true)
        }
    }
}

fn validate_regular_reservations(
    partitioned_memory: &M1PartitionedModelMemoryKvPoolV1,
    custody: &M1KvWorkspaceReservationCustodyV1,
    target: bool,
) -> Result<(), M1PrepublicationBatchBuildErrorKindV1> {
    for (lane, pending) in custody.reservations().iter().enumerate() {
        validate_pending_pages(partitioned_memory, pending, lane, target)?;
    }
    Ok(())
}

fn validate_pending_pages(
    partitioned_memory: &M1PartitionedModelMemoryKvPoolV1,
    pending: &crate::PendingDeviceKvStepWrite,
    lane: usize,
    target: bool,
) -> Result<(), M1PrepublicationBatchBuildErrorKindV1> {
    for identity in pending.page_table() {
        let expected_role = if target {
            Qwen3ModelRole::Target8B
        } else {
            Qwen3ModelRole::Draft06B
        };
        if identity.page().role() != expected_role
            || partitioned_memory
                .validate_page_identity(
                    pending.request(),
                    identity.allocation_id(),
                    identity.page(),
                )
                .is_err()
        {
            return Err(if target {
                M1PrepublicationBatchBuildErrorKindV1::TargetKvPage {
                    lane,
                    logical_page: identity.logical_page(),
                }
            } else {
                M1PrepublicationBatchBuildErrorKindV1::DraftKvPage {
                    lane,
                    logical_page: identity.logical_page(),
                }
            });
        }
    }
    Ok(())
}

type ComposeOutcome = M1StepWorkspaceImageCompositionOutcomeV1;

enum FullComposeOutcomes {
    TargetOnly(ComposeOutcome),
    PairedPrefill(ComposeOutcome, ComposeOutcome),
    SpeculativeRound(ComposeOutcome, ComposeOutcome),
}

fn compose_all(
    plans: M1FullStepWorkspacePlans,
    tables: M1FullStepKvWorkspaceTablesV1,
) -> (FullComposeOutcomes, M1FullStepKvReservationCustodyV1) {
    match (plans, tables) {
        (
            M1FullStepWorkspacePlans::TargetOnly { target: plan },
            M1FullStepKvWorkspaceTablesV1::TargetOnly { target },
        ) => {
            let (inputs, pages, reservations) = target.into_workspace_image_parts();
            (
                FullComposeOutcomes::TargetOnly(compose_m1_step_workspace_image_v1(
                    *plan, inputs, pages,
                )),
                M1FullStepKvReservationCustodyV1::TargetOnly {
                    target: reservations,
                },
            )
        }
        (
            M1FullStepWorkspacePlans::PairedPrefill {
                draft: draft_plan,
                target: target_plan,
            },
            M1FullStepKvWorkspaceTablesV1::PairedPrefill { draft, target },
        ) => {
            let (draft_inputs, draft_pages, draft_reservations) =
                draft.into_workspace_image_parts();
            let (target_inputs, target_pages, target_reservations) =
                target.into_workspace_image_parts();
            (
                FullComposeOutcomes::PairedPrefill(
                    compose_m1_step_workspace_image_v1(*draft_plan, draft_inputs, draft_pages),
                    compose_m1_step_workspace_image_v1(*target_plan, target_inputs, target_pages),
                ),
                M1FullStepKvReservationCustodyV1::PairedPrefill {
                    draft: draft_reservations,
                    target: target_reservations,
                },
            )
        }
        (
            M1FullStepWorkspacePlans::SpeculativeRound {
                draft_decode: draft_plan,
                target_speculative: target_plan,
            },
            M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
                draft_decode,
                target_speculative,
            },
        ) => {
            let (draft_inputs, draft_pages, draft_reservations) =
                draft_decode.into_workspace_image_parts();
            let (target_inputs, target_pages, target_reservations) =
                target_speculative.into_workspace_image_parts();
            (
                FullComposeOutcomes::SpeculativeRound(
                    compose_m1_step_workspace_image_v1(*draft_plan, draft_inputs, draft_pages),
                    compose_m1_step_workspace_image_v1(*target_plan, target_inputs, target_pages),
                ),
                M1FullStepKvReservationCustodyV1::SpeculativeRound {
                    draft_decode: draft_reservations,
                    target_speculative: target_reservations,
                },
            )
        }
        _ => unreachable!("preflight accepts only matching closed shapes"),
    }
}

fn collect_composed(
    outcomes: FullComposeOutcomes,
) -> Result<ComposedM1FullStepWorkspaceSetV1, Box<[M1WorkspaceImageResidueV1]>> {
    match outcomes {
        FullComposeOutcomes::TargetOnly(target) => match target {
            ComposeOutcome::Composed(target) => {
                Ok(ComposedM1FullStepWorkspaceSetV1::target_only(target))
            }
            rejected @ ComposeOutcome::Rejected(_) => {
                Err(vec![into_residue(rejected)].into_boxed_slice())
            }
        },
        FullComposeOutcomes::PairedPrefill(draft, target) => match (draft, target) {
            (ComposeOutcome::Composed(draft), ComposeOutcome::Composed(target)) => Ok(
                ComposedM1FullStepWorkspaceSetV1::paired_prefill(draft, target),
            ),
            (draft, target) => {
                Err(vec![into_residue(draft), into_residue(target)].into_boxed_slice())
            }
        },
        FullComposeOutcomes::SpeculativeRound(draft, target) => match (draft, target) {
            (ComposeOutcome::Composed(draft), ComposeOutcome::Composed(target)) => Ok(
                ComposedM1FullStepWorkspaceSetV1::speculative_round(draft, target),
            ),
            (draft, target) => {
                Err(vec![into_residue(draft), into_residue(target)].into_boxed_slice())
            }
        },
    }
}

fn into_residue(outcome: ComposeOutcome) -> M1WorkspaceImageResidueV1 {
    match outcome {
        ComposeOutcome::Composed(image) => M1WorkspaceImageResidueV1::Composed(image),
        ComposeOutcome::Rejected(failure) => M1WorkspaceImageResidueV1::Rejected(failure),
    }
}

fn validate_join(
    scheduled: &M1ScheduledDispatchV1,
    runner: &LogicalRunnerDeclaration,
    plans: &M1FullStepWorkspacePlans,
    tables: &M1FullStepKvWorkspaceTablesV1,
) -> Result<[Option<StepPlan>; MAX_LANES], M1PrepublicationJoinErrorV1> {
    if plans.kind() != tables.kind() {
        return Err(M1PrepublicationJoinErrorV1::InputKind {
            plans: plans.kind(),
            tables: tables.kind(),
        });
    }
    match (plans, tables) {
        (
            M1FullStepWorkspacePlans::TargetOnly { target: plan },
            M1FullStepKvWorkspaceTablesV1::TargetOnly { target },
        ) => {
            validate_exact_selection(
                Qwen3ModelRole::Target8B,
                plan.selection(),
                target.selection(),
            )?;
            validate_inputs(
                scheduled,
                runner,
                Qwen3ModelRole::Target8B,
                target.inputs(),
                plan.selection(),
            )
        }
        (
            M1FullStepWorkspacePlans::PairedPrefill {
                draft: draft_plan,
                target: target_plan,
            },
            M1FullStepKvWorkspaceTablesV1::PairedPrefill { draft, target },
        ) => {
            let expected_target = target_plan.selection();
            validate_exact_selection(
                Qwen3ModelRole::Target8B,
                expected_target,
                target.selection(),
            )?;
            let expected_draft = paired_draft_selection(expected_target).ok_or(
                M1PrepublicationJoinErrorV1::Selection {
                    role: Qwen3ModelRole::Target8B,
                    expected: expected_target,
                    actual: expected_target,
                },
            )?;
            validate_exact_selection(Qwen3ModelRole::Draft06B, expected_draft, draft.selection())?;
            if draft_plan.selection() != expected_draft {
                return Err(M1PrepublicationJoinErrorV1::Selection {
                    role: Qwen3ModelRole::Draft06B,
                    expected: expected_draft,
                    actual: draft_plan.selection(),
                });
            }
            validate_inputs(
                scheduled,
                runner,
                Qwen3ModelRole::Draft06B,
                draft.inputs(),
                draft_plan.selection(),
            )?;
            let target_plans = validate_inputs(
                scheduled,
                runner,
                Qwen3ModelRole::Target8B,
                target.inputs(),
                target_plan.selection(),
            )?;
            validate_paired_inputs(draft.inputs(), target.inputs())?;
            Ok(target_plans)
        }
        (
            M1FullStepWorkspacePlans::SpeculativeRound {
                draft_decode: draft_plan,
                target_speculative: target_plan,
            },
            M1FullStepKvWorkspaceTablesV1::SpeculativeRound {
                draft_decode,
                target_speculative,
            },
        ) => {
            let target_selection = target_plan.selection();
            validate_exact_selection(
                Qwen3ModelRole::Target8B,
                target_selection,
                target_speculative.selection(),
            )?;
            let Some((draft_selection, k)) = speculative_draft_selection(target_selection) else {
                return Err(M1PrepublicationJoinErrorV1::SpeculativeWidth);
            };
            if draft_plan.selection() != draft_selection
                || draft_decode.draft_decode_selection() != draft_selection
                || draft_decode.target_speculative_selection() != target_selection
                || draft_decode.draft_tokens() != k
            {
                return Err(M1PrepublicationJoinErrorV1::SpeculativeWidth);
            }
            validate_inputs(
                scheduled,
                runner,
                Qwen3ModelRole::Draft06B,
                draft_decode.inputs(),
                draft_plan.selection(),
            )?;
            let target_plans = validate_inputs(
                scheduled,
                runner,
                Qwen3ModelRole::Target8B,
                target_speculative.inputs(),
                target_plan.selection(),
            )?;
            validate_speculative_anchor_inputs(draft_decode.inputs(), target_speculative.inputs())?;
            Ok(target_plans)
        }
        _ => unreachable!("shape equality checked above"),
    }
}

fn validate_exact_selection(
    role: Qwen3ModelRole,
    expected: Qwen3PlanSelection,
    actual: Qwen3PlanSelection,
) -> Result<(), M1PrepublicationJoinErrorV1> {
    if expected == actual && actual.role == role {
        Ok(())
    } else {
        Err(M1PrepublicationJoinErrorV1::Selection {
            role,
            expected,
            actual,
        })
    }
}

fn validate_inputs(
    scheduled: &M1ScheduledDispatchV1,
    runner: &LogicalRunnerDeclaration,
    role: Qwen3ModelRole,
    inputs: &ValidatedM1StepInputs,
    expected_selection: Qwen3PlanSelection,
) -> Result<[Option<StepPlan>; MAX_LANES], M1PrepublicationJoinErrorV1> {
    let count = scheduled.member_count();
    let actual = inputs.live_lane_count() as usize;
    if actual != count {
        return Err(M1PrepublicationJoinErrorV1::MemberCount {
            expected: count,
            actual,
        });
    }
    if inputs.selection() != expected_selection {
        return Err(M1PrepublicationJoinErrorV1::Selection {
            role,
            expected: expected_selection,
            actual: inputs.selection(),
        });
    }
    let published_plan = runner
        .plan(inputs.selection())
        .map_err(|_| M1PrepublicationJoinErrorV1::PlanIdentity { role, lane: 0 })?;
    let mut result = [None; MAX_LANES];
    for (lane, entry) in inputs.lanes().iter().take(count).enumerate() {
        let plan = entry.ok_or(M1PrepublicationJoinErrorV1::MissingLane { role, lane })?;
        let expected_request = scheduled
            .member(lane)
            .ok_or(M1PrepublicationJoinErrorV1::MissingLane { role, lane })?;
        if plan.request() != expected_request {
            return Err(M1PrepublicationJoinErrorV1::RequestOrder {
                role,
                lane,
                expected: expected_request,
                actual: plan.request(),
            });
        }
        if plan.completion_epoch() != scheduled.epoch() {
            return Err(M1PrepublicationJoinErrorV1::CompletionEpoch { role, lane });
        }
        if plan.selection() != inputs.selection() {
            return Err(M1PrepublicationJoinErrorV1::LaneSelection { role, lane });
        }
        if plan.plan_id() != &published_plan.plan_id {
            return Err(M1PrepublicationJoinErrorV1::PlanIdentity { role, lane });
        }
        result[lane] = Some(plan);
    }
    Ok(result)
}

fn validate_contexts(
    draft: &ValidatedM1StepInputs,
    target: &ValidatedM1StepInputs,
) -> Result<(), M1PrepublicationJoinErrorV1> {
    for lane in 0..draft.live_lane_count() as usize {
        if draft.context_lengths()[lane] != target.context_lengths()[lane] {
            return Err(M1PrepublicationJoinErrorV1::ContextLength {
                lane,
                draft: draft.context_lengths()[lane],
                target: target.context_lengths()[lane],
            });
        }
    }
    Ok(())
}

fn validate_paired_inputs(
    draft: &ValidatedM1StepInputs,
    target: &ValidatedM1StepInputs,
) -> Result<(), M1PrepublicationJoinErrorV1> {
    validate_contexts(draft, target)?;
    let draft_width = draft.dimensions().active_tokens as usize;
    let target_width = target.dimensions().active_tokens as usize;
    for lane in 0..draft.live_lane_count() as usize {
        let active = draft.active_lengths()[lane];
        if active != target.active_lengths()[lane] {
            return Err(M1PrepublicationJoinErrorV1::ActiveLength {
                lane,
                draft: active,
                target: target.active_lengths()[lane],
            });
        }
        for column in 0..active as usize {
            let draft_index = lane * draft_width + column;
            let target_index = lane * target_width + column;
            if draft.token_ids()[draft_index] != target.token_ids()[target_index] {
                return Err(M1PrepublicationJoinErrorV1::Token { lane, column });
            }
            if draft.position_ids()[draft_index] != target.position_ids()[target_index] {
                return Err(M1PrepublicationJoinErrorV1::Position { lane, column });
            }
        }
    }
    Ok(())
}

fn validate_speculative_anchor_inputs(
    draft: &ValidatedM1StepInputs,
    target: &ValidatedM1StepInputs,
) -> Result<(), M1PrepublicationJoinErrorV1> {
    validate_contexts(draft, target)?;
    let draft_width = draft.dimensions().active_tokens as usize;
    let target_width = target.dimensions().active_tokens as usize;
    for lane in 0..draft.live_lane_count() as usize {
        let draft_index = lane * draft_width;
        let target_index = lane * target_width;
        if draft.token_ids()[draft_index] != target.token_ids()[target_index] {
            return Err(M1PrepublicationJoinErrorV1::Token { lane, column: 0 });
        }
        if draft.position_ids()[draft_index] != target.position_ids()[target_index] {
            return Err(M1PrepublicationJoinErrorV1::Position { lane, column: 0 });
        }
    }
    Ok(())
}

fn paired_draft_selection(target: Qwen3PlanSelection) -> Option<Qwen3PlanSelection> {
    (target.role == Qwen3ModelRole::Target8B && target.mode == Qwen3ExecutionMode::Prefill)
        .then_some(Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: target.mode,
            bucket: target.bucket,
        })
}

fn speculative_draft_selection(target: Qwen3PlanSelection) -> Option<(Qwen3PlanSelection, u32)> {
    if target.role != Qwen3ModelRole::Target8B || target.mode != Qwen3ExecutionMode::Speculative {
        return None;
    }
    let (bucket, k) = match target.bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192 => (Qwen3PlanBucket::DecodeS1C8192, 4),
        Qwen3PlanBucket::SpeculativeS8K4C8192 => (Qwen3PlanBucket::DecodeS8C8192, 4),
        Qwen3PlanBucket::SpeculativeS1K8C8192 => (Qwen3PlanBucket::DecodeS1C8192, 8),
        Qwen3PlanBucket::SpeculativeS1K16C8192 => (Qwen3PlanBucket::DecodeS1C8192, 16),
        _ => return None,
    };
    Some((
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket,
        },
        k,
    ))
}

#[cfg(test)]
mod tests {
    use ferric_build::{
        generate_qwen3_gfx942_runner_declaration, publish_qwen3_gfx942_runner_declaration,
        qwen3_runner_closure_test_fixture,
    };
    use ferric_spec::completion::CompletionEpoch;
    use ferric_spec::{
        validate_m1_step_inputs, Identity, M1StepInputCandidate, M1StepInputValidationOutcome,
    };

    use super::*;
    use crate::Engine;

    fn identity(tag: u8) -> Identity {
        Identity::new([tag; 32])
    }

    fn runner() -> LogicalRunnerDeclaration {
        let declaration =
            generate_qwen3_gfx942_runner_declaration(qwen3_runner_closure_test_fixture()).unwrap();
        LogicalRunnerDeclaration::from_published(
            publish_qwen3_gfx942_runner_declaration(declaration).unwrap(),
        )
    }

    fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn validated_inputs(
        selected: Qwen3PlanSelection,
        plans: &[StepPlan],
        contexts: &[u32],
    ) -> ValidatedM1StepInputs {
        let dimensions = selected
            .bucket
            .dimensions(selected.role, selected.mode)
            .unwrap();
        let sequences = dimensions.sequences as usize;
        let width = dimensions.active_tokens as usize;
        let mut lanes = vec![None; sequences];
        let mut token_ids = vec![0; sequences * width];
        let mut position_ids = vec![0; sequences * width];
        let mut active_lengths = vec![0; sequences];
        let mut context_lengths = vec![0; sequences];
        for (lane, plan) in plans.iter().copied().enumerate() {
            lanes[lane] = Some(plan);
            active_lengths[lane] = 1;
            context_lengths[lane] = contexts[lane];
            token_ids[lane * width] = 1;
            position_ids[lane * width] = contexts[lane];
        }
        match validate_m1_step_inputs(M1StepInputCandidate::new(
            selected,
            lanes,
            token_ids,
            position_ids,
            active_lengths,
            context_lengths,
        )) {
            M1StepInputValidationOutcome::Validated(inputs) => inputs,
            M1StepInputValidationOutcome::Rejected(failure) => {
                panic!("fixture rejected: {:?}", failure.error())
            }
        }
    }

    fn scheduled_two() -> (M1ScheduledDispatchV1, [RequestId; 2]) {
        let mut engine = Engine::<8>::new(32, 8, 64).unwrap();
        let first = engine.admit().unwrap();
        let second = engine.admit().unwrap();
        engine.append_tentative(first, 1).unwrap();
        engine.append_tentative(second, 1).unwrap();
        (
            engine.dispatch_m1_ready().unwrap().unwrap(),
            [first, second],
        )
    }

    #[test]
    fn target_prefix_accepts_exact_count_order_generation_epoch_and_plan() {
        let runner = runner();
        let selected = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let plan_id = runner.plan(selected).unwrap().plan_id;
        let (scheduled, requests) = scheduled_two();
        let plans =
            requests.map(|request| StepPlan::new(request, scheduled.epoch(), plan_id, selected));
        let inputs = validated_inputs(selected, &plans, &[3, 5]);
        let bound = validate_inputs(
            &scheduled,
            &runner,
            Qwen3ModelRole::Target8B,
            &inputs,
            selected,
        )
        .unwrap();
        assert_eq!(bound[0], Some(plans[0]));
        assert_eq!(bound[1], Some(plans[1]));
        assert!(bound[2..].iter().all(Option::is_none));
    }

    #[test]
    fn hostile_count_order_generation_epoch_selection_and_plan_id_fail_closed() {
        let runner = runner();
        let selected = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let other = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS32C8192,
        );
        let plan_id = runner.plan(selected).unwrap().plan_id;
        let (scheduled, requests) = scheduled_two();

        let one = [StepPlan::new(
            requests[0],
            scheduled.epoch(),
            plan_id,
            selected,
        )];
        assert!(matches!(
            validate_inputs(
                &scheduled,
                &runner,
                Qwen3ModelRole::Target8B,
                &validated_inputs(selected, &one, &[0]),
                selected
            ),
            Err(M1PrepublicationJoinErrorV1::MemberCount {
                expected: 2,
                actual: 1
            })
        ));

        let reversed = [
            StepPlan::new(requests[1], scheduled.epoch(), plan_id, selected),
            StepPlan::new(requests[0], scheduled.epoch(), plan_id, selected),
        ];
        assert!(matches!(
            validate_inputs(
                &scheduled,
                &runner,
                Qwen3ModelRole::Target8B,
                &validated_inputs(selected, &reversed, &[0, 0]),
                selected
            ),
            Err(M1PrepublicationJoinErrorV1::RequestOrder { lane: 0, .. })
        ));

        let stale = [
            StepPlan::new(
                RequestId::new(requests[0].slot(), requests[0].generation() + 1),
                scheduled.epoch(),
                plan_id,
                selected,
            ),
            StepPlan::new(requests[1], scheduled.epoch(), plan_id, selected),
        ];
        assert!(matches!(
            validate_inputs(
                &scheduled,
                &runner,
                Qwen3ModelRole::Target8B,
                &validated_inputs(selected, &stale, &[0, 0]),
                selected
            ),
            Err(M1PrepublicationJoinErrorV1::RequestOrder { lane: 0, .. })
        ));

        let wrong_epoch = requests.map(|request| {
            StepPlan::new(
                request,
                CompletionEpoch::new(scheduled.epoch().value() + 1),
                plan_id,
                selected,
            )
        });
        assert!(matches!(
            validate_inputs(
                &scheduled,
                &runner,
                Qwen3ModelRole::Target8B,
                &validated_inputs(selected, &wrong_epoch, &[0, 0]),
                selected
            ),
            Err(M1PrepublicationJoinErrorV1::CompletionEpoch { lane: 0, .. })
        ));

        let exact =
            requests.map(|request| StepPlan::new(request, scheduled.epoch(), plan_id, selected));
        assert!(matches!(
            validate_inputs(
                &scheduled,
                &runner,
                Qwen3ModelRole::Target8B,
                &validated_inputs(selected, &exact, &[0, 0]),
                other
            ),
            Err(M1PrepublicationJoinErrorV1::Selection { .. })
        ));

        let wrong_plan = requests
            .map(|request| StepPlan::new(request, scheduled.epoch(), identity(99), selected));
        assert!(matches!(
            validate_inputs(
                &scheduled,
                &runner,
                Qwen3ModelRole::Target8B,
                &validated_inputs(selected, &wrong_plan, &[0, 0]),
                selected
            ),
            Err(M1PrepublicationJoinErrorV1::PlanIdentity { lane: 0, .. })
        ));
    }

    #[test]
    fn hostile_context_k_capacity_and_arena_identity_fail_closed() {
        let speculative_target = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        );
        let (draft, k) = speculative_draft_selection(speculative_target).unwrap();
        assert_eq!(draft.bucket, Qwen3PlanBucket::DecodeS1C8192);
        assert_eq!(k, 16);
        assert!(speculative_draft_selection(selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        ))
        .is_none());

        let request = RequestId::new(0, 1);
        let epoch = CompletionEpoch::new(1);
        let target = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let target_plan = StepPlan::new(request, epoch, identity(1), target);
        let draft_plan = StepPlan::new(request, epoch, identity(2), draft);
        let target_inputs = validated_inputs(target, &[target_plan], &[8]);
        let draft_inputs = validated_inputs(draft, &[draft_plan], &[7]);
        assert_eq!(
            validate_contexts(&draft_inputs, &target_inputs),
            Err(M1PrepublicationJoinErrorV1::ContextLength {
                lane: 0,
                draft: 7,
                target: 8
            })
        );

        assert_eq!(
            validate_completion_capacity(9, 8),
            Err(M1PrepublicationBatchBuildErrorKindV1::CompletionCapacity {
                members: 9,
                capacity: 8
            })
        );
        assert_eq!(validate_completion_capacity(8, 8), Ok(()));
        assert_eq!(
            validate_kv_arena_identity(Qwen3ModelRole::Target8B, identity(4), identity(5)),
            Err(M1PrepublicationBatchBuildErrorKindV1::TargetKvArena {
                expected: identity(4),
                actual: identity(5)
            })
        );
        assert_eq!(
            validate_kv_arena_identity(Qwen3ModelRole::Draft06B, identity(6), identity(7)),
            Err(M1PrepublicationBatchBuildErrorKindV1::DraftKvArena {
                expected: identity(6),
                actual: identity(7)
            })
        );
    }
}
