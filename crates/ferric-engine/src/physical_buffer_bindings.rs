//! Owner-checked service-buffer bindings for exact M1 physical recipes.
//!
//! This layer resolves Ferric semantic sources through already-bound workspace
//! and model-memory owners plus one coherent completion-output owner. It
//! retains every linear input and produces only ordered
//! [`ServiceFixedDispatchBufferV1`] rosters. It does not construct
//! packets or batches, transfer allocation custody, publish a queue, launch
//! work, authenticate contents, or prove completion or refinement.

use core::fmt;

use fe2o3_service_host::{
    ServiceAllocationErrorV1, ServiceDeviceDispatchRangeV1, ServiceFixedDispatchBufferV1,
    ServiceHostDispatchRangeV1, ServiceHostDispatchSnapshotRangeV1,
};
use ferric_spec::Identity;

use crate::{
    m1_completion_output_shape_v1, AddresslessM1PhysicalBufferRecipeV1,
    AddresslessM1PhysicalKernargRecipeV1, BoundM1CompletionOutputV1,
    BoundM1FullStepWorkspaceSubleases, M1CompletionOutputErrorV1, M1CompletionOutputShapeV1,
    M1DirectDiagnosticChoicesErrorV1, M1FullStepWorkspaceDispatchRangeError,
    M1FullStepWorkspaceSubleaseBindingError, M1FullStepWorkspaceSubleaseOwners,
    M1PartitionedModelMemoryKvPoolV1, M1PhysicalBufferRecipeErrorV1, M1PhysicalBufferRecipeRowV1,
    M1PhysicalBufferSentinelV1, M1PhysicalBufferSourceV1, M1PhysicalProgramV1,
    M1QualificationLogitsErrorV1, M1SpeculativeDiagnosticChoicesErrorV1, M1StepDispatchIntent,
    ModelMemoryDispatchRangeErrorV1,
};

/// Owner-checked physical-buffer binding format.
pub const M1_PHYSICAL_BUFFER_BINDING_VERSION_V1: u32 = 1;

/// One physical row's exact ordered generic service-buffer roster.
#[derive(Debug, Eq, PartialEq)]
pub struct M1BoundPhysicalBufferRowV1 {
    dispatch_index: u32,
    profile_id: Identity,
    program: M1PhysicalProgramV1,
    buffers: Box<[ServiceFixedDispatchBufferV1]>,
}

impl M1BoundPhysicalBufferRowV1 {
    pub(crate) fn from_queue_rearm(
        source: &M1PhysicalBufferRecipeRowV1,
        buffers: Box<[ServiceFixedDispatchBufferV1]>,
    ) -> Self {
        Self {
            dispatch_index: source.dispatch_index(),
            profile_id: source.profile_id(),
            program: source.program(),
            buffers,
        }
    }

    /// Zero-based position in the complete physical step.
    #[must_use]
    pub const fn dispatch_index(&self) -> u32 {
        self.dispatch_index
    }

    /// Exact canonical profile identity retained by the source row.
    #[must_use]
    pub const fn profile_id(&self) -> Identity {
        self.profile_id
    }

    /// Exact physical program retained by the source row.
    #[must_use]
    pub const fn program(&self) -> M1PhysicalProgramV1 {
        self.program
    }

    /// Complete generic buffer roster in inspected explicit-argument order.
    #[must_use]
    pub fn buffers(&self) -> &[ServiceFixedDispatchBufferV1] {
        &self.buffers
    }

    /// This inert roster grants no packet or queue authority.
    #[must_use]
    pub const fn grants_execution_authority(&self) -> bool {
        false
    }
}

/// Move-only custody of every exact input and owner-checked buffer roster.
///
/// ```compile_fail
/// use ferric_engine::BoundM1PhysicalBufferBindingsV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<BoundM1PhysicalBufferBindingsV1>();
/// ```
#[must_use = "all physical, allocation, and buffer custody must remain retained"]
#[derive(Debug)]
pub struct BoundM1PhysicalBufferBindingsV1 {
    version: u32,
    kernargs: AddresslessM1PhysicalKernargRecipeV1,
    source_rows: Box<[M1PhysicalBufferRecipeRowV1]>,
    workspaces: BoundM1FullStepWorkspaceSubleases,
    partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    completion_output: BoundM1CompletionOutputV1,
    rows: Box<[M1BoundPhysicalBufferRowV1]>,
}

impl BoundM1PhysicalBufferBindingsV1 {
    pub(crate) fn from_parts(
        recipe: AddresslessM1PhysicalBufferRecipeV1,
        workspace_owners: M1FullStepWorkspaceSubleaseOwners,
        partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
        completion_output: BoundM1CompletionOutputV1,
        rows: Box<[M1BoundPhysicalBufferRowV1]>,
    ) -> Self {
        let (kernargs, composition, source_rows) = recipe.into_parts();
        Self {
            version: M1_PHYSICAL_BUFFER_BINDING_VERSION_V1,
            kernargs,
            source_rows,
            workspaces: BoundM1FullStepWorkspaceSubleases::from_parts(
                composition,
                workspace_owners,
            ),
            partitioned_memory,
            completion_output,
            rows,
        }
    }

    /// Binding format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Exact retained zero-pointer kernarg recipe.
    #[must_use]
    pub const fn kernarg_recipe(&self) -> &AddresslessM1PhysicalKernargRecipeV1 {
        &self.kernargs
    }

    /// Exact retained Ferric access and semantic-source rows.
    #[must_use]
    pub fn source_rows(&self) -> &[M1PhysicalBufferRecipeRowV1] {
        &self.source_rows
    }

    /// Exact retained full-step workspace custody.
    #[must_use = "the exact workspace custody remains retained by the physical binding"]
    pub const fn workspace_bindings(&self) -> &BoundM1FullStepWorkspaceSubleases {
        &self.workspaces
    }

    /// Exact retained partitioned model-memory and allocation custody.
    #[must_use = "partitioned model-memory custody remains retained by the binding"]
    pub const fn partitioned_memory(&self) -> &M1PartitionedModelMemoryKvPoolV1 {
        &self.partitioned_memory
    }

    /// Exact retained coherent host-download completion-output custody.
    #[must_use = "the exact completion-output custody remains retained by the physical binding"]
    pub const fn completion_output(&self) -> &BoundM1CompletionOutputV1 {
        &self.completion_output
    }

    /// Complete owner-checked service-buffer rosters in global dispatch order.
    #[must_use]
    pub fn rows(&self) -> &[M1BoundPhysicalBufferRowV1] {
        &self.rows
    }

    /// Recovers every exact input owner plus the derived generic rosters.
    #[must_use = "all original linear inputs and derived buffer rosters remain retained"]
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        AddresslessM1PhysicalBufferRecipeV1,
        M1FullStepWorkspaceSubleaseOwners,
        M1PartitionedModelMemoryKvPoolV1,
        BoundM1CompletionOutputV1,
        Box<[M1BoundPhysicalBufferRowV1]>,
    ) {
        let (composition, owners) = self.workspaces.into_parts();
        let recipe = AddresslessM1PhysicalBufferRecipeV1::from_parts(
            self.kernargs,
            composition,
            self.source_rows,
        );
        (
            recipe,
            owners,
            self.partitioned_memory,
            self.completion_output,
            self.rows,
        )
    }

    /// These rosters grant no packet, batch, queue, or launch authority.
    #[must_use]
    pub const fn grants_execution_authority(&self) -> bool {
        false
    }

    /// Owner-checked ranges alone authenticate no buffer contents.
    #[must_use]
    pub const fn authenticates_contents(&self) -> bool {
        false
    }

    /// This structural join proves no hardware result or operator refinement.
    #[must_use]
    pub const fn proves_hardware_or_refinement(&self) -> bool {
        false
    }
}

/// Fail-closed physical-buffer binding diagnostic.
#[derive(Debug)]
pub enum M1PhysicalBufferBindingErrorV1 {
    /// The retained addressless recipe no longer revalidates exactly.
    Recipe(M1PhysicalBufferRecipeErrorV1),
    /// A source requires an unimplemented in-batch materialization step.
    MaterializationRequired {
        /// Global physical row.
        dispatch_index: u32,
        /// Inspected explicit-argument ordinal.
        argument: usize,
        /// Exact unresolved semantic prerequisite.
        source: M1PhysicalBufferSourceV1,
    },
    /// The addressless composition did not join the exact workspace owners.
    WorkspaceOwner(M1FullStepWorkspaceSubleaseBindingError),
    /// A physical source row no longer matches its exact physical row/image.
    RowMetadata { dispatch_index: u32 },
    /// A source row's buffer roster has an unexpected cardinality.
    BufferCount {
        /// Global physical row.
        dispatch_index: u32,
        /// Exact retained semantic-source count.
        expected: usize,
        /// Derived generic buffer count.
        actual: usize,
    },
    /// An inspected explicit-argument ordinal was reordered or substituted.
    ArgumentOrdinal {
        /// Global physical row.
        dispatch_index: u32,
        /// Exact required ordinal.
        expected: usize,
        /// Rejected ordinal.
        actual: usize,
    },
    /// An exact workspace source could not be resolved through its owner.
    WorkspaceRange {
        /// Global physical row.
        dispatch_index: u32,
        /// Inspected explicit-argument ordinal.
        argument: usize,
        /// Exact owner diagnostic.
        error: M1FullStepWorkspaceDispatchRangeError,
    },
    /// An exact model-weight or KV source could not be resolved.
    ModelMemoryRange {
        /// Global physical row.
        dispatch_index: u32,
        /// Inspected explicit-argument ordinal.
        argument: usize,
        /// Exact model-memory diagnostic.
        error: ModelMemoryDispatchRangeErrorV1,
    },
    /// A partitioned KV-plane source could not be resolved exactly.
    PartitionedKvRange {
        /// Global physical row.
        dispatch_index: u32,
        /// Inspected explicit-argument ordinal.
        argument: usize,
        /// Exact partition/model/allocation diagnostic.
        error: crate::M1DeviceKvArenaLeaseErrorV1,
    },
    /// A fixed nonempty sentinel subrange could not be narrowed exactly.
    SentinelRange {
        /// Global physical row.
        dispatch_index: u32,
        /// Inspected explicit-argument ordinal.
        argument: usize,
        /// Exact generic range diagnostic.
        error: ServiceAllocationErrorV1,
    },
    /// The recipe did not contain exactly one target compact-output source.
    CompletionOutputCount {
        /// Exact required source count.
        expected: usize,
        /// Rejected source count.
        actual: usize,
    },
    /// A compact source and retained host output name different exact shapes.
    CompletionOutputShape {
        /// Global physical row.
        dispatch_index: u32,
        /// Inspected explicit-argument ordinal.
        argument: usize,
        /// Target selection named by the compact row.
        expected_selection: ferric_spec::Qwen3PlanSelection,
        /// Selection retained by the host-output owner.
        actual_selection: ferric_spec::Qwen3PlanSelection,
        /// Sequence count required by the compact row selection.
        expected_sequences: u32,
        /// Sequence count encoded by the semantic source.
        source_sequences: u32,
        /// Sequence count retained by the host-output owner.
        actual_sequences: u32,
        /// Exact canonical byte extent required by the compact row.
        expected_extent: u64,
        /// Byte extent retained by the host-output owner.
        actual_extent: u64,
    },
    /// The generic allocation owner rejected the coherent completion range.
    CompletionOutputRange {
        /// Global physical row.
        dispatch_index: u32,
        /// Inspected explicit-argument ordinal.
        argument: usize,
        /// Exact host-output owner diagnostic.
        error: M1CompletionOutputErrorV1,
    },
    /// The generic fixed-buffer constructor rejected the uniquely associated
    /// completion interior and enclosing initialized snapshot.
    CompletionSnapshotAssociation {
        /// Global physical row.
        dispatch_index: u32,
        /// Inspected explicit-argument ordinal.
        argument: usize,
        /// Exact generic range/association diagnostic.
        error: ServiceAllocationErrorV1,
    },
    /// Qualification capture was attached to a non-target-only physical recipe.
    QualificationLogitsIntent,
    /// The physical recipe did not retain exactly the two target logits bindings.
    QualificationLogitsSources { expected: usize, actual: usize },
    /// The qualification logits owner rejected shape or allocation revalidation.
    QualificationLogitsRange { error: M1QualificationLogitsErrorV1 },
    /// Direct target choices were attached to a non-direct physical recipe.
    DirectDiagnosticChoicesIntent,
    /// Direct target choice capture was combined with another semantic capture.
    DirectDiagnosticChoicesConflict,
    /// The direct choice owner rejected shape or allocation revalidation.
    DirectDiagnosticChoicesRange {
        error: M1DirectDiagnosticChoicesErrorV1,
    },
    /// The physical recipe did not retain exactly two target choice bindings.
    DirectDiagnosticChoiceSources { expected: usize, actual: usize },
    /// Diagnostic choices were attached to a non-finite-speculative recipe.
    SpeculativeDiagnosticChoicesIntent,
    /// The diagnostic choice owner rejected shape or allocation revalidation.
    SpeculativeDiagnosticChoicesRange {
        error: M1SpeculativeDiagnosticChoicesErrorV1,
    },
    /// An exact diagnostic draft-choice row could not be narrowed.
    SpeculativeDiagnosticDraftSubrange {
        dispatch_index: u32,
        argument: usize,
        error: ServiceAllocationErrorV1,
    },
    /// The exact finite-speculative draft/target choice source roster drifted.
    SpeculativeDiagnosticChoiceSources {
        draft_rows: usize,
        draft_whole: usize,
        target_rows: usize,
    },
}

impl fmt::Display for M1PhysicalBufferBindingErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 physical buffer binding rejected: {self:?}")
    }
}

impl std::error::Error for M1PhysicalBufferBindingErrorV1 {}

/// Retry-safe rejection retaining every exact unchanged linear input.
///
/// ```compile_fail
/// use ferric_engine::M1PhysicalBufferBindingFailureV1;
///
/// fn recover_twice(failure: M1PhysicalBufferBindingFailureV1) {
///     let _first = failure.into_parts();
///     let _second = failure.into_parts();
/// }
/// ```
#[must_use = "all rejected physical and allocation owners remain recoverable"]
#[derive(Debug)]
pub struct M1PhysicalBufferBindingFailureV1 {
    error: Box<M1PhysicalBufferBindingErrorV1>,
    recipe: Box<AddresslessM1PhysicalBufferRecipeV1>,
    workspace_owners: Box<M1FullStepWorkspaceSubleaseOwners>,
    partitioned_memory: Box<M1PartitionedModelMemoryKvPoolV1>,
    completion_output: Box<BoundM1CompletionOutputV1>,
}

impl M1PhysicalBufferBindingFailureV1 {
    /// Exact fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> &M1PhysicalBufferBindingErrorV1 {
        &self.error
    }

    /// Recovers the diagnostic and every exact unchanged input owner.
    #[must_use = "all original linear inputs remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalBufferBindingErrorV1,
        AddresslessM1PhysicalBufferRecipeV1,
        M1FullStepWorkspaceSubleaseOwners,
        M1PartitionedModelMemoryKvPoolV1,
        BoundM1CompletionOutputV1,
    ) {
        (
            *self.error,
            *self.recipe,
            *self.workspace_owners,
            *self.partitioned_memory,
            *self.completion_output,
        )
    }
}

/// Resolves every exact semantic source into generic owner-checked buffer rosters.
///
/// Materialization prerequisites fail before any input is decomposed. All
/// subsequent failures reconstruct and return the exact addressless recipe,
/// workspace owners, model-memory owner, and completion-output owner unchanged.
///
/// # Errors
///
/// Returns [`M1PhysicalBufferBindingFailureV1`] for recipe, materialization,
/// workspace-owner, row/order/ordinal, workspace-range, model-range,
/// completion-output, or sentinel-subrange rejection.
pub fn bind_m1_physical_buffer_ranges_v1(
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    workspace_owners: M1FullStepWorkspaceSubleaseOwners,
    partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    completion_output: BoundM1CompletionOutputV1,
) -> Result<BoundM1PhysicalBufferBindingsV1, M1PhysicalBufferBindingFailureV1> {
    if let Err(error) = recipe.revalidate() {
        return Err(failure(
            M1PhysicalBufferBindingErrorV1::Recipe(error),
            recipe,
            workspace_owners,
            partitioned_memory,
            completion_output,
        ));
    }
    if let Some(error) = first_materialization_requirement(&recipe) {
        return Err(failure(
            error,
            recipe,
            workspace_owners,
            partitioned_memory,
            completion_output,
        ));
    }

    let completion_binding =
        match preflight_completion_output(&recipe, &completion_output, &partitioned_memory) {
            Ok(range) => range,
            Err(error) => {
                return Err(failure(
                    error,
                    recipe,
                    workspace_owners,
                    partitioned_memory,
                    completion_output,
                ));
            }
        };
    let qualification_logits_range =
        match preflight_qualification_logits(&recipe, &completion_output, &partitioned_memory) {
            Ok(range) => range,
            Err(error) => {
                return Err(failure(
                    error,
                    recipe,
                    workspace_owners,
                    partitioned_memory,
                    completion_output,
                ));
            }
        };
    let direct_diagnostic_choices_range =
        match preflight_direct_diagnostic_choices(&recipe, &completion_output, &partitioned_memory)
        {
            Ok(range) => range,
            Err(error) => {
                return Err(failure(
                    error,
                    recipe,
                    workspace_owners,
                    partitioned_memory,
                    completion_output,
                ));
            }
        };
    let speculative_diagnostic_choices_ranges = match preflight_speculative_diagnostic_choices(
        &recipe,
        &completion_output,
        &partitioned_memory,
    ) {
        Ok(ranges) => ranges,
        Err(error) => {
            return Err(failure(
                error,
                recipe,
                workspace_owners,
                partitioned_memory,
                completion_output,
            ));
        }
    };

    let (kernargs, composition, source_rows) = recipe.into_parts();
    let workspaces = match partitioned_memory
        .bind_full_step_workspaces(composition, workspace_owners)
    {
        Ok(workspaces) => workspaces,
        Err(rejection) => {
            let (error, composition, workspace_owners) = rejection.into_parts();
            let recipe =
                AddresslessM1PhysicalBufferRecipeV1::from_parts(kernargs, composition, source_rows);
            return Err(failure(
                M1PhysicalBufferBindingErrorV1::WorkspaceOwner(error),
                recipe,
                workspace_owners,
                partitioned_memory,
                completion_output,
            ));
        }
    };

    let resolution = SourceResolutionContextV1 {
        partitioned_memory: &partitioned_memory,
        workspaces: &workspaces,
        completion_shape: completion_output.shape(),
        completion_binding,
        direct_diagnostic_choices_range,
        qualification_logits_range,
        speculative_diagnostic_choices_ranges,
    };
    match resolve_rows(&kernargs, &source_rows, &resolution) {
        Ok(rows) => Ok(BoundM1PhysicalBufferBindingsV1 {
            version: M1_PHYSICAL_BUFFER_BINDING_VERSION_V1,
            kernargs,
            source_rows,
            workspaces,
            partitioned_memory,
            completion_output,
            rows,
        }),
        Err(error) => {
            let (composition, workspace_owners) = workspaces.into_parts();
            let recipe =
                AddresslessM1PhysicalBufferRecipeV1::from_parts(kernargs, composition, source_rows);
            Err(failure(
                error,
                recipe,
                workspace_owners,
                partitioned_memory,
                completion_output,
            ))
        }
    }
}

fn failure(
    error: M1PhysicalBufferBindingErrorV1,
    recipe: AddresslessM1PhysicalBufferRecipeV1,
    workspace_owners: M1FullStepWorkspaceSubleaseOwners,
    partitioned_memory: M1PartitionedModelMemoryKvPoolV1,
    completion_output: BoundM1CompletionOutputV1,
) -> M1PhysicalBufferBindingFailureV1 {
    M1PhysicalBufferBindingFailureV1 {
        error: Box::new(error),
        recipe: Box::new(recipe),
        workspace_owners: Box::new(workspace_owners),
        partitioned_memory: Box::new(partitioned_memory),
        completion_output: Box::new(completion_output),
    }
}

fn first_materialization_requirement(
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
) -> Option<M1PhysicalBufferBindingErrorV1> {
    recipe.rows().iter().find_map(|row| {
        row.buffers().iter().find_map(|buffer| {
            let source = buffer.source();
            source.requires_future_materialization().then_some(
                M1PhysicalBufferBindingErrorV1::MaterializationRequired {
                    dispatch_index: row.dispatch_index(),
                    argument: buffer.explicit_argument_index(),
                    source,
                },
            )
        })
    })
}

fn preflight_completion_output(
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    completion_output: &BoundM1CompletionOutputV1,
    partitioned_memory: &M1PartitionedModelMemoryKvPoolV1,
) -> Result<CompletionOutputBindingRangeV1, M1PhysicalBufferBindingErrorV1> {
    let mut matches = recipe.rows().iter().flat_map(|row| {
        row.buffers().iter().filter_map(move |buffer| {
            let M1PhysicalBufferSourceV1::CompletionOutput { sequences } = buffer.source() else {
                return None;
            };
            Some((
                row.dispatch_index(),
                buffer.explicit_argument_index(),
                row.selection(),
                sequences,
            ))
        })
    });
    let Some((dispatch_index, argument, selection, source_sequences)) = matches.next() else {
        return Err(M1PhysicalBufferBindingErrorV1::CompletionOutputCount {
            expected: 1,
            actual: 0,
        });
    };
    if matches.next().is_some() {
        let actual = 2 + matches.count();
        return Err(M1PhysicalBufferBindingErrorV1::CompletionOutputCount {
            expected: 1,
            actual,
        });
    }
    validate_completion_output_shape(
        dispatch_index,
        argument,
        selection,
        source_sequences,
        completion_output.shape(),
    )?;
    let interior = partitioned_memory
        .completion_output_dispatch_range(completion_output, selection)
        .map_err(
            |error| M1PhysicalBufferBindingErrorV1::CompletionOutputRange {
                dispatch_index,
                argument,
                error,
            },
        )?;
    Ok(CompletionOutputBindingRangeV1 {
        interior,
        snapshot: completion_output
            .completion_canary()
            .map(crate::BoundM1CompletionCanaryV1::snapshot_range),
    })
}

fn preflight_qualification_logits(
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    completion_output: &BoundM1CompletionOutputV1,
    partitioned_memory: &M1PartitionedModelMemoryKvPoolV1,
) -> Result<Option<ServiceHostDispatchRangeV1>, M1PhysicalBufferBindingErrorV1> {
    let Some(logits) = completion_output.qualification_logits() else {
        return Ok(None);
    };
    let M1StepDispatchIntent::TargetOnly(selection) =
        recipe.workspace_composition().dispatch_plan().intent()
    else {
        return Err(M1PhysicalBufferBindingErrorV1::QualificationLogitsIntent);
    };
    if selection != logits.shape().selection() {
        return Err(M1PhysicalBufferBindingErrorV1::QualificationLogitsRange {
            error: M1QualificationLogitsErrorV1::SelectionDrift {
                expected: logits.shape().selection(),
                actual: selection,
            },
        });
    }
    let (sources, exact_sources) = qualification_logits_source_isolation(recipe, selection);
    if sources != 2 || !exact_sources {
        return Err(M1PhysicalBufferBindingErrorV1::QualificationLogitsSources {
            expected: 2,
            actual: sources,
        });
    }
    partitioned_memory
        .qualification_logits_dispatch_range(logits, selection)
        .map(Some)
        .map_err(|error| M1PhysicalBufferBindingErrorV1::QualificationLogitsRange { error })
}

fn preflight_direct_diagnostic_choices(
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    completion_output: &BoundM1CompletionOutputV1,
    partitioned_memory: &M1PartitionedModelMemoryKvPoolV1,
) -> Result<Option<ServiceHostDispatchRangeV1>, M1PhysicalBufferBindingErrorV1> {
    let Some(choices) = completion_output.direct_diagnostic_choices() else {
        return Ok(None);
    };
    if completion_output.qualification_logits().is_some()
        || completion_output.speculative_diagnostic_choices().is_some()
    {
        return Err(M1PhysicalBufferBindingErrorV1::DirectDiagnosticChoicesConflict);
    }
    let intent = recipe.workspace_composition().dispatch_plan().intent();
    let (selection, target_segment) = match intent {
        M1StepDispatchIntent::TargetOnly(selection) => (selection, 0),
        M1StepDispatchIntent::PairedPrefill(selection) => (selection, 1),
        M1StepDispatchIntent::SpeculativeRound(_) => {
            return Err(M1PhysicalBufferBindingErrorV1::DirectDiagnosticChoicesIntent)
        }
    };
    if selection != choices.shape().selection() {
        return Err(M1PhysicalBufferBindingErrorV1::DirectDiagnosticChoicesIntent);
    }
    let (sources, exact) =
        direct_diagnostic_choice_source_isolation(recipe, selection, target_segment);
    if !exact || sources != 2 {
        return Err(
            M1PhysicalBufferBindingErrorV1::DirectDiagnosticChoiceSources {
                expected: 2,
                actual: sources,
            },
        );
    }
    partitioned_memory
        .direct_diagnostic_choices_dispatch_range(choices, selection)
        .map(Some)
        .map_err(|error| M1PhysicalBufferBindingErrorV1::DirectDiagnosticChoicesRange { error })
}

fn direct_diagnostic_choice_source_isolation(
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    selection: ferric_spec::Qwen3PlanSelection,
    target_segment: u8,
) -> (usize, bool) {
    let mut target_sources = 0;
    let exact = recipe.rows().iter().all(|row| {
        row.buffers().iter().all(|buffer| match buffer.source() {
            M1PhysicalBufferSourceV1::Workspace {
                workspace: crate::M1FullStepWorkspaceRole::Target,
                range: ferric_build::M1StepWorkspaceRangeRole::Choices,
            } => {
                target_sources += 1;
                row.segment_index() == target_segment && row.selection() == selection
            }
            M1PhysicalBufferSourceV1::Workspace {
                workspace: crate::M1FullStepWorkspaceRole::Draft,
                range: ferric_build::M1StepWorkspaceRangeRole::Choices,
            } => target_segment == 1 && row.segment_index() == 0,
            _ => true,
        })
    });
    (target_sources, exact)
}

#[derive(Clone, Copy)]
struct SpeculativeDiagnosticChoiceRangesV1 {
    draft: ServiceHostDispatchRangeV1,
    target: ServiceHostDispatchRangeV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SpeculativeDiagnosticChoiceSourceRouteV1 {
    OrdinaryDevice,
    DirectTargetWholeHost,
    DraftWholeHost,
    TargetWholeHost,
    DraftScalarHost {
        iteration: u8,
        relative_offset: u64,
        extent: u64,
        alignment: u64,
    },
}

fn speculative_diagnostic_draft_choice_geometry(
    iteration: u8,
    sequence_count: u32,
) -> Option<(u64, u64, u64)> {
    let maximum = u8::try_from(crate::M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1).ok()?;
    if iteration >= maximum || sequence_count == 0 {
        return None;
    }
    let extent = u64::from(sequence_count).checked_mul(4)?;
    Some((u64::from(iteration).checked_mul(extent)?, extent, 4))
}

pub(crate) fn speculative_diagnostic_choice_source_route(
    source: M1PhysicalBufferSourceV1,
    diagnostics_enabled: bool,
    direct_diagnostics_enabled: bool,
) -> Result<SpeculativeDiagnosticChoiceSourceRouteV1, ()> {
    if direct_diagnostics_enabled
        && matches!(
            source,
            M1PhysicalBufferSourceV1::Workspace {
                workspace: crate::M1FullStepWorkspaceRole::Target,
                range: ferric_build::M1StepWorkspaceRangeRole::Choices,
            }
        )
    {
        return Ok(SpeculativeDiagnosticChoiceSourceRouteV1::DirectTargetWholeHost);
    }
    if !diagnostics_enabled {
        return Ok(SpeculativeDiagnosticChoiceSourceRouteV1::OrdinaryDevice);
    }
    match source {
        M1PhysicalBufferSourceV1::Workspace {
            workspace: crate::M1FullStepWorkspaceRole::Target,
            range: ferric_build::M1StepWorkspaceRangeRole::DraftChoices,
        } => Ok(SpeculativeDiagnosticChoiceSourceRouteV1::DraftWholeHost),
        M1PhysicalBufferSourceV1::Workspace {
            workspace: crate::M1FullStepWorkspaceRole::Target,
            range: ferric_build::M1StepWorkspaceRangeRole::Choices,
        } => Ok(SpeculativeDiagnosticChoiceSourceRouteV1::TargetWholeHost),
        M1PhysicalBufferSourceV1::SpeculativeDraftChoices(expected) => {
            let (relative_offset, extent, alignment) =
                speculative_diagnostic_draft_choice_geometry(
                    expected.iteration(),
                    expected.sequence_count(),
                )
                .ok_or(())?;
            Ok(SpeculativeDiagnosticChoiceSourceRouteV1::DraftScalarHost {
                iteration: expected.iteration(),
                relative_offset,
                extent,
                alignment,
            })
        }
        _ => Ok(SpeculativeDiagnosticChoiceSourceRouteV1::OrdinaryDevice),
    }
}

fn preflight_speculative_diagnostic_choices(
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    completion_output: &BoundM1CompletionOutputV1,
    partitioned_memory: &M1PartitionedModelMemoryKvPoolV1,
) -> Result<Option<SpeculativeDiagnosticChoiceRangesV1>, M1PhysicalBufferBindingErrorV1> {
    let Some(choices) = completion_output.speculative_diagnostic_choices() else {
        return Ok(None);
    };
    let M1StepDispatchIntent::SpeculativeRound(selection) =
        recipe.workspace_composition().dispatch_plan().intent()
    else {
        return Err(M1PhysicalBufferBindingErrorV1::SpeculativeDiagnosticChoicesIntent);
    };
    if selection != choices.shape().selection() {
        return Err(M1PhysicalBufferBindingErrorV1::SpeculativeDiagnosticChoicesIntent);
    }
    let shape = choices.shape();
    let (draft_rows, draft_whole, target_rows, exact) =
        speculative_diagnostic_choice_source_isolation(recipe, shape);
    let expected_draft_rows = usize::from(shape.draft_tokens()) * 2 - 1;
    if !exact || (draft_rows, draft_whole, target_rows) != (expected_draft_rows, 2, 2) {
        return Err(
            M1PhysicalBufferBindingErrorV1::SpeculativeDiagnosticChoiceSources {
                draft_rows,
                draft_whole,
                target_rows,
            },
        );
    }
    partitioned_memory
        .speculative_diagnostic_choices_dispatch_ranges(choices, selection)
        .map(|(draft, target)| Some(SpeculativeDiagnosticChoiceRangesV1 { draft, target }))
        .map_err(
            |error| M1PhysicalBufferBindingErrorV1::SpeculativeDiagnosticChoicesRange { error },
        )
}

fn speculative_diagnostic_choice_source_isolation(
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    shape: crate::M1SpeculativeDiagnosticChoicesShapeV1,
) -> (usize, usize, usize, bool) {
    let mut draft_rows = 0;
    let mut draft_whole = 0;
    let mut target_rows = 0;
    let exact = recipe.rows().iter().all(|row| {
        row.buffers().iter().all(|buffer| match buffer.source() {
            M1PhysicalBufferSourceV1::SpeculativeDraftChoices(choice) => {
                draft_rows += 1;
                choice.iteration() < shape.draft_tokens()
                    && choice.producer_segment() == choice.iteration()
                    && choice.sequence_count() == shape.sequences()
            }
            M1PhysicalBufferSourceV1::Workspace {
                workspace: crate::M1FullStepWorkspaceRole::Target,
                range: ferric_build::M1StepWorkspaceRangeRole::DraftChoices,
            } => {
                draft_whole += 1;
                true
            }
            M1PhysicalBufferSourceV1::Workspace {
                workspace: crate::M1FullStepWorkspaceRole::Target,
                range: ferric_build::M1StepWorkspaceRangeRole::Choices,
            } => {
                target_rows += 1;
                row.segment_index() == shape.draft_tokens() && row.selection() == shape.selection()
            }
            _ => true,
        })
    });
    (draft_rows, draft_whole, target_rows, exact)
}

fn qualification_logits_source_isolation(
    recipe: &AddresslessM1PhysicalBufferRecipeV1,
    selection: ferric_spec::Qwen3PlanSelection,
) -> (usize, bool) {
    let mut sources = 0;
    let exact = recipe.rows().iter().all(|row| {
        row.buffers().iter().all(|buffer| {
            let is_logits = matches!(
                buffer.source(),
                M1PhysicalBufferSourceV1::Workspace {
                    range: ferric_build::M1StepWorkspaceRangeRole::Logits,
                    ..
                }
            );
            if is_logits {
                sources += 1;
            }
            !is_logits
                || matches!(
                    buffer.source(),
                    M1PhysicalBufferSourceV1::Workspace {
                        workspace: crate::M1FullStepWorkspaceRole::Target,
                        range: ferric_build::M1StepWorkspaceRangeRole::Logits,
                    }
                ) && row.segment_index() == 0
                    && row.selection() == selection
        })
    });
    (sources, exact)
}

fn validate_completion_output_shape(
    dispatch_index: u32,
    argument: usize,
    selection: ferric_spec::Qwen3PlanSelection,
    source_sequences: u32,
    actual: M1CompletionOutputShapeV1,
) -> Result<(), M1PhysicalBufferBindingErrorV1> {
    let expected = m1_completion_output_shape_v1(selection).map_err(|_| {
        M1PhysicalBufferBindingErrorV1::CompletionOutputShape {
            dispatch_index,
            argument,
            expected_selection: selection,
            actual_selection: actual.selection(),
            expected_sequences: 0,
            source_sequences,
            actual_sequences: actual.sequences(),
            expected_extent: 0,
            actual_extent: actual.extent_bytes(),
        }
    })?;
    if actual != expected || source_sequences != expected.sequences() {
        return Err(M1PhysicalBufferBindingErrorV1::CompletionOutputShape {
            dispatch_index,
            argument,
            expected_selection: selection,
            actual_selection: actual.selection(),
            expected_sequences: expected.sequences(),
            source_sequences,
            actual_sequences: actual.sequences(),
            expected_extent: expected.extent_bytes(),
            actual_extent: actual.extent_bytes(),
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolvedM1PhysicalBufferRangeV1 {
    Device(ServiceDeviceDispatchRangeV1),
    HostVisible(ServiceHostDispatchRangeV1),
    CompletionOutput(CompletionOutputBindingRangeV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CompletionOutputBindingRangeV1 {
    interior: ServiceHostDispatchRangeV1,
    snapshot: Option<ServiceHostDispatchSnapshotRangeV1>,
}

#[derive(Clone, Copy)]
struct SourceResolutionContextV1<'a> {
    partitioned_memory: &'a M1PartitionedModelMemoryKvPoolV1,
    workspaces: &'a BoundM1FullStepWorkspaceSubleases,
    completion_shape: M1CompletionOutputShapeV1,
    completion_binding: CompletionOutputBindingRangeV1,
    direct_diagnostic_choices_range: Option<ServiceHostDispatchRangeV1>,
    qualification_logits_range: Option<ServiceHostDispatchRangeV1>,
    speculative_diagnostic_choices_ranges: Option<SpeculativeDiagnosticChoiceRangesV1>,
}

impl ResolvedM1PhysicalBufferRangeV1 {
    fn into_fixed_buffer(
        self,
        dispatch_index: u32,
        explicit_argument_index: usize,
    ) -> Result<ServiceFixedDispatchBufferV1, M1PhysicalBufferBindingErrorV1> {
        match self {
            Self::Device(range) => Ok(ServiceFixedDispatchBufferV1::new(
                explicit_argument_index,
                range,
            )),
            Self::HostVisible(range) => Ok(ServiceFixedDispatchBufferV1::new_host_visible(
                explicit_argument_index,
                range,
            )),
            Self::CompletionOutput(CompletionOutputBindingRangeV1 {
                interior,
                snapshot: None,
            }) => Ok(ServiceFixedDispatchBufferV1::new_host_visible(
                explicit_argument_index,
                interior,
            )),
            Self::CompletionOutput(CompletionOutputBindingRangeV1 {
                interior,
                snapshot: Some(snapshot),
            }) => ServiceFixedDispatchBufferV1::new_host_visible_with_completed_snapshot(
                explicit_argument_index,
                interior,
                snapshot,
            )
            .map_err(|error| {
                M1PhysicalBufferBindingErrorV1::CompletionSnapshotAssociation {
                    dispatch_index,
                    argument: explicit_argument_index,
                    error,
                }
            }),
        }
    }
}

trait M1PhysicalBufferResolutionBackendV1 {
    type ResolvedRange;
    type Buffer;

    fn speculative_diagnostics_enabled(&self) -> bool;
    fn direct_diagnostics_enabled(&self) -> bool;
    fn qualification_logits_enabled(&self) -> bool;

    fn resolve_diagnostic_source(
        &mut self,
        row: &M1PhysicalBufferRecipeRowV1,
        argument: usize,
        source: M1PhysicalBufferSourceV1,
        route: SpeculativeDiagnosticChoiceSourceRouteV1,
    ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1>;

    fn resolve_device_source(
        &mut self,
        row: &M1PhysicalBufferRecipeRowV1,
        argument: usize,
        source: M1PhysicalBufferSourceV1,
    ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1>;

    fn resolve_normal_host_source(
        &mut self,
        row: &M1PhysicalBufferRecipeRowV1,
        argument: usize,
        source: M1PhysicalBufferSourceV1,
    ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1>;

    fn bind_buffer(
        &mut self,
        dispatch_index: u32,
        argument: usize,
        range: Self::ResolvedRange,
    ) -> Result<Self::Buffer, M1PhysicalBufferBindingErrorV1>;
}

struct M1ResolvedPhysicalBufferRowV1<T> {
    dispatch_index: u32,
    profile_id: Identity,
    program: M1PhysicalProgramV1,
    buffers: Box<[T]>,
}

struct M1ProductionPhysicalBufferResolutionBackendV1<'a> {
    context: &'a SourceResolutionContextV1<'a>,
}

impl M1PhysicalBufferResolutionBackendV1 for M1ProductionPhysicalBufferResolutionBackendV1<'_> {
    type ResolvedRange = ResolvedM1PhysicalBufferRangeV1;
    type Buffer = ServiceFixedDispatchBufferV1;

    fn speculative_diagnostics_enabled(&self) -> bool {
        self.context.speculative_diagnostic_choices_ranges.is_some()
    }

    fn direct_diagnostics_enabled(&self) -> bool {
        self.context.direct_diagnostic_choices_range.is_some()
    }

    fn qualification_logits_enabled(&self) -> bool {
        self.context.qualification_logits_range.is_some()
    }

    fn resolve_diagnostic_source(
        &mut self,
        row: &M1PhysicalBufferRecipeRowV1,
        argument: usize,
        source: M1PhysicalBufferSourceV1,
        route: SpeculativeDiagnosticChoiceSourceRouteV1,
    ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
        resolve_production_diagnostic_source(self.context, row, argument, source, route)
    }

    fn resolve_device_source(
        &mut self,
        row: &M1PhysicalBufferRecipeRowV1,
        argument: usize,
        source: M1PhysicalBufferSourceV1,
    ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
        resolve_production_ordinary_source(self.context, row, argument, source)
    }

    fn resolve_normal_host_source(
        &mut self,
        row: &M1PhysicalBufferRecipeRowV1,
        argument: usize,
        source: M1PhysicalBufferSourceV1,
    ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
        resolve_production_ordinary_source(self.context, row, argument, source)
    }

    fn bind_buffer(
        &mut self,
        dispatch_index: u32,
        argument: usize,
        range: Self::ResolvedRange,
    ) -> Result<Self::Buffer, M1PhysicalBufferBindingErrorV1> {
        range.into_fixed_buffer(dispatch_index, argument)
    }
}

fn resolve_rows(
    kernargs: &AddresslessM1PhysicalKernargRecipeV1,
    source_rows: &[M1PhysicalBufferRecipeRowV1],
    context: &SourceResolutionContextV1<'_>,
) -> Result<Box<[M1BoundPhysicalBufferRowV1]>, M1PhysicalBufferBindingErrorV1> {
    let mut backend = M1ProductionPhysicalBufferResolutionBackendV1 { context };
    resolve_rows_with_backend(kernargs, source_rows, &mut backend).map(|rows| {
        rows.into_vec()
            .into_iter()
            .map(|row| M1BoundPhysicalBufferRowV1 {
                dispatch_index: row.dispatch_index,
                profile_id: row.profile_id,
                program: row.program,
                buffers: row.buffers,
            })
            .collect()
    })
}

fn resolve_rows_with_backend<B: M1PhysicalBufferResolutionBackendV1>(
    kernargs: &AddresslessM1PhysicalKernargRecipeV1,
    source_rows: &[M1PhysicalBufferRecipeRowV1],
    backend: &mut B,
) -> Result<Box<[M1ResolvedPhysicalBufferRowV1<B::Buffer>]>, M1PhysicalBufferBindingErrorV1> {
    let physical_rows = kernargs.source_recipe().rows();
    if source_rows.len() != physical_rows.len() {
        return Err(M1PhysicalBufferBindingErrorV1::RowMetadata { dispatch_index: 0 });
    }
    validate_row_metadata(source_rows, physical_rows)?;
    let mut rows = Vec::with_capacity(source_rows.len());
    for (position, source_row) in source_rows.iter().enumerate() {
        let dispatch_index =
            u32::try_from(position).map_err(|_| M1PhysicalBufferBindingErrorV1::RowMetadata {
                dispatch_index: u32::MAX,
            })?;
        let mut buffers = Vec::with_capacity(source_row.buffers().len());
        for (buffer_position, source_buffer) in source_row.buffers().iter().enumerate() {
            let expected_ordinal = validate_argument_ordinal(
                dispatch_index,
                buffer_position,
                source_buffer.explicit_argument_index(),
            )?;
            let range = resolve_source_with_backend(
                backend,
                source_row,
                source_buffer.explicit_argument_index(),
                source_buffer.source(),
            )?;
            buffers.push(backend.bind_buffer(dispatch_index, expected_ordinal, range)?);
        }
        if buffers.len() != source_row.buffers().len() {
            return Err(M1PhysicalBufferBindingErrorV1::BufferCount {
                dispatch_index,
                expected: source_row.buffers().len(),
                actual: buffers.len(),
            });
        }
        rows.push(M1ResolvedPhysicalBufferRowV1 {
            dispatch_index,
            profile_id: source_row.profile_id(),
            program: source_row.program(),
            buffers: buffers.into_boxed_slice(),
        });
    }
    Ok(rows.into_boxed_slice())
}

#[derive(Clone, Copy)]
struct BindingRowMetadata {
    expected_dispatch_index: u32,
    source_dispatch_index: u32,
    physical_dispatch_index: u32,
    source_segment_index: u8,
    physical_segment_index: u8,
    source_stage: crate::M1StepDispatchStage,
    physical_stage: crate::M1StepDispatchStage,
    source_selection: ferric_spec::Qwen3PlanSelection,
    physical_selection: ferric_spec::Qwen3PlanSelection,
    logical_ordinal_matches: bool,
    profile_matches: bool,
    kind_matches: bool,
    source_profile_id: Identity,
    physical_profile_id: Identity,
    source_program: M1PhysicalProgramV1,
    physical_program: M1PhysicalProgramV1,
}

fn validate_row_metadata(
    source_rows: &[M1PhysicalBufferRecipeRowV1],
    physical_rows: &[crate::M1PhysicalDispatchRecipeRowV1],
) -> Result<(), M1PhysicalBufferBindingErrorV1> {
    for (position, (source, physical)) in source_rows.iter().zip(physical_rows).enumerate() {
        let expected_dispatch_index =
            u32::try_from(position).map_err(|_| M1PhysicalBufferBindingErrorV1::RowMetadata {
                dispatch_index: u32::MAX,
            })?;
        validate_row_metadata_entry(BindingRowMetadata {
            expected_dispatch_index,
            source_dispatch_index: source.dispatch_index(),
            physical_dispatch_index: physical.dispatch_index(),
            source_segment_index: source.segment_index(),
            physical_segment_index: physical.segment_index(),
            source_stage: source.stage(),
            physical_stage: physical.stage(),
            source_selection: source.selection(),
            physical_selection: physical.selection(),
            logical_ordinal_matches: source.logical_ordinal() == physical.logical_ordinal(),
            profile_matches: source.profile() == physical.profile(),
            kind_matches: source.kind() == physical.kind(),
            source_profile_id: source.profile_id(),
            physical_profile_id: physical.profile_id(),
            source_program: source.program(),
            physical_program: physical.program(),
        })?;
    }
    Ok(())
}

fn validate_row_metadata_entry(
    metadata: BindingRowMetadata,
) -> Result<(), M1PhysicalBufferBindingErrorV1> {
    if metadata.source_dispatch_index != metadata.expected_dispatch_index
        || metadata.physical_dispatch_index != metadata.expected_dispatch_index
        || metadata.source_segment_index != metadata.physical_segment_index
        || metadata.source_stage != metadata.physical_stage
        || metadata.source_selection != metadata.physical_selection
        || !metadata.logical_ordinal_matches
        || !metadata.profile_matches
        || !metadata.kind_matches
        || metadata.source_profile_id != metadata.physical_profile_id
        || metadata.source_program != metadata.physical_program
    {
        return Err(M1PhysicalBufferBindingErrorV1::RowMetadata {
            dispatch_index: metadata.expected_dispatch_index,
        });
    }
    Ok(())
}

fn validate_argument_ordinal(
    dispatch_index: u32,
    buffer_position: usize,
    actual: usize,
) -> Result<usize, M1PhysicalBufferBindingErrorV1> {
    let expected =
        buffer_position
            .checked_mul(2)
            .ok_or(M1PhysicalBufferBindingErrorV1::ArgumentOrdinal {
                dispatch_index,
                expected: usize::MAX,
                actual,
            })?;
    if actual != expected {
        return Err(M1PhysicalBufferBindingErrorV1::ArgumentOrdinal {
            dispatch_index,
            expected,
            actual,
        });
    }
    Ok(expected)
}

fn resolve_source_with_backend<B: M1PhysicalBufferResolutionBackendV1>(
    backend: &mut B,
    row: &M1PhysicalBufferRecipeRowV1,
    argument: usize,
    source: M1PhysicalBufferSourceV1,
) -> Result<B::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
    let route = speculative_diagnostic_choice_source_route(
        source,
        backend.speculative_diagnostics_enabled(),
        backend.direct_diagnostics_enabled(),
    )
    .map_err(|()| M1PhysicalBufferBindingErrorV1::SpeculativeDiagnosticChoicesIntent)?;
    match route {
        SpeculativeDiagnosticChoiceSourceRouteV1::OrdinaryDevice => {
            let normal_host = matches!(source, M1PhysicalBufferSourceV1::CompletionOutput { .. })
                || matches!(
                    source,
                    M1PhysicalBufferSourceV1::Workspace {
                        workspace: crate::M1FullStepWorkspaceRole::Target,
                        range: ferric_build::M1StepWorkspaceRangeRole::Logits,
                    }
                ) && backend.qualification_logits_enabled();
            if normal_host {
                backend.resolve_normal_host_source(row, argument, source)
            } else {
                backend.resolve_device_source(row, argument, source)
            }
        }
        route => backend.resolve_diagnostic_source(row, argument, source, route),
    }
}

fn resolve_production_diagnostic_source(
    context: &SourceResolutionContextV1<'_>,
    row: &M1PhysicalBufferRecipeRowV1,
    argument: usize,
    source: M1PhysicalBufferSourceV1,
    diagnostic_route: SpeculativeDiagnosticChoiceSourceRouteV1,
) -> Result<ResolvedM1PhysicalBufferRangeV1, M1PhysicalBufferBindingErrorV1> {
    let dispatch_index = row.dispatch_index();
    let diagnostic_ranges = context.speculative_diagnostic_choices_ranges;
    match diagnostic_route {
        SpeculativeDiagnosticChoiceSourceRouteV1::OrdinaryDevice => {
            unreachable!("ordinary sources use the production ordinary resolver")
        }
        SpeculativeDiagnosticChoiceSourceRouteV1::DirectTargetWholeHost => {
            Ok(ResolvedM1PhysicalBufferRangeV1::HostVisible(
                context
                    .direct_diagnostic_choices_range
                    .expect("direct diagnostic route retains exact range"),
            ))
        }
        SpeculativeDiagnosticChoiceSourceRouteV1::DraftWholeHost => {
            Ok(ResolvedM1PhysicalBufferRangeV1::HostVisible(
                diagnostic_ranges
                    .expect("diagnostic route retains exact ranges")
                    .draft,
            ))
        }
        SpeculativeDiagnosticChoiceSourceRouteV1::TargetWholeHost => {
            Ok(ResolvedM1PhysicalBufferRangeV1::HostVisible(
                diagnostic_ranges
                    .expect("diagnostic route retains exact ranges")
                    .target,
            ))
        }
        SpeculativeDiagnosticChoiceSourceRouteV1::DraftScalarHost {
            iteration,
            relative_offset,
            extent,
            alignment,
        } => {
            let M1PhysicalBufferSourceV1::SpeculativeDraftChoices(expected) = source else {
                unreachable!("draft-scalar route requires a draft-choice source")
            };
            debug_assert_eq!(expected.iteration(), iteration);
            if context
                .workspaces
                .speculative_draft_choice_subrange(expected.producer_segment())
                != Some(expected)
            {
                return Err(M1PhysicalBufferBindingErrorV1::RowMetadata { dispatch_index });
            }
            diagnostic_ranges
                .expect("diagnostic route retains exact ranges")
                .draft
                .checked_subrange(relative_offset, extent, alignment)
                .map(ResolvedM1PhysicalBufferRangeV1::HostVisible)
                .map_err(|error| {
                    M1PhysicalBufferBindingErrorV1::SpeculativeDiagnosticDraftSubrange {
                        dispatch_index,
                        argument,
                        error,
                    }
                })
        }
    }
}

fn resolve_production_ordinary_source(
    context: &SourceResolutionContextV1<'_>,
    row: &M1PhysicalBufferRecipeRowV1,
    argument: usize,
    source: M1PhysicalBufferSourceV1,
) -> Result<ResolvedM1PhysicalBufferRangeV1, M1PhysicalBufferBindingErrorV1> {
    let dispatch_index = row.dispatch_index();
    match source {
        M1PhysicalBufferSourceV1::Workspace { workspace, range }
            if workspace == crate::M1FullStepWorkspaceRole::Target
                && range == ferric_build::M1StepWorkspaceRangeRole::Logits
                && context.qualification_logits_range.is_some() =>
        {
            Ok(ResolvedM1PhysicalBufferRangeV1::HostVisible(
                context
                    .qualification_logits_range
                    .expect("qualification logits guard retains exact range"),
            ))
        }
        M1PhysicalBufferSourceV1::Workspace { workspace, range } => context
            .partitioned_memory
            .workspace_segment_dispatch_range(
                context.workspaces,
                row.segment_index(),
                workspace,
                range,
            )
            .map_err(|error| M1PhysicalBufferBindingErrorV1::WorkspaceRange {
                dispatch_index,
                argument,
                error,
            })
            .map(ResolvedM1PhysicalBufferRangeV1::Device),
        M1PhysicalBufferSourceV1::WorkspaceSentinel {
            workspace,
            range,
            purpose,
        } => {
            let parent = context
                .partitioned_memory
                .workspace_segment_dispatch_range(
                    context.workspaces,
                    row.segment_index(),
                    workspace,
                    range,
                )
                .map_err(|error| M1PhysicalBufferBindingErrorV1::WorkspaceRange {
                    dispatch_index,
                    argument,
                    error,
                })?;
            let (relative_offset, extent, alignment) = sentinel_geometry(purpose);
            parent
                .checked_subrange(relative_offset, extent, alignment)
                .map_err(|error| M1PhysicalBufferBindingErrorV1::SentinelRange {
                    dispatch_index,
                    argument,
                    error,
                })
                .map(ResolvedM1PhysicalBufferRangeV1::Device)
        }
        M1PhysicalBufferSourceV1::SpeculativeDraftChoices(expected) => {
            if context
                .workspaces
                .speculative_draft_choice_subrange(expected.producer_segment())
                != Some(expected)
            {
                return Err(M1PhysicalBufferBindingErrorV1::RowMetadata { dispatch_index });
            }
            context
                .partitioned_memory
                .speculative_draft_choice_dispatch_range(
                    context.workspaces,
                    expected.producer_segment(),
                )
                .map_err(|error| M1PhysicalBufferBindingErrorV1::WorkspaceRange {
                    dispatch_index,
                    argument,
                    error,
                })
                .map(ResolvedM1PhysicalBufferRangeV1::Device)
        }
        M1PhysicalBufferSourceV1::SpeculativeDraftAnchorTokenIds {
            workspace,
            range,
            verification_segment,
        } => {
            if workspace != crate::M1FullStepWorkspaceRole::Draft
                || range != ferric_build::M1StepWorkspaceRangeRole::TokenIds
                || verification_segment != row.segment_index()
            {
                return Err(M1PhysicalBufferBindingErrorV1::RowMetadata { dispatch_index });
            }
            context
                .partitioned_memory
                .speculative_token_assembly_anchor_dispatch_range(
                    context.workspaces,
                    verification_segment,
                )
                .map_err(|error| M1PhysicalBufferBindingErrorV1::WorkspaceRange {
                    dispatch_index,
                    argument,
                    error,
                })
                .map(ResolvedM1PhysicalBufferRangeV1::Device)
        }
        M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
            workspace,
            range,
            draft_segment,
            iteration,
        } => {
            let exact = match range {
                ferric_build::M1StepWorkspaceRangeRole::DraftPositionIds => context
                    .workspaces
                    .speculative_draft_position_subrange(draft_segment),
                ferric_build::M1StepWorkspaceRangeRole::DraftContextLengths => context
                    .workspaces
                    .speculative_draft_context_subrange(draft_segment),
                _ => None,
            };
            if workspace != crate::M1FullStepWorkspaceRole::Target
                || draft_segment != row.segment_index()
                || exact.is_none_or(|metadata| {
                    metadata.draft_segment() != draft_segment
                        || metadata.iteration() != iteration
                        || metadata.range().role() != range
                })
            {
                return Err(M1PhysicalBufferBindingErrorV1::RowMetadata { dispatch_index });
            }
            let resolved = match range {
                ferric_build::M1StepWorkspaceRangeRole::DraftPositionIds => context
                    .partitioned_memory
                    .speculative_draft_position_dispatch_range(context.workspaces, draft_segment),
                ferric_build::M1StepWorkspaceRangeRole::DraftContextLengths => context
                    .partitioned_memory
                    .speculative_draft_context_dispatch_range(context.workspaces, draft_segment),
                _ => unreachable!(),
            };
            resolved
                .map_err(|error| M1PhysicalBufferBindingErrorV1::WorkspaceRange {
                    dispatch_index,
                    argument,
                    error,
                })
                .map(ResolvedM1PhysicalBufferRangeV1::Device)
        }
        M1PhysicalBufferSourceV1::ModelWeight { role, kind, layer } => context
            .partitioned_memory
            .weight_dispatch_range(role, kind, layer)
            .map_err(|error| M1PhysicalBufferBindingErrorV1::ModelMemoryRange {
                dispatch_index,
                argument,
                error,
            })
            .map(ResolvedM1PhysicalBufferRangeV1::Device),
        M1PhysicalBufferSourceV1::KvCachePlane {
            role,
            component,
            layer,
        } => context
            .partitioned_memory
            .kv_dispatch_range(role, component, layer)
            .map_err(|error| M1PhysicalBufferBindingErrorV1::PartitionedKvRange {
                dispatch_index,
                argument,
                error,
            })
            .map(ResolvedM1PhysicalBufferRangeV1::Device),
        M1PhysicalBufferSourceV1::CompletionOutput { sequences } => {
            validate_completion_output_shape(
                dispatch_index,
                argument,
                row.selection(),
                sequences,
                context.completion_shape,
            )?;
            Ok(ResolvedM1PhysicalBufferRangeV1::CompletionOutput(
                context.completion_binding,
            ))
        }
        M1PhysicalBufferSourceV1::SpeculativeTargetTokenIds { .. } => {
            Err(M1PhysicalBufferBindingErrorV1::MaterializationRequired {
                dispatch_index,
                argument,
                source,
            })
        }
    }
}

const fn sentinel_geometry(purpose: M1PhysicalBufferSentinelV1) -> (u64, u64, u64) {
    match purpose {
        M1PhysicalBufferSentinelV1::RmsInactiveResidual
        | M1PhysicalBufferSentinelV1::RmsInactiveFusedOutput => (0, 2, 2),
        M1PhysicalBufferSentinelV1::CompactNoDraftTokens => (0, 4, 4),
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_service_host::ServiceAllocationErrorV1;
    use ferric_build::{KvCacheComponent, M1StepWorkspaceRangeRole};
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3TensorKind,
        QWEN3_NO_LAYER,
    };

    use super::{
        direct_diagnostic_choice_source_isolation, first_materialization_requirement,
        qualification_logits_source_isolation, resolve_rows_with_backend, sentinel_geometry,
        speculative_diagnostic_choice_source_isolation,
        speculative_diagnostic_draft_choice_geometry, validate_argument_ordinal,
        validate_completion_output_shape, validate_row_metadata_entry, BindingRowMetadata,
        M1PhysicalBufferBindingErrorV1, M1PhysicalBufferResolutionBackendV1,
        SpeculativeDiagnosticChoiceSourceRouteV1,
    };
    use crate::physical_buffer_recipe::tests::{complete_intents, exact_inputs};
    use crate::{
        derive_m1_physical_buffer_recipe_v1, m1_completion_output_shape_v1,
        m1_speculative_diagnostic_choices_shape_v1, AddresslessM1PhysicalBufferRecipeV1,
        M1FullStepWorkspaceDispatchRangeError, M1FullStepWorkspaceRole, M1PhysicalBufferAccessV1,
        M1PhysicalBufferRecipeRowV1, M1PhysicalBufferSentinelV1, M1PhysicalBufferSourceV1,
        M1PhysicalProgramV1, M1StepDispatchIntent, M1StepDispatchStage,
        M1StepWorkspaceDispatchRangeError,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TestResolutionRouteV1 {
        Device,
        HostVisible,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestResolvedRangeV1 {
        source: M1PhysicalBufferSourceV1,
        route: TestResolutionRouteV1,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestBoundRangeV1 {
        argument: usize,
        source: M1PhysicalBufferSourceV1,
        route: TestResolutionRouteV1,
    }

    #[derive(Debug, Default)]
    struct DiagnosticOffResolutionBackendV1;

    #[derive(Debug, Default)]
    struct DirectDiagnosticResolutionBackendV1 {
        diagnostic_routes: Vec<SpeculativeDiagnosticChoiceSourceRouteV1>,
    }

    impl M1PhysicalBufferResolutionBackendV1 for DiagnosticOffResolutionBackendV1 {
        type ResolvedRange = TestResolvedRangeV1;
        type Buffer = TestBoundRangeV1;

        fn speculative_diagnostics_enabled(&self) -> bool {
            false
        }

        fn direct_diagnostics_enabled(&self) -> bool {
            false
        }

        fn qualification_logits_enabled(&self) -> bool {
            false
        }

        fn resolve_diagnostic_source(
            &mut self,
            _row: &M1PhysicalBufferRecipeRowV1,
            _argument: usize,
            _source: M1PhysicalBufferSourceV1,
            _route: SpeculativeDiagnosticChoiceSourceRouteV1,
        ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
            panic!("diagnostic-off binding must not enter a host diagnostic route")
        }

        fn resolve_device_source(
            &mut self,
            row: &M1PhysicalBufferRecipeRowV1,
            argument: usize,
            source: M1PhysicalBufferSourceV1,
        ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
            if matches!(
                source,
                M1PhysicalBufferSourceV1::SpeculativeTargetTokenIds { .. }
            ) {
                return Err(M1PhysicalBufferBindingErrorV1::MaterializationRequired {
                    dispatch_index: row.dispatch_index(),
                    argument,
                    source,
                });
            }
            Ok(TestResolvedRangeV1 {
                source,
                route: TestResolutionRouteV1::Device,
            })
        }

        fn resolve_normal_host_source(
            &mut self,
            _row: &M1PhysicalBufferRecipeRowV1,
            _argument: usize,
            source: M1PhysicalBufferSourceV1,
        ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
            Ok(TestResolvedRangeV1 {
                source,
                route: TestResolutionRouteV1::HostVisible,
            })
        }

        fn bind_buffer(
            &mut self,
            _dispatch_index: u32,
            argument: usize,
            range: Self::ResolvedRange,
        ) -> Result<Self::Buffer, M1PhysicalBufferBindingErrorV1> {
            Ok(TestBoundRangeV1 {
                argument,
                source: range.source,
                route: range.route,
            })
        }
    }

    impl M1PhysicalBufferResolutionBackendV1 for DirectDiagnosticResolutionBackendV1 {
        type ResolvedRange = TestResolvedRangeV1;
        type Buffer = TestBoundRangeV1;

        fn speculative_diagnostics_enabled(&self) -> bool {
            false
        }

        fn direct_diagnostics_enabled(&self) -> bool {
            true
        }

        fn qualification_logits_enabled(&self) -> bool {
            false
        }

        fn resolve_diagnostic_source(
            &mut self,
            _row: &M1PhysicalBufferRecipeRowV1,
            _argument: usize,
            source: M1PhysicalBufferSourceV1,
            route: SpeculativeDiagnosticChoiceSourceRouteV1,
        ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
            self.diagnostic_routes.push(route);
            Ok(TestResolvedRangeV1 {
                source,
                route: TestResolutionRouteV1::HostVisible,
            })
        }

        fn resolve_device_source(
            &mut self,
            row: &M1PhysicalBufferRecipeRowV1,
            argument: usize,
            source: M1PhysicalBufferSourceV1,
        ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
            if matches!(
                source,
                M1PhysicalBufferSourceV1::SpeculativeTargetTokenIds { .. }
            ) {
                return Err(M1PhysicalBufferBindingErrorV1::MaterializationRequired {
                    dispatch_index: row.dispatch_index(),
                    argument,
                    source,
                });
            }
            Ok(TestResolvedRangeV1 {
                source,
                route: TestResolutionRouteV1::Device,
            })
        }

        fn resolve_normal_host_source(
            &mut self,
            _row: &M1PhysicalBufferRecipeRowV1,
            _argument: usize,
            source: M1PhysicalBufferSourceV1,
        ) -> Result<Self::ResolvedRange, M1PhysicalBufferBindingErrorV1> {
            Ok(TestResolvedRangeV1 {
                source,
                route: TestResolutionRouteV1::HostVisible,
            })
        }

        fn bind_buffer(
            &mut self,
            _dispatch_index: u32,
            argument: usize,
            range: Self::ResolvedRange,
        ) -> Result<Self::Buffer, M1PhysicalBufferBindingErrorV1> {
            Ok(TestBoundRangeV1 {
                argument,
                source: range.source,
                route: range.route,
            })
        }
    }

    fn exact_recipe(
        intent: M1StepDispatchIntent,
        identity_byte: u8,
    ) -> AddresslessM1PhysicalBufferRecipeV1 {
        let (kernargs, workspaces) = exact_inputs(intent, identity_byte);
        derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap()
    }

    fn target(
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> ferric_spec::Qwen3PlanSelection {
        ferric_spec::Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    fn non_speculative_intents() -> [M1StepDispatchIntent; 11] {
        let all = complete_intents();
        core::array::from_fn(|index| all[index])
    }

    #[test]
    fn lossless_parts_revalidate_all_complete_intents_and_rows() {
        let mut programs = Vec::new();
        for (case, intent) in complete_intents().into_iter().enumerate() {
            let recipe = exact_recipe(intent, 10 + u8::try_from(case).unwrap() * 2);
            recipe.revalidate().unwrap();
            for row in recipe.rows() {
                if !programs.contains(&row.program()) {
                    programs.push(row.program());
                }
            }
            let (kernargs, workspaces, rows) = recipe.into_parts();
            let recovered =
                AddresslessM1PhysicalBufferRecipeV1::from_parts(kernargs, workspaces, rows);
            recovered.revalidate().unwrap();
        }
        programs.sort_unstable();
        assert_eq!(programs, M1PhysicalProgramV1::ALL);
    }

    #[test]
    fn every_complete_intent_is_structurally_binding_ready() {
        for (case, intent) in complete_intents().into_iter().enumerate() {
            let recipe = exact_recipe(intent, 50 + u8::try_from(case).unwrap() * 2);
            assert!(!recipe.requires_future_materialization());
            assert!(first_materialization_requirement(&recipe).is_none());
            let physical_rows = recipe.kernarg_recipe().source_recipe().rows();
            assert_eq!(recipe.rows().len(), physical_rows.len());
            for (position, (source, physical)) in
                recipe.rows().iter().zip(physical_rows).enumerate()
            {
                validate_row_metadata_entry(metadata(position, source, physical)).unwrap();
                for (buffer_position, buffer) in source.buffers().iter().enumerate() {
                    assert_eq!(
                        validate_argument_ordinal(
                            source.dispatch_index(),
                            buffer_position,
                            buffer.explicit_argument_index(),
                        )
                        .unwrap(),
                        buffer.explicit_argument_index()
                    );
                }
            }
        }
    }

    #[test]
    fn every_complete_intent_has_one_exact_host_completion_source() {
        for (case, intent) in complete_intents().into_iter().enumerate() {
            let recipe = exact_recipe(intent, 70 + u8::try_from(case).unwrap() * 2);
            let outputs = recipe
                .rows()
                .iter()
                .flat_map(|row| {
                    row.buffers().iter().filter_map(move |buffer| {
                        let M1PhysicalBufferSourceV1::CompletionOutput { sequences } =
                            buffer.source()
                        else {
                            return None;
                        };
                        Some((row, buffer.explicit_argument_index(), sequences))
                    })
                })
                .collect::<Vec<_>>();
            assert_eq!(outputs.len(), 1);
            let (row, argument, sequences) = outputs[0];
            assert_eq!(row.program(), M1PhysicalProgramV1::LogitsCompact);
            assert_eq!(argument, 14);
            let shape = m1_completion_output_shape_v1(intent.target_selection()).unwrap();
            validate_completion_output_shape(
                row.dispatch_index(),
                argument,
                row.selection(),
                sequences,
                shape,
            )
            .unwrap();
        }
    }

    #[test]
    fn qualification_override_isolated_to_two_target_logits_arguments() {
        let intent = M1StepDispatchIntent::TargetOnly(target(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        ));
        let recipe = exact_recipe(intent, 210);
        assert_eq!(
            qualification_logits_source_isolation(&recipe, intent.target_selection()),
            (2, true)
        );
        let access_routes = recipe
            .rows()
            .iter()
            .flat_map(|row| {
                row.buffers().iter().filter_map(move |buffer| {
                    matches!(
                        buffer.source(),
                        M1PhysicalBufferSourceV1::Workspace {
                            workspace: M1FullStepWorkspaceRole::Target,
                            range: M1StepWorkspaceRangeRole::Logits,
                        }
                    )
                    .then_some((
                        row.program(),
                        buffer.explicit_argument_index(),
                        buffer.access(),
                    ))
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            access_routes,
            vec![
                (
                    M1PhysicalProgramV1::GemmReference,
                    4,
                    M1PhysicalBufferAccessV1::ReadWrite,
                ),
                (
                    M1PhysicalProgramV1::LogitsArgmax,
                    0,
                    M1PhysicalBufferAccessV1::ReadOnly,
                ),
            ]
        );
        let total = recipe
            .rows()
            .iter()
            .map(|row| row.buffers().len())
            .sum::<usize>();
        let untouched = recipe
            .rows()
            .iter()
            .flat_map(M1PhysicalBufferRecipeRowV1::buffers)
            .filter(|buffer| {
                !matches!(
                    buffer.source(),
                    M1PhysicalBufferSourceV1::Workspace {
                        workspace: M1FullStepWorkspaceRole::Target,
                        range: M1StepWorkspaceRangeRole::Logits,
                    }
                )
            })
            .count();
        assert_eq!(untouched + 2, total);
    }

    #[test]
    fn every_serving_direct_shape_isolates_exact_target_choice_sources() {
        for (case, intent) in [
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS32C8192,
            )),
        ]
        .into_iter()
        .enumerate()
        {
            let recipe = exact_recipe(intent, 220 + u8::try_from(case).unwrap() * 2);
            let target_segment = u8::from(matches!(intent, M1StepDispatchIntent::PairedPrefill(_)));
            assert_eq!(
                direct_diagnostic_choice_source_isolation(
                    &recipe,
                    intent.target_selection(),
                    target_segment,
                ),
                (2, true)
            );
            let target_choices = recipe
                .rows()
                .iter()
                .flat_map(M1PhysicalBufferRecipeRowV1::buffers)
                .filter(|buffer| {
                    matches!(
                        buffer.source(),
                        M1PhysicalBufferSourceV1::Workspace {
                            workspace: M1FullStepWorkspaceRole::Target,
                            range: M1StepWorkspaceRangeRole::Choices,
                        }
                    )
                })
                .count();
            assert_eq!(target_choices, 2);
        }
    }

    #[test]
    fn every_serving_direct_shape_routes_both_target_choice_uses_to_host() {
        for (case, intent) in [
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
            )),
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS32C8192,
            )),
        ]
        .into_iter()
        .enumerate()
        {
            let recipe = exact_recipe(intent, 234 + u8::try_from(case).unwrap() * 2);
            let mut backend = DirectDiagnosticResolutionBackendV1::default();
            let resolved =
                resolve_rows_with_backend(recipe.kernarg_recipe(), recipe.rows(), &mut backend)
                    .unwrap();
            let routed_choices = resolved
                .iter()
                .flat_map(|row| row.buffers.iter())
                .filter(|buffer| {
                    matches!(
                        buffer.source,
                        M1PhysicalBufferSourceV1::Workspace {
                            workspace: M1FullStepWorkspaceRole::Target,
                            range: M1StepWorkspaceRangeRole::Choices,
                        }
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(routed_choices.len(), 2);
            assert!(routed_choices
                .iter()
                .all(|buffer| buffer.route == TestResolutionRouteV1::HostVisible));
            assert_eq!(
                backend.diagnostic_routes,
                vec![SpeculativeDiagnosticChoiceSourceRouteV1::DirectTargetWholeHost; 2]
            );
        }
    }

    #[test]
    fn every_finite_speculative_diagnostic_shape_has_exact_choice_source_roster() {
        for (case, bucket, draft_tokens) in [
            (0_u8, Qwen3PlanBucket::SpeculativeS1K4C8192, 4_usize),
            (1, Qwen3PlanBucket::SpeculativeS8K4C8192, 4),
            (2, Qwen3PlanBucket::SpeculativeS1K8C8192, 8),
            (3, Qwen3PlanBucket::SpeculativeS1K16C8192, 16),
        ] {
            let selection = target(Qwen3ExecutionMode::Speculative, bucket);
            let intent = M1StepDispatchIntent::SpeculativeRound(selection);
            let recipe = exact_recipe(intent, 212 + case);
            let shape = m1_speculative_diagnostic_choices_shape_v1(selection).unwrap();
            assert_eq!(
                speculative_diagnostic_choice_source_isolation(&recipe, shape),
                (draft_tokens * 2 - 1, 2, 2, true)
            );
            let draft_writes = recipe
                .rows()
                .iter()
                .flat_map(M1PhysicalBufferRecipeRowV1::buffers)
                .filter(|buffer| {
                    matches!(
                        buffer.source(),
                        M1PhysicalBufferSourceV1::SpeculativeDraftChoices(_)
                    ) && buffer.access() == M1PhysicalBufferAccessV1::WriteOnly
                })
                .count();
            let target_full_writes = recipe
                .rows()
                .iter()
                .flat_map(M1PhysicalBufferRecipeRowV1::buffers)
                .filter(|buffer| {
                    matches!(
                        buffer.source(),
                        M1PhysicalBufferSourceV1::Workspace {
                            workspace: M1FullStepWorkspaceRole::Target,
                            range: M1StepWorkspaceRangeRole::Choices,
                        }
                    ) && buffer.access() == M1PhysicalBufferAccessV1::WriteOnly
                })
                .count();
            assert_eq!(draft_writes, draft_tokens);
            assert_eq!(target_full_writes, 1);
        }
    }

    #[test]
    fn diagnostic_draft_subranges_are_exact_for_s1_and_s8_and_bounded_at_k16() {
        assert_eq!(
            (0..4)
                .map(|iteration| speculative_diagnostic_draft_choice_geometry(iteration, 1))
                .collect::<Vec<_>>(),
            vec![
                Some((0, 4, 4)),
                Some((4, 4, 4)),
                Some((8, 4, 4)),
                Some((12, 4, 4)),
            ]
        );
        assert_eq!(
            speculative_diagnostic_draft_choice_geometry(3, 8),
            Some((96, 32, 4))
        );
        assert_eq!(
            speculative_diagnostic_draft_choice_geometry(15, 1),
            Some((60, 4, 4))
        );
        assert_eq!(speculative_diagnostic_draft_choice_geometry(16, 1), None);
    }

    #[test]
    fn diagnostic_absence_traverses_normal_binding_and_preserves_device_routes() {
        let target_only = exact_recipe(
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            214,
        );
        let speculative = exact_recipe(
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            )),
            216,
        );

        for recipe in [&target_only, &speculative] {
            let mut backend = DiagnosticOffResolutionBackendV1;
            let resolved =
                resolve_rows_with_backend(recipe.kernarg_recipe(), recipe.rows(), &mut backend)
                    .unwrap();
            assert_eq!(resolved.len(), recipe.rows().len());
            for (source_row, resolved_row) in recipe.rows().iter().zip(resolved.iter()) {
                assert_eq!(resolved_row.dispatch_index, source_row.dispatch_index());
                assert_eq!(resolved_row.profile_id, source_row.profile_id());
                assert_eq!(resolved_row.program, source_row.program());
                assert_eq!(resolved_row.buffers.len(), source_row.buffers().len());
                for (source_buffer, resolved_buffer) in
                    source_row.buffers().iter().zip(resolved_row.buffers.iter())
                {
                    assert_eq!(
                        resolved_buffer.argument,
                        source_buffer.explicit_argument_index()
                    );
                    assert_eq!(resolved_buffer.source, source_buffer.source());
                    let expected_route = if matches!(
                        source_buffer.source(),
                        M1PhysicalBufferSourceV1::CompletionOutput { .. }
                    ) {
                        TestResolutionRouteV1::HostVisible
                    } else {
                        TestResolutionRouteV1::Device
                    };
                    assert_eq!(resolved_buffer.route, expected_route);
                }
            }
        }

        let diagnostic_sources = speculative
            .rows()
            .iter()
            .flat_map(M1PhysicalBufferRecipeRowV1::buffers)
            .filter(|buffer| {
                matches!(
                    buffer.source(),
                    M1PhysicalBufferSourceV1::SpeculativeDraftChoices(_)
                        | M1PhysicalBufferSourceV1::Workspace {
                            workspace: M1FullStepWorkspaceRole::Target,
                            range: M1StepWorkspaceRangeRole::DraftChoices
                                | M1StepWorkspaceRangeRole::Choices,
                        }
                )
            })
            .count();
        assert_eq!(diagnostic_sources, 11);
    }

    #[test]
    fn hostile_completion_selection_and_source_extent_fail_exactly() {
        let exact = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192);
        let stale = target(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128);
        let stale_shape = m1_completion_output_shape_v1(stale).unwrap();
        let error = validate_completion_output_shape(544, 14, exact, 8, stale_shape).unwrap_err();
        let M1PhysicalBufferBindingErrorV1::CompletionOutputShape {
            dispatch_index,
            argument,
            expected_selection,
            actual_selection,
            expected_sequences,
            source_sequences,
            actual_sequences,
            expected_extent,
            actual_extent,
        } = error
        else {
            panic!("completion selection drift lost its exact diagnostic")
        };
        assert_eq!((dispatch_index, argument), (544, 14));
        assert_eq!((expected_selection, actual_selection), (exact, stale));
        assert_eq!(
            (expected_sequences, source_sequences, actual_sequences),
            (8, 8, 8)
        );
        assert_eq!((expected_extent, actual_extent), (960, 960));

        let exact_shape = m1_completion_output_shape_v1(exact).unwrap();
        assert!(matches!(
            validate_completion_output_shape(544, 14, exact, 1, exact_shape),
            Err(M1PhysicalBufferBindingErrorV1::CompletionOutputShape {
                expected_sequences: 8,
                source_sequences: 1,
                actual_sequences: 8,
                expected_extent: 960,
                actual_extent: 960,
                ..
            })
        ));
    }

    #[test]
    fn paired_prefill_reaches_all_710_weight_and_128_kv_coordinates() {
        let recipe = exact_recipe(
            M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
            80,
        );
        let mut weights = Vec::new();
        let mut kv_planes = Vec::new();
        for source in recipe
            .rows()
            .iter()
            .flat_map(crate::M1PhysicalBufferRecipeRowV1::buffers)
            .map(|buffer| buffer.source())
        {
            match source {
                M1PhysicalBufferSourceV1::ModelWeight { role, kind, layer } => {
                    let coordinate = (role, kind, layer);
                    if !weights.contains(&coordinate) {
                        weights.push(coordinate);
                    }
                }
                M1PhysicalBufferSourceV1::KvCachePlane {
                    role,
                    component,
                    layer,
                } => {
                    let coordinate = (role, component, layer);
                    if !kv_planes.contains(&coordinate) {
                        kv_planes.push(coordinate);
                    }
                }
                _ => {}
            }
        }

        assert_eq!(weights.len(), 710);
        assert_eq!(
            weights
                .iter()
                .filter(|(role, _, _)| *role == Qwen3ModelRole::Target8B)
                .count(),
            399
        );
        assert_eq!(
            weights
                .iter()
                .filter(|(role, _, _)| *role == Qwen3ModelRole::Draft06B)
                .count(),
            311
        );
        for (role, kind, layer) in weights {
            match kind {
                Qwen3TensorKind::LanguageModelHead
                | Qwen3TensorKind::TokenEmbedding
                | Qwen3TensorKind::FinalNorm => assert_eq!(layer, QWEN3_NO_LAYER),
                _ => assert!(layer < role.layers()),
            }
        }

        assert_eq!(kv_planes.len(), 128);
        assert_eq!(
            kv_planes
                .iter()
                .filter(|(role, _, _)| *role == Qwen3ModelRole::Target8B)
                .count(),
            72
        );
        assert_eq!(
            kv_planes
                .iter()
                .filter(|(role, _, _)| *role == Qwen3ModelRole::Draft06B)
                .count(),
            56
        );
        for (role, component, layer) in kv_planes {
            assert!(layer < role.layers());
            assert!(matches!(
                component,
                KvCacheComponent::Key | KvCacheComponent::Value
            ));
        }
    }

    #[test]
    fn sentinel_sources_have_fixed_nonempty_aligned_geometry_and_exact_roles() {
        let mut purposes = Vec::new();
        for (case, intent) in non_speculative_intents().into_iter().enumerate() {
            let recipe = exact_recipe(intent, 105 + u8::try_from(case).unwrap() * 2);
            for row in recipe.rows() {
                for buffer in row.buffers() {
                    let M1PhysicalBufferSourceV1::WorkspaceSentinel {
                        workspace,
                        range,
                        purpose,
                    } = buffer.source()
                    else {
                        continue;
                    };
                    let (relative_offset, extent, alignment) = sentinel_geometry(purpose);
                    assert_eq!(relative_offset, 0);
                    assert!(extent > 0);
                    assert_eq!(extent, alignment);
                    match purpose {
                        M1PhysicalBufferSentinelV1::RmsInactiveResidual => {
                            assert_eq!((relative_offset, extent, alignment), (0, 2, 2));
                            assert_eq!(range, M1StepWorkspaceRangeRole::TokenIds);
                        }
                        M1PhysicalBufferSentinelV1::RmsInactiveFusedOutput => {
                            assert_eq!((relative_offset, extent, alignment), (0, 2, 2));
                            assert_eq!(range, M1StepWorkspaceRangeRole::PositionIds);
                        }
                        M1PhysicalBufferSentinelV1::CompactNoDraftTokens => {
                            assert_eq!((relative_offset, extent, alignment), (0, 4, 4));
                            assert_eq!(range, M1StepWorkspaceRangeRole::PositionIds);
                        }
                    }
                    assert_eq!(workspace_role_for_stage(row.stage()), workspace);
                    if !purposes.contains(&purpose) {
                        purposes.push(purpose);
                    }
                }
            }
        }
        assert_eq!(purposes.len(), 3);
    }

    #[test]
    fn every_speculative_shape_routes_assembly_and_exact_target_metadata() {
        for (case, intent) in complete_intents().into_iter().skip(11).enumerate() {
            let recipe = exact_recipe(intent, 140 + u8::try_from(case).unwrap() * 2);
            assert!(!recipe.requires_future_materialization());
            assert!(first_materialization_requirement(&recipe).is_none());
            let assembly = recipe
                .rows()
                .iter()
                .find(|row| row.program() == M1PhysicalProgramV1::SpeculativeTokenAssembly)
                .unwrap();
            assert_eq!(assembly.logical_ordinal(), None);
            assert_eq!(assembly.operator(), None);
            assert_eq!(assembly.buffers().len(), 3);
            assert!(matches!(
                assembly.buffers()[0].source(),
                M1PhysicalBufferSourceV1::SpeculativeDraftAnchorTokenIds {
                    workspace: M1FullStepWorkspaceRole::Draft,
                    range: M1StepWorkspaceRangeRole::TokenIds,
                    verification_segment,
                } if verification_segment == assembly.segment_index()
            ));
            assert_eq!(
                assembly.buffers()[1].source(),
                M1PhysicalBufferSourceV1::Workspace {
                    workspace: M1FullStepWorkspaceRole::Target,
                    range: M1StepWorkspaceRangeRole::DraftChoices,
                }
            );
            assert_eq!(
                assembly.buffers()[2].source(),
                M1PhysicalBufferSourceV1::Workspace {
                    workspace: M1FullStepWorkspaceRole::Target,
                    range: M1StepWorkspaceRangeRole::TokenIds,
                }
            );

            let mut position_rows = Vec::new();
            let mut context_rows = Vec::new();
            for source in recipe
                .rows()
                .iter()
                .flat_map(crate::M1PhysicalBufferRecipeRowV1::buffers)
                .map(|buffer| buffer.source())
            {
                let M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
                    workspace,
                    range,
                    draft_segment,
                    iteration,
                } = source
                else {
                    continue;
                };
                assert_eq!(workspace, M1FullStepWorkspaceRole::Target);
                assert!(iteration > 0);
                assert_eq!(draft_segment, iteration);
                match range {
                    M1StepWorkspaceRangeRole::DraftPositionIds => {
                        if !position_rows.contains(&(draft_segment, iteration)) {
                            position_rows.push((draft_segment, iteration));
                        }
                    }
                    M1StepWorkspaceRangeRole::DraftContextLengths => {
                        if !context_rows.contains(&(draft_segment, iteration)) {
                            context_rows.push((draft_segment, iteration));
                        }
                    }
                    _ => panic!("non-staged metadata source"),
                }
            }
            let draft_iterations = match assembly.stage() {
                M1StepDispatchStage::TargetVerification { draft_iterations } => draft_iterations,
                _ => unreachable!(),
            };
            assert_eq!(position_rows.len(), usize::from(draft_iterations - 1));
            assert_eq!(context_rows.len(), usize::from(draft_iterations - 1));

            let (kernargs, workspaces, rows) = recipe.into_parts();
            AddresslessM1PhysicalBufferRecipeV1::from_parts(kernargs, workspaces, rows)
                .revalidate()
                .unwrap();
        }
    }

    #[test]
    fn hostile_row_order_segment_stage_selection_profile_program_and_ordinal_fail() {
        let recipe = exact_recipe(
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            160,
        );
        let source = &recipe.rows()[0];
        let physical = &recipe.kernarg_recipe().source_recipe().rows()[0];
        let exact = metadata(0, source, physical);
        validate_row_metadata_entry(exact).unwrap();

        let mut hostile = exact;
        hostile.source_dispatch_index = 1;
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.physical_dispatch_index = 1;
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.source_segment_index = hostile.physical_segment_index.wrapping_add(1);
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.source_stage = M1StepDispatchStage::DraftPrefill;
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.source_selection.role = Qwen3ModelRole::Draft06B;
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.logical_ordinal_matches = false;
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.profile_matches = false;
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.kind_matches = false;
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.source_profile_id = Identity::new([255; 32]);
        assert_row_metadata(hostile);
        hostile = exact;
        hostile.source_program = M1PhysicalProgramV1::Rope;
        assert_row_metadata(hostile);

        assert!(matches!(
            validate_argument_ordinal(7, 2, 6),
            Err(M1PhysicalBufferBindingErrorV1::ArgumentOrdinal {
                dispatch_index: 7,
                expected: 4,
                actual: 6
            })
        ));
    }

    #[test]
    fn stale_generation_sublease_and_wrong_owner_diagnostics_remain_exact() {
        for allocation_error in [
            ServiceAllocationErrorV1::AllocationGenerationMismatch,
            ServiceAllocationErrorV1::SubleaseBindingMismatch,
            ServiceAllocationErrorV1::OwnerBindingMismatch,
        ] {
            let error = M1PhysicalBufferBindingErrorV1::WorkspaceRange {
                dispatch_index: 9,
                argument: 4,
                error: M1FullStepWorkspaceDispatchRangeError::Range {
                    workspace: M1FullStepWorkspaceRole::Target,
                    error: M1StepWorkspaceDispatchRangeError::Allocation(allocation_error),
                },
            };
            let M1PhysicalBufferBindingErrorV1::WorkspaceRange {
                dispatch_index,
                argument,
                error:
                    M1FullStepWorkspaceDispatchRangeError::Range {
                        workspace,
                        error: M1StepWorkspaceDispatchRangeError::Allocation(actual),
                    },
            } = error
            else {
                panic!("generic owner diagnostic was erased")
            };
            assert_eq!((dispatch_index, argument), (9, 4));
            assert_eq!(workspace, M1FullStepWorkspaceRole::Target);
            assert!(matches!(
                actual,
                ServiceAllocationErrorV1::AllocationGenerationMismatch
                    | ServiceAllocationErrorV1::SubleaseBindingMismatch
                    | ServiceAllocationErrorV1::OwnerBindingMismatch
            ));
        }
    }

    fn metadata(
        position: usize,
        source: &crate::M1PhysicalBufferRecipeRowV1,
        physical: &crate::M1PhysicalDispatchRecipeRowV1,
    ) -> BindingRowMetadata {
        BindingRowMetadata {
            expected_dispatch_index: u32::try_from(position).unwrap(),
            source_dispatch_index: source.dispatch_index(),
            physical_dispatch_index: physical.dispatch_index(),
            source_segment_index: source.segment_index(),
            physical_segment_index: physical.segment_index(),
            source_stage: source.stage(),
            physical_stage: physical.stage(),
            source_selection: source.selection(),
            physical_selection: physical.selection(),
            logical_ordinal_matches: source.logical_ordinal() == physical.logical_ordinal(),
            profile_matches: source.profile() == physical.profile(),
            kind_matches: source.kind() == physical.kind(),
            source_profile_id: source.profile_id(),
            physical_profile_id: physical.profile_id(),
            source_program: source.program(),
            physical_program: physical.program(),
        }
    }

    fn workspace_role_for_stage(stage: M1StepDispatchStage) -> M1FullStepWorkspaceRole {
        match stage {
            M1StepDispatchStage::TargetOnly
            | M1StepDispatchStage::TargetPrefill
            | M1StepDispatchStage::TargetVerification { .. } => M1FullStepWorkspaceRole::Target,
            M1StepDispatchStage::DraftPrefill | M1StepDispatchStage::DraftDecode { .. } => {
                M1FullStepWorkspaceRole::Draft
            }
        }
    }

    fn assert_row_metadata(metadata: BindingRowMetadata) {
        assert!(matches!(
            validate_row_metadata_entry(metadata),
            Err(M1PhysicalBufferBindingErrorV1::RowMetadata { .. })
        ));
    }
}
