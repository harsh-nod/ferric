//! Addressless explicit-buffer recipes for complete M1 physical steps.
//!
//! This layer joins the exact zero-pointer COV6 kernarg recipe to the exact
//! full-step workspace composition. Each inspected global-pointer argument is
//! assigned one semantic source, but no allocation range or native address is
//! resolved here. The resulting owner constructs no packet, publishes no
//! queue, launches no work, authenticates no contents, and proves no operator
//! refinement or hardware result.

use core::fmt;

use ferric_build::{KvCacheComponent, M1StepWorkspaceRangeRole};
use ferric_spec::{
    expected_step, Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3Operator, Qwen3PlanSelection,
    Qwen3TensorKind, QWEN3_NO_LAYER,
};

use crate::{
    AddresslessM1FullStepWorkspaceComposition, AddresslessM1PhysicalKernargRecipeV1,
    M1FullStepWorkspaceRole, M1OperationDispatchKind, M1PhysicalDispatchKindV1,
    M1PhysicalDispatchProfileV1, M1PhysicalProgramV1, M1SpeculativeDraftChoiceSubrange,
    M1StepDispatchStage, M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1,
    M1_PHYSICAL_KERNARG_RECIPE_VERSION_V1,
};

/// Addressless explicit-buffer recipe format.
pub const M1_PHYSICAL_BUFFER_RECIPE_VERSION_V1: u32 = 1;

/// Exact inspected access mode for one explicit global-pointer argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalBufferAccessV1 {
    /// The inspected kernel only reads this range.
    ReadOnly,
    /// The inspected kernel only writes this range.
    WriteOnly,
    /// The inspected kernel may read and write this range.
    ReadWrite,
}

/// Why a nonempty workspace range substitutes for an inactive zero-length ABI slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalBufferSentinelV1 {
    /// Pure `RMSNorm`'s inactive residual input.
    RmsInactiveResidual,
    /// Pure `RMSNorm`'s inactive fused-residual output.
    RmsInactiveFusedOutput,
    /// Non-speculative compact completion's inactive draft-token input.
    CompactNoDraftTokens,
}

/// Exact addressless semantic source of one inspected explicit pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalBufferSourceV1 {
    /// One complete semantic range in the segment's target or draft workspace.
    Workspace {
        /// Exact retained workspace selected for this source.
        workspace: M1FullStepWorkspaceRole,
        /// Semantic range within that workspace.
        range: M1StepWorkspaceRangeRole,
    },
    /// A nonempty workspace range supplied for an inactive zero-length slice.
    WorkspaceSentinel {
        /// Exact retained workspace containing the sentinel.
        workspace: M1FullStepWorkspaceRole,
        /// Nonempty semantic range used as the private pointer source.
        range: M1StepWorkspaceRangeRole,
        /// Exact inactive ABI position represented by the sentinel.
        purpose: M1PhysicalBufferSentinelV1,
    },
    /// One exact target `DraftChoices [K,S]` iteration row.
    SpeculativeDraftChoices(M1SpeculativeDraftChoiceSubrange),
    /// Initial draft `TokenIds [S]` consumed by target-token assembly.
    SpeculativeDraftAnchorTokenIds {
        /// Exact draft workspace containing the anchor tokens.
        workspace: M1FullStepWorkspaceRole,
        /// Exact anchor-token workspace range.
        range: M1StepWorkspaceRangeRole,
        /// Target-verification segment immediately following the assembly row.
        verification_segment: u8,
    },
    /// Target-verification token IDs that a future in-batch materialization
    /// step must assemble from the initial token and ordered draft choices.
    SpeculativeTargetTokenIds {
        /// Exact target workspace containing the declared destination range.
        workspace: M1FullStepWorkspaceRole,
        /// Target `TokenIds` destination declared by the workspace plan.
        range: M1StepWorkspaceRangeRole,
        /// Exact number of preceding autoregressive draft iterations.
        draft_iterations: u8,
    },
    /// One exact target-owned draft-decode position or committed-context row.
    SpeculativeDraftIterationMetadata {
        /// Target workspace owning the staged iteration-major rows.
        workspace: M1FullStepWorkspaceRole,
        /// `DraftPositionIds` or `DraftContextLengths` target range.
        range: M1StepWorkspaceRangeRole,
        /// Draft segment consuming this exact row.
        draft_segment: u8,
        /// Zero-based iteration-major row index.
        iteration: u8,
    },
    /// One exact immutable model-weight tensor coordinate.
    ModelWeight {
        /// Target or draft model owning the tensor.
        role: Qwen3ModelRole,
        /// Canonical tensor kind.
        kind: Qwen3TensorKind,
        /// Exact layer or [`QWEN3_NO_LAYER`] for global tensors.
        layer: u32,
    },
    /// One exact key or value plane in a role-scoped global KV arena.
    KvCachePlane {
        /// Target or draft model owning the arena.
        role: Qwen3ModelRole,
        /// Key or value plane.
        component: KvCacheComponent,
        /// Exact transformer layer.
        layer: u32,
    },
}

/// One explicit global-pointer ordinal and its exact semantic source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1PhysicalExplicitBufferV1 {
    explicit_argument_index: usize,
    access: M1PhysicalBufferAccessV1,
    source: M1PhysicalBufferSourceV1,
}

impl M1PhysicalExplicitBufferV1 {
    /// Ordinal in the inspected explicit-argument roster.
    #[must_use]
    pub const fn explicit_argument_index(self) -> usize {
        self.explicit_argument_index
    }

    /// Exact inspected access mode.
    #[must_use]
    pub const fn access(self) -> M1PhysicalBufferAccessV1 {
        self.access
    }

    /// Exact addressless semantic source.
    #[must_use]
    pub const fn source(self) -> M1PhysicalBufferSourceV1 {
        self.source
    }
}

impl M1PhysicalBufferSourceV1 {
    /// Whether this source names a required future materialization rather than
    /// a value whose production is established by the retained inputs.
    #[must_use]
    pub const fn requires_future_materialization(self) -> bool {
        matches!(self, Self::SpeculativeTargetTokenIds { .. })
    }
}

/// One checked physical dispatch row's complete explicit-pointer roster.
#[derive(Debug, Eq, PartialEq)]
pub struct M1PhysicalBufferRecipeRowV1 {
    dispatch_index: u32,
    segment_index: u8,
    stage: M1StepDispatchStage,
    selection: Qwen3PlanSelection,
    profile: M1PhysicalDispatchProfileV1,
    program: M1PhysicalProgramV1,
    buffers: Box<[M1PhysicalExplicitBufferV1]>,
}

impl M1PhysicalBufferRecipeRowV1 {
    /// Zero-based position in the complete fixed publication shape.
    #[must_use]
    pub const fn dispatch_index(&self) -> u32 {
        self.dispatch_index
    }

    /// Zero-based full-step segment position.
    #[must_use]
    pub const fn segment_index(&self) -> u8 {
        self.segment_index
    }

    /// Exact semantic role of the containing segment.
    #[must_use]
    pub const fn stage(&self) -> M1StepDispatchStage {
        self.stage
    }

    /// Exact role, execution mode, and finite bucket.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Operation ordinal within the selected generated plan.
    #[must_use]
    pub const fn logical_ordinal(&self) -> Option<u32> {
        match self.profile {
            M1PhysicalDispatchProfileV1::Model {
                logical_ordinal, ..
            } => Some(logical_ordinal),
            M1PhysicalDispatchProfileV1::SpeculativeTokenAssembly(_) => None,
        }
    }

    /// Generated Qwen operator, absent for infrastructure rows.
    #[must_use]
    pub const fn operator(&self) -> Option<Qwen3Operator> {
        match self.profile {
            M1PhysicalDispatchProfileV1::Model { descriptor, .. } => Some(descriptor.step.operator),
            M1PhysicalDispatchProfileV1::SpeculativeTokenAssembly(_) => None,
        }
    }

    /// Exact model or infrastructure dispatch profile.
    #[must_use]
    pub const fn profile(&self) -> M1PhysicalDispatchProfileV1 {
        self.profile
    }

    /// Physical row role without assigning infrastructure a Qwen operator.
    #[must_use]
    pub const fn kind(&self) -> M1PhysicalDispatchKindV1 {
        self.profile.kind()
    }

    /// Exact canonical profile identity.
    #[must_use]
    pub const fn profile_id(&self) -> Identity {
        self.profile.profile_id()
    }

    /// Exact physical entry point selected for this row.
    #[must_use]
    pub const fn program(&self) -> M1PhysicalProgramV1 {
        self.program
    }

    /// Complete explicit global-pointer roster in inspected argument order.
    #[must_use]
    pub fn buffers(&self) -> &[M1PhysicalExplicitBufferV1] {
        &self.buffers
    }

    /// This row contains no resolved allocation or address.
    #[must_use]
    pub const fn binds_device_memory(&self) -> bool {
        false
    }

    /// Whether this row depends on a value a future in-batch materialization
    /// step must establish before packet construction is sound.
    #[must_use]
    pub fn requires_future_materialization(&self) -> bool {
        self.buffers
            .iter()
            .any(|buffer| buffer.source.requires_future_materialization())
    }
}

/// Move-only custody of every exact structural input and derived buffer row.
///
/// ```compile_fail
/// use ferric_engine::AddresslessM1PhysicalBufferRecipeV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AddresslessM1PhysicalBufferRecipeV1>();
/// ```
#[must_use = "the exact linear kernarg and workspace inputs remain retained"]
#[derive(Debug, Eq, PartialEq)]
pub struct AddresslessM1PhysicalBufferRecipeV1 {
    version: u32,
    kernargs: AddresslessM1PhysicalKernargRecipeV1,
    workspaces: AddresslessM1FullStepWorkspaceComposition,
    rows: Box<[M1PhysicalBufferRecipeRowV1]>,
}

impl AddresslessM1PhysicalBufferRecipeV1 {
    /// Buffer-recipe format version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Exact retained zero-pointer kernarg recipe.
    #[must_use]
    pub const fn kernarg_recipe(&self) -> &AddresslessM1PhysicalKernargRecipeV1 {
        &self.kernargs
    }

    /// Exact retained full-step workspace composition.
    #[must_use = "the exact workspace composition remains retained"]
    pub const fn workspace_composition(&self) -> &AddresslessM1FullStepWorkspaceComposition {
        &self.workspaces
    }

    /// Complete buffer recipes in global dispatch order.
    #[must_use]
    pub fn rows(&self) -> &[M1PhysicalBufferRecipeRowV1] {
        &self.rows
    }

    /// Recovers both exact linear inputs and discards derived copyable rows.
    #[must_use = "both exact structural inputs remain retained"]
    pub fn into_inputs(
        self,
    ) -> (
        AddresslessM1PhysicalKernargRecipeV1,
        AddresslessM1FullStepWorkspaceComposition,
    ) {
        (self.kernargs, self.workspaces)
    }

    /// Recovers both exact linear inputs and every derived semantic row.
    #[must_use = "all exact structural inputs and derived rows remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        AddresslessM1PhysicalKernargRecipeV1,
        AddresslessM1FullStepWorkspaceComposition,
        Box<[M1PhysicalBufferRecipeRowV1]>,
    ) {
        (self.kernargs, self.workspaces, self.rows)
    }

    /// This recipe resolves no allocation range or native address.
    #[must_use]
    pub const fn binds_device_memory(&self) -> bool {
        false
    }

    /// This recipe constructs no packet and grants no queue authority.
    #[must_use]
    pub const fn grants_packet_or_queue_authority(&self) -> bool {
        false
    }

    /// This recipe authenticates no buffer contents.
    #[must_use]
    pub const fn authenticates_contents(&self) -> bool {
        false
    }

    /// Whether any row exposes an unfulfilled structural materialization prerequisite.
    #[must_use]
    pub fn requires_future_materialization(&self) -> bool {
        self.rows
            .iter()
            .any(M1PhysicalBufferRecipeRowV1::requires_future_materialization)
    }

    /// This recipe proves no launch, completion, hardware result, or refinement.
    #[must_use]
    pub const fn proves_execution_or_refinement(&self) -> bool {
        false
    }

    pub(crate) fn revalidate(&self) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
        let expected = derive_rows(&self.kernargs, &self.workspaces)?;
        if expected != self.rows {
            return Err(M1PhysicalBufferRecipeErrorV1::RetainedRows);
        }
        Ok(())
    }

    pub(crate) fn from_parts(
        kernargs: AddresslessM1PhysicalKernargRecipeV1,
        workspaces: AddresslessM1FullStepWorkspaceComposition,
        rows: Box<[M1PhysicalBufferRecipeRowV1]>,
    ) -> Self {
        Self {
            version: M1_PHYSICAL_BUFFER_RECIPE_VERSION_V1,
            kernargs,
            workspaces,
            rows,
        }
    }
}

/// Fail-closed physical explicit-buffer derivation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalBufferRecipeErrorV1 {
    /// The retained kernarg recipe format drifted.
    KernargVersion { expected: u32, actual: u32 },
    /// The retained physical source format drifted.
    PhysicalVersion { expected: u32, actual: u32 },
    /// The kernarg and workspace inputs name different step compositions.
    CompositionIdentity,
    /// One retained row/image/dispatch count differs.
    DispatchCount {
        expected: usize,
        physical: usize,
        images: usize,
    },
    /// A physical row, image, or flattened workspace row is out of order.
    DispatchOrder { expected: u32, actual: u32 },
    /// A physical row does not match the exact flattened workspace row.
    PhysicalRow { dispatch_index: u32 },
    /// A kernarg image does not match its exact physical row.
    KernargImage { dispatch_index: u32 },
    /// The segment-to-workspace binding is missing or drifted.
    WorkspaceBinding { segment_index: u8 },
    /// A row selected a program incompatible with its exact operator and kind.
    Program {
        dispatch_index: u32,
        program: M1PhysicalProgramV1,
    },
    /// A required workspace semantic range is absent or empty.
    WorkspaceRange {
        dispatch_index: u32,
        workspace: M1FullStepWorkspaceRole,
        role: M1StepWorkspaceRangeRole,
    },
    /// A mapped model weight or KV plane has an invalid layer coordinate.
    Layer { dispatch_index: u32, layer: u32 },
    /// A derived row has the wrong number of pointer records.
    BufferCount {
        dispatch_index: u32,
        expected: usize,
        actual: usize,
    },
    /// A pointer record names the wrong inspected explicit-argument ordinal.
    ArgumentOrdinal {
        dispatch_index: u32,
        expected: usize,
        actual: usize,
    },
    /// A pointer record has the wrong inspected access mode.
    Access {
        dispatch_index: u32,
        argument: usize,
    },
    /// A pointer record names the wrong workspace/model/KV/sentinel source.
    Source {
        dispatch_index: u32,
        argument: usize,
    },
    /// The retained derived row roster differs from exact rederivation.
    RetainedRows,
}

impl fmt::Display for M1PhysicalBufferRecipeErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 physical buffer recipe rejected: {self:?}")
    }
}

impl std::error::Error for M1PhysicalBufferRecipeErrorV1 {}

/// Retry-safe rejection retaining both unchanged linear inputs.
#[must_use = "both rejected structural inputs remain recoverable"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1PhysicalBufferRecipeFailureV1 {
    error: M1PhysicalBufferRecipeErrorV1,
    kernargs: Box<AddresslessM1PhysicalKernargRecipeV1>,
    workspaces: Box<AddresslessM1FullStepWorkspaceComposition>,
}

impl M1PhysicalBufferRecipeFailureV1 {
    /// Exact fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> M1PhysicalBufferRecipeErrorV1 {
        self.error
    }

    /// Recovers the diagnostic and both exact unchanged inputs.
    #[must_use = "both exact structural inputs remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1PhysicalBufferRecipeErrorV1,
        AddresslessM1PhysicalKernargRecipeV1,
        AddresslessM1FullStepWorkspaceComposition,
    ) {
        (self.error, *self.kernargs, *self.workspaces)
    }
}

#[derive(Clone, Copy)]
struct MappingInput {
    dispatch_index: u32,
    segment_index: u8,
    stage: M1StepDispatchStage,
    selection: Qwen3PlanSelection,
    logical_ordinal: u32,
    operator: Qwen3Operator,
    layer: u32,
    kind: M1OperationDispatchKind,
    program: M1PhysicalProgramV1,
    workspace: M1FullStepWorkspaceRole,
    draft_choice_subrange: Option<M1SpeculativeDraftChoiceSubrange>,
    token_input: Option<M1PhysicalBufferSourceV1>,
}

/// Joins exact kernarg and workspace structure into an addressless buffer recipe.
///
/// On rejection, [`M1PhysicalBufferRecipeFailureV1::into_parts`] returns both
/// exact unchanged linear inputs.
///
/// # Errors
///
/// Returns a retry-safe failure for version, identity, count, order, flattened
/// row, image, segment/workspace, program, range, layer, explicit ordinal,
/// access, or semantic-source drift.
pub fn derive_m1_physical_buffer_recipe_v1(
    kernargs: AddresslessM1PhysicalKernargRecipeV1,
    workspaces: AddresslessM1FullStepWorkspaceComposition,
) -> Result<AddresslessM1PhysicalBufferRecipeV1, M1PhysicalBufferRecipeFailureV1> {
    match derive_rows(&kernargs, &workspaces) {
        Ok(rows) => Ok(AddresslessM1PhysicalBufferRecipeV1 {
            version: M1_PHYSICAL_BUFFER_RECIPE_VERSION_V1,
            kernargs,
            workspaces,
            rows,
        }),
        Err(error) => Err(M1PhysicalBufferRecipeFailureV1 {
            error,
            kernargs: Box::new(kernargs),
            workspaces: Box::new(workspaces),
        }),
    }
}

fn derive_rows(
    kernargs: &AddresslessM1PhysicalKernargRecipeV1,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Result<Box<[M1PhysicalBufferRecipeRowV1]>, M1PhysicalBufferRecipeErrorV1> {
    validate_input_headers(kernargs, workspaces)?;
    let physical = kernargs.source_recipe();
    let dispatch = workspaces.dispatch_plan();
    let mut rows = Vec::with_capacity(physical.rows().len());
    let mut physical_position = 0_usize;
    let mut logical_dispatch_index = 0_u32;

    for segment in dispatch.segments() {
        let binding = workspaces
            .segment_binding(segment.segment_index())
            .filter(|binding| {
                binding.segment_index() == segment.segment_index()
                    && binding.workspace_selection() == segment.selection()
                    && workspaces
                        .workspace_plans()
                        .workspace(binding.workspace_role())
                        .is_some_and(|plan| {
                            plan.workspace_id() == binding.workspace_id()
                                && plan.selection() == binding.workspace_selection()
                        })
            })
            .ok_or(M1PhysicalBufferRecipeErrorV1::WorkspaceBinding {
                segment_index: segment.segment_index(),
            })?;

        if let M1StepDispatchStage::TargetVerification { draft_iterations } = segment.stage() {
            let (physical_row, image, dispatch_index) =
                physical_entry(physical.rows(), kernargs.images(), physical_position)?;
            validate_physical_entry(physical_row, image, segment, dispatch_index)?;
            let profile = match physical_row.profile() {
                M1PhysicalDispatchProfileV1::SpeculativeTokenAssembly(profile) => profile,
                M1PhysicalDispatchProfileV1::Model { .. } => {
                    return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow { dispatch_index });
                }
            };
            if physical_row.program() != M1PhysicalProgramV1::SpeculativeTokenAssembly
                || profile.speculative_k() != u32::from(draft_iterations)
                || segment
                    .selection()
                    .bucket
                    .dimensions(segment.selection().role, segment.selection().mode)
                    .is_none_or(|dimensions| {
                        dimensions.sequences != profile.sequences()
                            || dimensions.active_tokens != profile.speculative_k() + 1
                    })
            {
                return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow { dispatch_index });
            }
            let buffers = assembly_buffers(segment.segment_index());
            let row = M1PhysicalBufferRecipeRowV1 {
                dispatch_index,
                segment_index: segment.segment_index(),
                stage: segment.stage(),
                selection: segment.selection(),
                profile: physical_row.profile(),
                program: physical_row.program(),
                buffers,
            };
            validate_assembly_buffer_row(&row, workspaces)?;
            rows.push(row);
            physical_position = physical_position.checked_add(1).ok_or(
                M1PhysicalBufferRecipeErrorV1::DispatchCount {
                    expected: physical.rows().len(),
                    physical: physical.rows().len(),
                    images: kernargs.images().len(),
                },
            )?;
        }

        for logical in segment.rows() {
            let (physical_row, image, dispatch_index) =
                physical_entry(physical.rows(), kernargs.images(), physical_position)?;
            validate_physical_entry(physical_row, image, segment, dispatch_index)?;
            let expected_logical_dispatch_index = segment
                .dispatch_start()
                .checked_add(logical.dispatch_index())
                .ok_or(M1PhysicalBufferRecipeErrorV1::PhysicalRow { dispatch_index })?;
            let (logical_ordinal, descriptor, kind, profile_id) = match physical_row.profile() {
                M1PhysicalDispatchProfileV1::Model {
                    logical_ordinal,
                    descriptor,
                    kind,
                    profile_id,
                } => (logical_ordinal, descriptor, kind, profile_id),
                M1PhysicalDispatchProfileV1::SpeculativeTokenAssembly(_) => {
                    return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow { dispatch_index });
                }
            };
            if expected_logical_dispatch_index != logical_dispatch_index
                || logical.logical_ordinal() != logical_ordinal
                || logical.profile() != &descriptor
                || logical.kind() != kind
                || logical.operation().profile_id() != profile_id
            {
                return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow { dispatch_index });
            }
            let input = MappingInput {
                dispatch_index,
                segment_index: segment.segment_index(),
                stage: segment.stage(),
                selection: segment.selection(),
                logical_ordinal,
                operator: descriptor.step.operator,
                layer: descriptor.step.layer,
                kind,
                program: physical_row.program(),
                workspace: binding.workspace_role(),
                draft_choice_subrange: binding.draft_choice_subrange(),
                token_input: match segment.stage() {
                    M1StepDispatchStage::DraftDecode { iteration } if iteration > 0 => workspaces
                        .segment_binding(iteration - 1)
                        .and_then(crate::M1FullStepWorkspaceSegmentBinding::draft_choice_subrange)
                        .map(M1PhysicalBufferSourceV1::SpeculativeDraftChoices),
                    _ => None,
                },
            };
            validate_mapping_input(input)?;
            validate_speculative_metadata_binding(input, workspaces)?;
            let buffers = expected_buffers(input, workspaces)?;
            let row = M1PhysicalBufferRecipeRowV1 {
                dispatch_index,
                segment_index: input.segment_index,
                stage: input.stage,
                selection: input.selection,
                profile: physical_row.profile(),
                program: input.program,
                buffers,
            };
            validate_buffer_row(&row, input, workspaces)?;
            rows.push(row);
            physical_position = physical_position.checked_add(1).ok_or(
                M1PhysicalBufferRecipeErrorV1::DispatchCount {
                    expected: physical.rows().len(),
                    physical: physical.rows().len(),
                    images: kernargs.images().len(),
                },
            )?;
            logical_dispatch_index = logical_dispatch_index
                .checked_add(1)
                .ok_or(M1PhysicalBufferRecipeErrorV1::PhysicalRow { dispatch_index })?;
        }
    }
    if physical_position != physical.rows().len()
        || logical_dispatch_index != dispatch.dispatch_count()
    {
        return Err(M1PhysicalBufferRecipeErrorV1::DispatchCount {
            expected: physical_position,
            physical: physical.rows().len(),
            images: kernargs.images().len(),
        });
    }
    validate_recipe_order(&rows)?;
    Ok(rows.into_boxed_slice())
}

fn physical_entry<'a>(
    physical_rows: &'a [crate::M1PhysicalDispatchRecipeRowV1],
    images: &'a [crate::M1PhysicalKernargImageV1],
    position: usize,
) -> Result<
    (
        &'a crate::M1PhysicalDispatchRecipeRowV1,
        &'a crate::M1PhysicalKernargImageV1,
        u32,
    ),
    M1PhysicalBufferRecipeErrorV1,
> {
    let dispatch_index =
        u32::try_from(position).map_err(|_| M1PhysicalBufferRecipeErrorV1::DispatchCount {
            expected: physical_rows.len(),
            physical: physical_rows.len(),
            images: images.len(),
        })?;
    let physical =
        physical_rows
            .get(position)
            .ok_or(M1PhysicalBufferRecipeErrorV1::DispatchCount {
                expected: position + 1,
                physical: physical_rows.len(),
                images: images.len(),
            })?;
    let image = images
        .get(position)
        .ok_or(M1PhysicalBufferRecipeErrorV1::DispatchCount {
            expected: position + 1,
            physical: physical_rows.len(),
            images: images.len(),
        })?;
    Ok((physical, image, dispatch_index))
}

fn validate_physical_entry(
    physical: &crate::M1PhysicalDispatchRecipeRowV1,
    image: &crate::M1PhysicalKernargImageV1,
    segment: &crate::M1StepDispatchSegment,
    dispatch_index: u32,
) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    if physical.dispatch_index() != dispatch_index {
        return Err(M1PhysicalBufferRecipeErrorV1::DispatchOrder {
            expected: dispatch_index,
            actual: physical.dispatch_index(),
        });
    }
    if physical.segment_index() != segment.segment_index()
        || physical.stage() != segment.stage()
        || physical.selection() != segment.selection()
    {
        return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow { dispatch_index });
    }
    if image.dispatch_index() != dispatch_index {
        return Err(M1PhysicalBufferRecipeErrorV1::DispatchOrder {
            expected: dispatch_index,
            actual: image.dispatch_index(),
        });
    }
    if image.selection() != physical.selection()
        || image.profile_id() != physical.profile_id()
        || image.program() != physical.program()
        || image.bytes().len() != usize::try_from(physical.kernarg_bytes()).unwrap_or(0)
    {
        return Err(M1PhysicalBufferRecipeErrorV1::KernargImage { dispatch_index });
    }
    Ok(())
}

fn validate_recipe_order(
    rows: &[M1PhysicalBufferRecipeRowV1],
) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    for (position, row) in rows.iter().enumerate() {
        let expected =
            u32::try_from(position).map_err(|_| M1PhysicalBufferRecipeErrorV1::DispatchCount {
                expected: rows.len(),
                physical: rows.len(),
                images: rows.len(),
            })?;
        if row.dispatch_index != expected {
            return Err(M1PhysicalBufferRecipeErrorV1::DispatchOrder {
                expected,
                actual: row.dispatch_index,
            });
        }
    }
    Ok(())
}

fn validate_input_headers(
    kernargs: &AddresslessM1PhysicalKernargRecipeV1,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    if kernargs.version() != M1_PHYSICAL_KERNARG_RECIPE_VERSION_V1 {
        return Err(M1PhysicalBufferRecipeErrorV1::KernargVersion {
            expected: M1_PHYSICAL_KERNARG_RECIPE_VERSION_V1,
            actual: kernargs.version(),
        });
    }
    let physical = kernargs.source_recipe();
    if physical.version() != M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1 {
        return Err(M1PhysicalBufferRecipeErrorV1::PhysicalVersion {
            expected: M1_PHYSICAL_DISPATCH_RECIPE_VERSION_V1,
            actual: physical.version(),
        });
    }
    if physical.composition_id() != workspaces.dispatch_plan().composition_id() {
        return Err(M1PhysicalBufferRecipeErrorV1::CompositionIdentity);
    }
    let infrastructure = workspaces
        .dispatch_plan()
        .segments()
        .iter()
        .filter(|segment| {
            matches!(
                segment.stage(),
                M1StepDispatchStage::TargetVerification { .. }
            )
        })
        .count();
    let expected = usize::try_from(workspaces.dispatch_plan().dispatch_count())
        .unwrap_or(usize::MAX)
        .saturating_add(infrastructure);
    if expected != physical.rows().len()
        || usize::try_from(physical.dispatch_count()).ok() != Some(physical.rows().len())
        || physical.rows().len() != kernargs.images().len()
    {
        return Err(M1PhysicalBufferRecipeErrorV1::DispatchCount {
            expected,
            physical: physical.rows().len(),
            images: kernargs.images().len(),
        });
    }
    Ok(())
}

fn validate_mapping_input(input: MappingInput) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    let expected = expected_step(
        input.selection.role,
        input.selection.mode,
        input.selection.bucket,
        input.logical_ordinal,
    )
    .ok_or(M1PhysicalBufferRecipeErrorV1::PhysicalRow {
        dispatch_index: input.dispatch_index,
    })?;
    if expected.operator != input.operator || expected.layer != input.layer {
        return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow {
            dispatch_index: input.dispatch_index,
        });
    }
    match (input.stage, input.token_input) {
        (
            M1StepDispatchStage::DraftDecode { iteration },
            Some(M1PhysicalBufferSourceV1::SpeculativeDraftChoices(prior)),
        ) if iteration > 0
            && prior.producer_segment() == iteration - 1
            && prior.iteration() == iteration - 1 => {}
        (
            M1StepDispatchStage::DraftDecode { iteration: 0 }
            | M1StepDispatchStage::TargetOnly
            | M1StepDispatchStage::DraftPrefill
            | M1StepDispatchStage::TargetPrefill
            | M1StepDispatchStage::TargetVerification { .. },
            None,
        ) => {}
        _ => {
            return Err(M1PhysicalBufferRecipeErrorV1::WorkspaceBinding {
                segment_index: input.segment_index,
            });
        }
    }
    let program_matches = match input.operator {
        Qwen3Operator::TokenEmbedding => input.program == M1PhysicalProgramV1::TokenEmbedding,
        Qwen3Operator::QueryProjection
        | Qwen3Operator::KeyProjection
        | Qwen3Operator::ValueProjection
        | Qwen3Operator::AttentionOutputResidual
        | Qwen3Operator::GateProjection
        | Qwen3Operator::UpProjection
        | Qwen3Operator::DownResidual
        | Qwen3Operator::LogitsProjection => matches!(
            input.program,
            M1PhysicalProgramV1::GemmReference | M1PhysicalProgramV1::GemmVectorized
        ),
        Qwen3Operator::InputRmsNorm
        | Qwen3Operator::QueryRmsNorm
        | Qwen3Operator::KeyRmsNorm
        | Qwen3Operator::PostAttentionRmsNorm
        | Qwen3Operator::FinalRmsNorm => input.program == M1PhysicalProgramV1::RmsNorm,
        Qwen3Operator::Rope => input.program == M1PhysicalProgramV1::Rope,
        Qwen3Operator::KvWrite => input.program == M1PhysicalProgramV1::PagedKvWrite,
        Qwen3Operator::Attention => match input.selection.mode {
            Qwen3ExecutionMode::Prefill => input.program == M1PhysicalProgramV1::GqaPrefill,
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                input.program == M1PhysicalProgramV1::PagedGqaDecode
            }
        },
        Qwen3Operator::SwiGlu => input.program == M1PhysicalProgramV1::SwiGlu,
        Qwen3Operator::ArgmaxCompactCompletion => match input.kind {
            M1OperationDispatchKind::K7Argmax => input.program == M1PhysicalProgramV1::LogitsArgmax,
            M1OperationDispatchKind::K7Compact => {
                input.program == M1PhysicalProgramV1::LogitsCompact
            }
            M1OperationDispatchKind::WholeOperation => false,
        },
    };
    if !program_matches {
        return Err(M1PhysicalBufferRecipeErrorV1::Program {
            dispatch_index: input.dispatch_index,
            program: input.program,
        });
    }
    Ok(())
}

fn validate_speculative_metadata_binding(
    input: MappingInput,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    let M1StepDispatchStage::DraftDecode { iteration } = input.stage else {
        return Ok(());
    };
    if iteration == 0 {
        return Ok(());
    }
    let binding = workspaces.segment_binding(input.segment_index).ok_or(
        M1PhysicalBufferRecipeErrorV1::WorkspaceBinding {
            segment_index: input.segment_index,
        },
    )?;
    for (role, metadata) in [
        (
            M1StepWorkspaceRangeRole::DraftPositionIds,
            binding.draft_position_ids_subrange(),
        ),
        (
            M1StepWorkspaceRangeRole::DraftContextLengths,
            binding.draft_context_lengths_subrange(),
        ),
    ] {
        let valid = metadata.is_some_and(|metadata| {
            metadata.draft_segment() == input.segment_index
                && metadata.iteration() == iteration
                && metadata.range().role() == role
                && metadata.target_workspace_id()
                    == workspaces.workspace_plans().target().workspace_id()
                && metadata.target_workspace_selection()
                    == workspaces.workspace_plans().target().selection()
                && metadata.target_allocation_id()
                    == workspaces
                        .workspace_plans()
                        .target()
                        .allocation()
                        .allocation_id()
        });
        if !valid {
            return Err(M1PhysicalBufferRecipeErrorV1::WorkspaceRange {
                dispatch_index: input.dispatch_index,
                workspace: M1FullStepWorkspaceRole::Target,
                role,
            });
        }
    }
    Ok(())
}

fn buffer(
    explicit_argument_index: usize,
    access: M1PhysicalBufferAccessV1,
    source: M1PhysicalBufferSourceV1,
) -> M1PhysicalExplicitBufferV1 {
    M1PhysicalExplicitBufferV1 {
        explicit_argument_index,
        access,
        source,
    }
}

fn workspace(
    input: MappingInput,
    range: M1StepWorkspaceRangeRole,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> M1PhysicalBufferSourceV1 {
    if let M1StepDispatchStage::DraftDecode { iteration } = input.stage {
        if iteration > 0
            && matches!(
                range,
                M1StepWorkspaceRangeRole::PositionIds | M1StepWorkspaceRangeRole::ContextLengths
            )
        {
            let metadata = workspaces
                .segment_binding(input.segment_index)
                .and_then(|binding| match range {
                    M1StepWorkspaceRangeRole::PositionIds => binding.draft_position_ids_subrange(),
                    M1StepWorkspaceRangeRole::ContextLengths => {
                        binding.draft_context_lengths_subrange()
                    }
                    _ => None,
                });
            if let Some(metadata) = metadata.filter(|metadata| {
                metadata.iteration() == iteration && metadata.draft_segment() == input.segment_index
            }) {
                return M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
                    workspace: M1FullStepWorkspaceRole::Target,
                    range: metadata.range().role(),
                    draft_segment: metadata.draft_segment(),
                    iteration: metadata.iteration(),
                };
            }
        }
    }
    M1PhysicalBufferSourceV1::Workspace {
        workspace: input.workspace,
        range,
    }
}

fn sentinel(
    input: MappingInput,
    range: M1StepWorkspaceRangeRole,
    purpose: M1PhysicalBufferSentinelV1,
) -> M1PhysicalBufferSourceV1 {
    M1PhysicalBufferSourceV1::WorkspaceSentinel {
        workspace: input.workspace,
        range,
        purpose,
    }
}

fn weight(input: MappingInput, kind: Qwen3TensorKind, layer: u32) -> M1PhysicalBufferSourceV1 {
    M1PhysicalBufferSourceV1::ModelWeight {
        role: input.selection.role,
        kind,
        layer,
    }
}

fn kv(input: MappingInput, component: KvCacheComponent) -> M1PhysicalBufferSourceV1 {
    M1PhysicalBufferSourceV1::KvCachePlane {
        role: input.selection.role,
        component,
        layer: input.layer,
    }
}

fn assembly_buffers(verification_segment: u8) -> Box<[M1PhysicalExplicitBufferV1]> {
    use M1PhysicalBufferAccessV1::{ReadOnly, WriteOnly};
    use M1StepWorkspaceRangeRole as W;

    vec![
        buffer(
            0,
            ReadOnly,
            M1PhysicalBufferSourceV1::SpeculativeDraftAnchorTokenIds {
                workspace: M1FullStepWorkspaceRole::Draft,
                range: W::TokenIds,
                verification_segment,
            },
        ),
        buffer(
            2,
            ReadOnly,
            M1PhysicalBufferSourceV1::Workspace {
                workspace: M1FullStepWorkspaceRole::Target,
                range: W::DraftChoices,
            },
        ),
        buffer(
            4,
            WriteOnly,
            M1PhysicalBufferSourceV1::Workspace {
                workspace: M1FullStepWorkspaceRole::Target,
                range: W::TokenIds,
            },
        ),
    ]
    .into_boxed_slice()
}

fn expected_buffers(
    input: MappingInput,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Result<Box<[M1PhysicalExplicitBufferV1]>, M1PhysicalBufferRecipeErrorV1> {
    use M1PhysicalBufferAccessV1::{ReadOnly, WriteOnly};
    use M1StepWorkspaceRangeRole as W;
    use Qwen3Operator as O;
    use Qwen3TensorKind as T;

    let buffers = match input.operator {
        O::TokenEmbedding => vec![
            buffer(
                0,
                ReadOnly,
                input
                    .token_input
                    .unwrap_or_else(|| workspace(input, W::TokenIds, workspaces)),
            ),
            buffer(
                2,
                ReadOnly,
                weight(input, T::TokenEmbedding, QWEN3_NO_LAYER),
            ),
            buffer(
                4,
                WriteOnly,
                workspace(input, W::ResidualHidden, workspaces),
            ),
        ],
        O::QueryProjection => gemm_buffers(
            input,
            W::NormalizedHidden,
            T::QueryProjection,
            W::Query,
            workspaces,
        ),
        O::KeyProjection => gemm_buffers(
            input,
            W::NormalizedHidden,
            T::KeyProjection,
            W::Key,
            workspaces,
        ),
        O::ValueProjection => gemm_buffers(
            input,
            W::NormalizedHidden,
            T::ValueProjection,
            W::Value,
            workspaces,
        ),
        O::AttentionOutputResidual => gemm_buffers(
            input,
            W::AttentionOutput,
            T::OutputProjection,
            W::ResidualHidden,
            workspaces,
        ),
        O::GateProjection => gemm_buffers(
            input,
            W::PostAttentionNormalized,
            T::GateProjection,
            W::Gate,
            workspaces,
        ),
        O::UpProjection => gemm_buffers(
            input,
            W::PostAttentionNormalized,
            T::UpProjection,
            W::Up,
            workspaces,
        ),
        O::DownResidual => gemm_buffers(
            input,
            W::Activated,
            T::DownProjection,
            W::ResidualHidden,
            workspaces,
        ),
        O::LogitsProjection => gemm_buffers(
            input,
            W::FinalNormalized,
            T::LanguageModelHead,
            W::Logits,
            workspaces,
        ),
        O::InputRmsNorm
        | O::QueryRmsNorm
        | O::KeyRmsNorm
        | O::PostAttentionRmsNorm
        | O::FinalRmsNorm => {
            let (input_range, weight_kind, output_range, weight_layer) = match input.operator {
                O::InputRmsNorm => (
                    W::ResidualHidden,
                    T::InputLayerNorm,
                    W::NormalizedHidden,
                    input.layer,
                ),
                O::QueryRmsNorm => (W::Query, T::QueryNorm, W::NormalizedQuery, input.layer),
                O::KeyRmsNorm => (W::Key, T::KeyNorm, W::NormalizedKey, input.layer),
                O::PostAttentionRmsNorm => (
                    W::ResidualHidden,
                    T::PostAttentionLayerNorm,
                    W::PostAttentionNormalized,
                    input.layer,
                ),
                O::FinalRmsNorm => (
                    W::ResidualHidden,
                    T::FinalNorm,
                    W::FinalNormalized,
                    QWEN3_NO_LAYER,
                ),
                _ => unreachable!(),
            };
            vec![
                buffer(0, ReadOnly, workspace(input, input_range, workspaces)),
                buffer(
                    2,
                    ReadOnly,
                    sentinel(
                        input,
                        W::TokenIds,
                        M1PhysicalBufferSentinelV1::RmsInactiveResidual,
                    ),
                ),
                buffer(4, ReadOnly, weight(input, weight_kind, weight_layer)),
                buffer(
                    6,
                    WriteOnly,
                    sentinel(
                        input,
                        W::PositionIds,
                        M1PhysicalBufferSentinelV1::RmsInactiveFusedOutput,
                    ),
                ),
                buffer(8, WriteOnly, workspace(input, output_range, workspaces)),
            ]
        }
        O::Rope => vec![
            buffer(
                0,
                ReadOnly,
                workspace(input, W::NormalizedQuery, workspaces),
            ),
            buffer(2, ReadOnly, workspace(input, W::NormalizedKey, workspaces)),
            buffer(4, ReadOnly, workspace(input, W::PositionIds, workspaces)),
            buffer(6, ReadOnly, workspace(input, W::RopeCosTable, workspaces)),
            buffer(8, ReadOnly, workspace(input, W::RopeSinTable, workspaces)),
            buffer(10, WriteOnly, workspace(input, W::RotatedQuery, workspaces)),
            buffer(12, WriteOnly, workspace(input, W::RotatedKey, workspaces)),
        ],
        O::KvWrite => vec![
            buffer(0, ReadOnly, workspace(input, W::RotatedKey, workspaces)),
            buffer(2, ReadOnly, workspace(input, W::Value, workspaces)),
            buffer(4, ReadOnly, workspace(input, W::ContextLengths, workspaces)),
            buffer(6, ReadOnly, workspace(input, W::KvPageIndices, workspaces)),
            buffer(8, WriteOnly, kv(input, KvCacheComponent::Key)),
            buffer(10, WriteOnly, kv(input, KvCacheComponent::Value)),
        ],
        O::Attention => match input.selection.mode {
            Qwen3ExecutionMode::Prefill => vec![
                buffer(0, ReadOnly, workspace(input, W::RotatedQuery, workspaces)),
                buffer(2, ReadOnly, kv(input, KvCacheComponent::Key)),
                buffer(4, ReadOnly, kv(input, KvCacheComponent::Value)),
                buffer(6, ReadOnly, workspace(input, W::KvPageIndices, workspaces)),
                buffer(
                    8,
                    WriteOnly,
                    workspace(input, W::AttentionOutput, workspaces),
                ),
            ],
            Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => vec![
                buffer(0, ReadOnly, workspace(input, W::RotatedQuery, workspaces)),
                buffer(2, ReadOnly, kv(input, KvCacheComponent::Key)),
                buffer(4, ReadOnly, kv(input, KvCacheComponent::Value)),
                buffer(6, ReadOnly, workspace(input, W::KvPageIndices, workspaces)),
                buffer(8, ReadOnly, workspace(input, W::ContextLengths, workspaces)),
                buffer(
                    10,
                    WriteOnly,
                    workspace(input, W::AttentionOutput, workspaces),
                ),
            ],
        },
        O::SwiGlu => vec![
            buffer(0, ReadOnly, workspace(input, W::Gate, workspaces)),
            buffer(2, ReadOnly, workspace(input, W::Up, workspaces)),
            buffer(4, WriteOnly, workspace(input, W::Activated, workspaces)),
        ],
        O::ArgmaxCompactCompletion => match input.kind {
            M1OperationDispatchKind::K7Argmax => {
                let output = match input.stage {
                    M1StepDispatchStage::DraftDecode { iteration } => {
                        let subrange = input.draft_choice_subrange.ok_or(
                            M1PhysicalBufferRecipeErrorV1::WorkspaceBinding {
                                segment_index: input.segment_index,
                            },
                        )?;
                        if subrange.producer_segment() != input.segment_index
                            || subrange.iteration() != iteration
                        {
                            return Err(M1PhysicalBufferRecipeErrorV1::WorkspaceBinding {
                                segment_index: input.segment_index,
                            });
                        }
                        M1PhysicalBufferSourceV1::SpeculativeDraftChoices(subrange)
                    }
                    _ => workspace(input, W::Choices, workspaces),
                };
                vec![
                    buffer(0, ReadOnly, workspace(input, W::Logits, workspaces)),
                    buffer(2, WriteOnly, output),
                ]
            }
            M1OperationDispatchKind::K7Compact => {
                let draft = if input.selection.mode == Qwen3ExecutionMode::Speculative {
                    workspace(input, W::DraftChoices, workspaces)
                } else {
                    sentinel(
                        input,
                        W::PositionIds,
                        M1PhysicalBufferSentinelV1::CompactNoDraftTokens,
                    )
                };
                vec![
                    buffer(0, ReadOnly, workspace(input, W::Choices, workspaces)),
                    buffer(2, ReadOnly, draft),
                    buffer(4, ReadOnly, workspace(input, W::ActiveLengths, workspaces)),
                    buffer(6, ReadOnly, workspace(input, W::RequestSlots, workspaces)),
                    buffer(
                        8,
                        ReadOnly,
                        workspace(input, W::RequestGenerations, workspaces),
                    ),
                    buffer(
                        10,
                        ReadOnly,
                        workspace(input, W::CompletionEpochs, workspaces),
                    ),
                    buffer(
                        12,
                        ReadOnly,
                        workspace(input, W::PlanIdentities, workspaces),
                    ),
                    buffer(
                        14,
                        WriteOnly,
                        workspace(input, W::CompactCompletionRecords, workspaces),
                    ),
                ]
            }
            M1OperationDispatchKind::WholeOperation => {
                return Err(M1PhysicalBufferRecipeErrorV1::Program {
                    dispatch_index: input.dispatch_index,
                    program: input.program,
                });
            }
        },
    };
    Ok(buffers.into_boxed_slice())
}

fn gemm_buffers(
    input: MappingInput,
    input_range: M1StepWorkspaceRangeRole,
    weight_kind: Qwen3TensorKind,
    output_range: M1StepWorkspaceRangeRole,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Vec<M1PhysicalExplicitBufferV1> {
    use M1PhysicalBufferAccessV1::{ReadOnly, ReadWrite};
    let layer = if weight_kind == Qwen3TensorKind::LanguageModelHead {
        QWEN3_NO_LAYER
    } else {
        input.layer
    };
    vec![
        buffer(0, ReadOnly, workspace(input, input_range, workspaces)),
        buffer(2, ReadOnly, weight(input, weight_kind, layer)),
        buffer(4, ReadWrite, workspace(input, output_range, workspaces)),
    ]
}

fn validate_buffer_row(
    row: &M1PhysicalBufferRecipeRowV1,
    input: MappingInput,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    validate_speculative_metadata_binding(input, workspaces)?;
    if row.dispatch_index != input.dispatch_index
        || row.segment_index != input.segment_index
        || row.stage != input.stage
        || row.selection != input.selection
        || row.logical_ordinal() != Some(input.logical_ordinal)
        || row.operator() != Some(input.operator)
        || row.kind() != M1PhysicalDispatchKindV1::Model(input.kind)
        || row.program != input.program
    {
        return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow {
            dispatch_index: input.dispatch_index,
        });
    }
    let expected = expected_buffers(input, workspaces)?;
    validate_exact_buffer_roster(row, &expected, workspaces)
}

fn validate_assembly_buffer_row(
    row: &M1PhysicalBufferRecipeRowV1,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    let M1StepDispatchStage::TargetVerification { .. } = row.stage else {
        return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow {
            dispatch_index: row.dispatch_index,
        });
    };
    let M1PhysicalDispatchProfileV1::SpeculativeTokenAssembly(profile) = row.profile else {
        return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow {
            dispatch_index: row.dispatch_index,
        });
    };
    let extents = profile.storage_extents();
    let draft_anchor_bytes = extents[0].checked_mul(4);
    let draft_choices_bytes = extents[1].checked_mul(4);
    let target_token_bytes = extents[2].checked_mul(4);
    let exact_extents = workspaces
        .workspace_plans()
        .draft()
        .and_then(|draft| draft.range(M1StepWorkspaceRangeRole::TokenIds))
        .zip(
            workspaces
                .workspace_plans()
                .target()
                .range(M1StepWorkspaceRangeRole::DraftChoices),
        )
        .zip(
            workspaces
                .workspace_plans()
                .target()
                .range(M1StepWorkspaceRangeRole::TokenIds),
        )
        .is_some_and(|((anchor, choices), target)| {
            Some(anchor.byte_len()) == draft_anchor_bytes
                && Some(choices.byte_len()) == draft_choices_bytes
                && Some(target.byte_len()) == target_token_bytes
        });
    if row.logical_ordinal().is_some()
        || row.operator().is_some()
        || row.kind() != M1PhysicalDispatchKindV1::SpeculativeTokenAssembly
        || row.program != M1PhysicalProgramV1::SpeculativeTokenAssembly
        || !exact_extents
    {
        return Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow {
            dispatch_index: row.dispatch_index,
        });
    }
    let expected = assembly_buffers(row.segment_index);
    validate_exact_buffer_roster(row, &expected, workspaces)
}

fn validate_exact_buffer_roster(
    row: &M1PhysicalBufferRecipeRowV1,
    expected: &[M1PhysicalExplicitBufferV1],
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    if row.buffers.len() != expected.len() {
        return Err(M1PhysicalBufferRecipeErrorV1::BufferCount {
            dispatch_index: row.dispatch_index,
            expected: expected.len(),
            actual: row.buffers.len(),
        });
    }
    for (actual, expected) in row.buffers.iter().zip(expected.iter()) {
        if actual.explicit_argument_index != expected.explicit_argument_index {
            return Err(M1PhysicalBufferRecipeErrorV1::ArgumentOrdinal {
                dispatch_index: row.dispatch_index,
                expected: expected.explicit_argument_index,
                actual: actual.explicit_argument_index,
            });
        }
        if actual.access != expected.access {
            return Err(M1PhysicalBufferRecipeErrorV1::Access {
                dispatch_index: row.dispatch_index,
                argument: actual.explicit_argument_index,
            });
        }
        if actual.source != expected.source {
            return Err(M1PhysicalBufferRecipeErrorV1::Source {
                dispatch_index: row.dispatch_index,
                argument: actual.explicit_argument_index,
            });
        }
        validate_source(row.dispatch_index, actual.source, workspaces)?;
    }
    Ok(())
}

fn validate_source(
    dispatch_index: u32,
    source: M1PhysicalBufferSourceV1,
    workspaces: &AddresslessM1FullStepWorkspaceComposition,
) -> Result<(), M1PhysicalBufferRecipeErrorV1> {
    match source {
        M1PhysicalBufferSourceV1::Workspace { workspace, range }
        | M1PhysicalBufferSourceV1::WorkspaceSentinel {
            workspace, range, ..
        } => {
            let valid = workspaces
                .workspace_plans()
                .workspace(workspace)
                .and_then(|plan| plan.range(range))
                .is_some_and(|range| range.byte_len() > 0);
            if !valid {
                return Err(M1PhysicalBufferRecipeErrorV1::WorkspaceRange {
                    dispatch_index,
                    workspace,
                    role: range,
                });
            }
        }
        M1PhysicalBufferSourceV1::SpeculativeDraftChoices(subrange) => {
            let target = M1FullStepWorkspaceRole::Target;
            let valid = workspaces
                .workspace_plans()
                .target()
                .range(M1StepWorkspaceRangeRole::DraftChoices)
                .is_some_and(|whole| {
                    let row = subrange.range();
                    row.role() == M1StepWorkspaceRangeRole::DraftChoices
                        && row.byte_len() > 0
                        && row.offset() >= whole.offset()
                        && row.checked_end().is_some_and(|end| {
                            whole
                                .checked_end()
                                .is_some_and(|whole_end| end <= whole_end)
                        })
                        && subrange.target_workspace_id()
                            == workspaces.workspace_plans().target().workspace_id()
                        && subrange.target_allocation_id()
                            == workspaces
                                .workspace_plans()
                                .target()
                                .allocation()
                                .allocation_id()
                });
            if !valid {
                return Err(M1PhysicalBufferRecipeErrorV1::WorkspaceRange {
                    dispatch_index,
                    workspace: target,
                    role: M1StepWorkspaceRangeRole::DraftChoices,
                });
            }
        }
        M1PhysicalBufferSourceV1::SpeculativeDraftAnchorTokenIds {
            workspace,
            range,
            verification_segment,
        } => {
            let valid = workspace == M1FullStepWorkspaceRole::Draft
                && range == M1StepWorkspaceRangeRole::TokenIds
                && workspaces
                    .dispatch_plan()
                    .segments()
                    .get(usize::from(verification_segment))
                    .is_some_and(|segment| {
                        segment.segment_index() == verification_segment
                            && matches!(
                                segment.stage(),
                                M1StepDispatchStage::TargetVerification { .. }
                            )
                    })
                && workspaces
                    .workspace_plans()
                    .draft()
                    .and_then(|plan| plan.range(range))
                    .is_some_and(|range| range.byte_len() > 0);
            if !valid {
                return Err(M1PhysicalBufferRecipeErrorV1::WorkspaceRange {
                    dispatch_index,
                    workspace,
                    role: range,
                });
            }
        }
        M1PhysicalBufferSourceV1::SpeculativeTargetTokenIds {
            workspace,
            range,
            draft_iterations,
        } => {
            let valid = workspace == M1FullStepWorkspaceRole::Target
                && range == M1StepWorkspaceRangeRole::TokenIds
                && draft_iterations > 0
                && workspaces
                    .workspace_plans()
                    .target()
                    .range(range)
                    .is_some_and(|range| range.byte_len() > 0);
            if !valid {
                return Err(M1PhysicalBufferRecipeErrorV1::WorkspaceRange {
                    dispatch_index,
                    workspace,
                    role: range,
                });
            }
        }
        M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
            workspace,
            range,
            draft_segment,
            iteration,
        } => {
            let exact = workspaces
                .segment_binding(draft_segment)
                .and_then(|binding| match range {
                    M1StepWorkspaceRangeRole::DraftPositionIds => {
                        binding.draft_position_ids_subrange()
                    }
                    M1StepWorkspaceRangeRole::DraftContextLengths => {
                        binding.draft_context_lengths_subrange()
                    }
                    _ => None,
                });
            let valid = workspace == M1FullStepWorkspaceRole::Target
                && iteration > 0
                && exact.is_some_and(|metadata| {
                    metadata.iteration() == iteration
                        && metadata.draft_segment() == draft_segment
                        && metadata.range().role() == range
                        && metadata.target_workspace_id()
                            == workspaces.workspace_plans().target().workspace_id()
                        && metadata.target_workspace_selection()
                            == workspaces.workspace_plans().target().selection()
                        && metadata.target_allocation_id()
                            == workspaces
                                .workspace_plans()
                                .target()
                                .allocation()
                                .allocation_id()
                });
            if !valid {
                return Err(M1PhysicalBufferRecipeErrorV1::WorkspaceRange {
                    dispatch_index,
                    workspace,
                    role: range,
                });
            }
        }
        M1PhysicalBufferSourceV1::ModelWeight { role, kind, layer } => {
            if !valid_weight_layer(role, kind, layer) {
                return Err(M1PhysicalBufferRecipeErrorV1::Layer {
                    dispatch_index,
                    layer,
                });
            }
        }
        M1PhysicalBufferSourceV1::KvCachePlane { role, layer, .. } => {
            if layer >= role.layers() {
                return Err(M1PhysicalBufferRecipeErrorV1::Layer {
                    dispatch_index,
                    layer,
                });
            }
        }
    }
    Ok(())
}

const fn valid_weight_layer(role: Qwen3ModelRole, kind: Qwen3TensorKind, layer: u32) -> bool {
    match kind {
        Qwen3TensorKind::LanguageModelHead
        | Qwen3TensorKind::TokenEmbedding
        | Qwen3TensorKind::FinalNorm => layer == QWEN3_NO_LAYER,
        Qwen3TensorKind::InputLayerNorm
        | Qwen3TensorKind::PostAttentionLayerNorm
        | Qwen3TensorKind::QueryNorm
        | Qwen3TensorKind::KeyNorm
        | Qwen3TensorKind::QueryProjection
        | Qwen3TensorKind::KeyProjection
        | Qwen3TensorKind::ValueProjection
        | Qwen3TensorKind::OutputProjection
        | Qwen3TensorKind::GateProjection
        | Qwen3TensorKind::UpProjection
        | Qwen3TensorKind::DownProjection => layer < role.layers(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashSet;

    use ferric_build::{
        m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
        AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace,
        DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
        M1StepWorkspaceRangeRole,
    };
    use ferric_spec::{
        expected_step, plan_step_count, Identity, Qwen3ExecutionMode, Qwen3ModelRole,
        Qwen3Operator, Qwen3PlanBucket, Qwen3PlanSelection, Qwen3TensorKind,
    };

    use super::{
        derive_m1_physical_buffer_recipe_v1, derive_rows, expected_buffers, validate_buffer_row,
        validate_mapping_input, validate_recipe_order, validate_source, M1PhysicalBufferAccessV1,
        M1PhysicalBufferRecipeErrorV1, M1PhysicalBufferRecipeRowV1, M1PhysicalBufferSentinelV1,
        M1PhysicalBufferSourceV1, MappingInput, M1_PHYSICAL_BUFFER_RECIPE_VERSION_V1,
    };
    use crate::operation_kernel_plan::tests::public_operation_kernel_plan_fixture;
    use crate::{
        compose_addressless_m1_full_step_workspaces, derive_m1_physical_dispatch_recipe_v1,
        derive_m1_physical_kernarg_recipe_v1, derive_m1_step_dispatch_plan,
        AddresslessM1FullStepWorkspaceComposition, AddresslessM1PhysicalKernargRecipeV1,
        M1FullStepWorkspaceCompositionOutcome, M1FullStepWorkspacePlans, M1FullStepWorkspaceRole,
        M1OperationDispatchKind, M1PhysicalProgramV1, M1StepDispatchIntent, M1StepDispatchStage,
    };

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    const fn target(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        selection(Qwen3ModelRole::Target8B, mode, bucket)
    }

    pub(crate) fn complete_intents() -> [M1StepDispatchIntent; 15] {
        [
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            )),
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
            )),
            M1StepDispatchIntent::TargetOnly(target(
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
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            )),
            M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            )),
        ]
    }

    fn exact_workspace_plan(
        selection: Qwen3PlanSelection,
        identity_byte: u8,
    ) -> AddresslessM1StepWorkspacePlan {
        let requirements = m1_step_workspace_requirements(selection).unwrap();
        let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
            selection,
            DeclaredM1StepWorkspaceAllocation::new(
                Identity::new([identity_byte; 32]),
                requirements.allocation_byte_len(),
                requirements.allocation_alignment(),
            ),
            requirements.ranges().to_vec().into_boxed_slice(),
        ));
        match plan_addressless_m1_step_workspace(selection, available) {
            M1StepWorkspacePlanOutcome::Planned(plan) => plan,
            M1StepWorkspacePlanOutcome::Rejected(_) => panic!("exact workspace fixture rejected"),
        }
    }

    fn draft_selection(target: Qwen3PlanSelection) -> Qwen3PlanSelection {
        let (mode, bucket) = match target.mode {
            Qwen3ExecutionMode::Prefill => (Qwen3ExecutionMode::Prefill, target.bucket),
            Qwen3ExecutionMode::Speculative => (
                Qwen3ExecutionMode::Decode,
                match target.bucket {
                    Qwen3PlanBucket::SpeculativeS8K4C8192 => Qwen3PlanBucket::DecodeS8C8192,
                    Qwen3PlanBucket::SpeculativeS1K4C8192
                    | Qwen3PlanBucket::SpeculativeS1K8C8192
                    | Qwen3PlanBucket::SpeculativeS1K16C8192 => Qwen3PlanBucket::DecodeS1C8192,
                    _ => unreachable!(),
                },
            ),
            Qwen3ExecutionMode::Decode => unreachable!(),
        };
        selection(Qwen3ModelRole::Draft06B, mode, bucket)
    }

    pub(crate) fn exact_inputs(
        intent: M1StepDispatchIntent,
        identity_byte: u8,
    ) -> (
        AddresslessM1PhysicalKernargRecipeV1,
        AddresslessM1FullStepWorkspaceComposition,
    ) {
        let operation_plan = public_operation_kernel_plan_fixture();
        let physical_step = derive_m1_step_dispatch_plan(&operation_plan, intent).unwrap();
        let physical = derive_m1_physical_dispatch_recipe_v1(&physical_step).unwrap();
        let kernargs = derive_m1_physical_kernarg_recipe_v1(physical).unwrap();

        let workspace_step = derive_m1_step_dispatch_plan(&operation_plan, intent).unwrap();
        let target_selection = intent.target_selection();
        let target_plan = exact_workspace_plan(target_selection, identity_byte);
        let plans = match intent {
            M1StepDispatchIntent::TargetOnly(_) => {
                M1FullStepWorkspacePlans::target_only(target_plan)
            }
            M1StepDispatchIntent::PairedPrefill(_) => {
                let draft =
                    exact_workspace_plan(draft_selection(target_selection), identity_byte + 1);
                M1FullStepWorkspacePlans::paired_prefill(draft, target_plan)
            }
            M1StepDispatchIntent::SpeculativeRound(_) => {
                let draft =
                    exact_workspace_plan(draft_selection(target_selection), identity_byte + 1);
                M1FullStepWorkspacePlans::speculative_round(draft, target_plan)
            }
        };
        let workspaces = match compose_addressless_m1_full_step_workspaces(workspace_step, plans) {
            M1FullStepWorkspaceCompositionOutcome::Composed(composition) => composition,
            M1FullStepWorkspaceCompositionOutcome::Rejected(failure) => {
                panic!(
                    "exact workspace composition rejected: {:?}",
                    failure.error()
                )
            }
        };
        (kernargs, workspaces)
    }

    fn expected_pointer_count(program: M1PhysicalProgramV1) -> usize {
        match program {
            M1PhysicalProgramV1::GemmReference
            | M1PhysicalProgramV1::GemmVectorized
            | M1PhysicalProgramV1::TokenEmbedding
            | M1PhysicalProgramV1::SwiGlu
            | M1PhysicalProgramV1::SpeculativeTokenAssembly => 3,
            M1PhysicalProgramV1::RmsNorm | M1PhysicalProgramV1::GqaPrefill => 5,
            M1PhysicalProgramV1::Rope => 7,
            M1PhysicalProgramV1::PagedKvWrite | M1PhysicalProgramV1::PagedGqaDecode => 6,
            M1PhysicalProgramV1::LogitsArgmax => 2,
            M1PhysicalProgramV1::LogitsCompact => 8,
        }
    }

    #[test]
    fn every_complete_intent_maps_all_twelve_programs_in_exact_row_order() {
        let mut programs = HashSet::new();
        let mut selections = Vec::new();
        for (case, intent) in complete_intents().into_iter().enumerate() {
            let (kernargs, workspaces) = exact_inputs(intent, 10 + u8::try_from(case).unwrap() * 2);
            let count = kernargs.images().len();
            let recipe = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap();
            assert_eq!(recipe.version(), M1_PHYSICAL_BUFFER_RECIPE_VERSION_V1);
            assert_eq!(recipe.rows().len(), count);
            assert!(!recipe.binds_device_memory());
            assert!(!recipe.grants_packet_or_queue_authority());
            assert!(!recipe.authenticates_contents());
            assert!(!recipe.proves_execution_or_refinement());
            for (position, row) in recipe.rows().iter().enumerate() {
                assert_eq!(row.dispatch_index(), u32::try_from(position).unwrap());
                assert_eq!(row.buffers().len(), expected_pointer_count(row.program()));
                assert!(row
                    .buffers()
                    .windows(2)
                    .all(|pair| pair[0].explicit_argument_index()
                        < pair[1].explicit_argument_index()));
                assert!(row
                    .buffers()
                    .iter()
                    .all(|buffer| buffer.explicit_argument_index() % 2 == 0));
                assert!(!row.binds_device_memory());
                programs.insert(row.program());
                if !selections.contains(&row.selection()) {
                    selections.push(row.selection());
                }
            }
        }
        assert_eq!(programs, M1PhysicalProgramV1::ALL.into_iter().collect());
        assert_eq!(selections.len(), 17);
    }

    fn all_finite_selections() -> Vec<Qwen3PlanSelection> {
        let buckets = [
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS8T128),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T512),
            (Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T2048),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192),
            (Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS32C8192),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
            ),
        ];
        [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B]
            .into_iter()
            .flat_map(|role| {
                buckets
                    .into_iter()
                    .map(move |(mode, bucket)| selection(role, mode, bucket))
            })
            .collect()
    }

    fn program_for(operator: Qwen3Operator, mode: Qwen3ExecutionMode) -> M1PhysicalProgramV1 {
        match operator {
            Qwen3Operator::TokenEmbedding => M1PhysicalProgramV1::TokenEmbedding,
            Qwen3Operator::QueryProjection
            | Qwen3Operator::KeyProjection
            | Qwen3Operator::ValueProjection
            | Qwen3Operator::AttentionOutputResidual
            | Qwen3Operator::GateProjection
            | Qwen3Operator::UpProjection
            | Qwen3Operator::DownResidual
            | Qwen3Operator::LogitsProjection => M1PhysicalProgramV1::GemmVectorized,
            Qwen3Operator::InputRmsNorm
            | Qwen3Operator::QueryRmsNorm
            | Qwen3Operator::KeyRmsNorm
            | Qwen3Operator::PostAttentionRmsNorm
            | Qwen3Operator::FinalRmsNorm => M1PhysicalProgramV1::RmsNorm,
            Qwen3Operator::Rope => M1PhysicalProgramV1::Rope,
            Qwen3Operator::KvWrite => M1PhysicalProgramV1::PagedKvWrite,
            Qwen3Operator::Attention => match mode {
                Qwen3ExecutionMode::Prefill => M1PhysicalProgramV1::GqaPrefill,
                Qwen3ExecutionMode::Decode | Qwen3ExecutionMode::Speculative => {
                    M1PhysicalProgramV1::PagedGqaDecode
                }
            },
            Qwen3Operator::SwiGlu => M1PhysicalProgramV1::SwiGlu,
            Qwen3Operator::ArgmaxCompactCompletion => M1PhysicalProgramV1::LogitsArgmax,
        }
    }

    #[test]
    fn all_twenty_two_finite_selections_have_exhaustive_operator_mappings() {
        let (_, workspaces) = exact_inputs(
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            70,
        );
        let selections = all_finite_selections();
        assert_eq!(selections.len(), 22);
        for selected in selections {
            let mut operators = Vec::new();
            for ordinal in 0..plan_step_count(selected.role) {
                let step = expected_step(selected.role, selected.mode, selected.bucket, ordinal)
                    .expect("finite selection has every generated step");
                let input = MappingInput {
                    dispatch_index: ordinal,
                    segment_index: 0,
                    stage: M1StepDispatchStage::TargetOnly,
                    selection: selected,
                    logical_ordinal: ordinal,
                    operator: step.operator,
                    layer: step.layer,
                    kind: if step.operator == Qwen3Operator::ArgmaxCompactCompletion {
                        M1OperationDispatchKind::K7Argmax
                    } else {
                        M1OperationDispatchKind::WholeOperation
                    },
                    program: program_for(step.operator, selected.mode),
                    workspace: M1FullStepWorkspaceRole::Target,
                    draft_choice_subrange: None,
                    token_input: None,
                };
                validate_mapping_input(input).unwrap();
                let buffers = expected_buffers(input, &workspaces).unwrap();
                assert_eq!(buffers.len(), expected_pointer_count(input.program));
                if !operators.contains(&step.operator) {
                    operators.push(step.operator);
                }
            }
            assert_eq!(operators.len(), 19);
        }
    }

    #[test]
    fn speculative_token_flow_and_draft_choice_rows_are_explicit() {
        let intent = M1StepDispatchIntent::SpeculativeRound(target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ));
        let (kernargs, workspaces) = exact_inputs(intent, 80);
        let recipe = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap();
        assert!(!recipe.requires_future_materialization());
        let embeddings = recipe
            .rows()
            .iter()
            .filter(|row| row.program() == M1PhysicalProgramV1::TokenEmbedding)
            .collect::<Vec<_>>();
        assert_eq!(embeddings.len(), 5);
        assert_eq!(
            embeddings[0].buffers()[0].source(),
            M1PhysicalBufferSourceV1::Workspace {
                workspace: M1FullStepWorkspaceRole::Draft,
                range: M1StepWorkspaceRangeRole::TokenIds,
            }
        );
        for iteration in 1..4_u8 {
            let source = embeddings[usize::from(iteration)].buffers()[0].source();
            let M1PhysicalBufferSourceV1::SpeculativeDraftChoices(prior) = source else {
                panic!("draft decode did not consume its prior argmax row");
            };
            assert_eq!(prior.producer_segment(), iteration - 1);
            assert_eq!(prior.iteration(), iteration - 1);

            let stage = M1StepDispatchStage::DraftDecode { iteration };
            let rope = recipe
                .rows()
                .iter()
                .find(|row| row.stage() == stage && row.program() == M1PhysicalProgramV1::Rope)
                .unwrap();
            assert_eq!(
                rope.buffers()[2].source(),
                M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
                    workspace: M1FullStepWorkspaceRole::Target,
                    range: M1StepWorkspaceRangeRole::DraftPositionIds,
                    draft_segment: iteration,
                    iteration,
                }
            );
            for (program, buffer_index) in [
                (M1PhysicalProgramV1::PagedKvWrite, 2),
                (M1PhysicalProgramV1::PagedGqaDecode, 4),
            ] {
                let row = recipe
                    .rows()
                    .iter()
                    .find(|row| row.stage() == stage && row.program() == program)
                    .unwrap();
                assert_eq!(
                    row.buffers()[buffer_index].source(),
                    M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
                        workspace: M1FullStepWorkspaceRole::Target,
                        range: M1StepWorkspaceRangeRole::DraftContextLengths,
                        draft_segment: iteration,
                        iteration,
                    }
                );
            }
        }
        let assembly = recipe
            .rows()
            .iter()
            .find(|row| row.program() == M1PhysicalProgramV1::SpeculativeTokenAssembly)
            .unwrap();
        assert_eq!(assembly.logical_ordinal(), None);
        assert_eq!(assembly.operator(), None);
        assert_eq!(
            assembly.buffers(),
            &[
                super::buffer(
                    0,
                    M1PhysicalBufferAccessV1::ReadOnly,
                    M1PhysicalBufferSourceV1::SpeculativeDraftAnchorTokenIds {
                        workspace: M1FullStepWorkspaceRole::Draft,
                        range: M1StepWorkspaceRangeRole::TokenIds,
                        verification_segment: 4,
                    },
                ),
                super::buffer(
                    2,
                    M1PhysicalBufferAccessV1::ReadOnly,
                    M1PhysicalBufferSourceV1::Workspace {
                        workspace: M1FullStepWorkspaceRole::Target,
                        range: M1StepWorkspaceRangeRole::DraftChoices,
                    },
                ),
                super::buffer(
                    4,
                    M1PhysicalBufferAccessV1::WriteOnly,
                    M1PhysicalBufferSourceV1::Workspace {
                        workspace: M1FullStepWorkspaceRole::Target,
                        range: M1StepWorkspaceRangeRole::TokenIds,
                    },
                ),
            ]
        );
        let target_source = embeddings[4].buffers()[0].source();
        assert_eq!(
            target_source,
            M1PhysicalBufferSourceV1::Workspace {
                workspace: M1FullStepWorkspaceRole::Target,
                range: M1StepWorkspaceRangeRole::TokenIds,
            }
        );
        assert!(!target_source.requires_future_materialization());
        assert!(assembly.dispatch_index() < embeddings[4].dispatch_index());
        assert_eq!(
            assembly.dispatch_index() + 1,
            embeddings[4].dispatch_index()
        );

        for row in recipe.rows().iter().filter(|row| {
            row.stage()
                == M1StepDispatchStage::DraftDecode {
                    iteration: row.segment_index(),
                }
                && row.program() == M1PhysicalProgramV1::LogitsArgmax
        }) {
            let M1PhysicalBufferSourceV1::SpeculativeDraftChoices(output) =
                row.buffers()[1].source()
            else {
                panic!("draft argmax did not target its exact preservation row");
            };
            assert_eq!(output.producer_segment(), row.segment_index());
        }
        let compact = recipe
            .rows()
            .iter()
            .find(|row| row.program() == M1PhysicalProgramV1::LogitsCompact)
            .unwrap();
        assert_eq!(
            compact.buffers()[1].source(),
            M1PhysicalBufferSourceV1::Workspace {
                workspace: M1FullStepWorkspaceRole::Target,
                range: M1StepWorkspaceRangeRole::DraftChoices,
            }
        );
    }

    #[test]
    fn all_four_speculative_counts_and_s8_k4_iteration_offsets_are_exact() {
        let buckets = [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 2_242_usize),
            (Qwen3PlanBucket::SpeculativeS8K4C8192, 2_242),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 3_938),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 7_330),
        ];
        for (case, (bucket, expected_count)) in buckets.into_iter().enumerate() {
            let intent = M1StepDispatchIntent::SpeculativeRound(target(
                Qwen3ExecutionMode::Speculative,
                bucket,
            ));
            let (kernargs, workspaces) =
                exact_inputs(intent, 130 + u8::try_from(case).unwrap() * 2);
            let recipe = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap();
            assert_eq!(recipe.rows().len(), expected_count);
            let assembly_position = recipe
                .rows()
                .iter()
                .position(|row| row.program() == M1PhysicalProgramV1::SpeculativeTokenAssembly)
                .unwrap();
            let target_embedding = recipe.rows()[assembly_position + 1].operator().unwrap();
            assert_eq!(target_embedding, Qwen3Operator::TokenEmbedding);
        }

        let intent = M1StepDispatchIntent::SpeculativeRound(target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        ));
        let (kernargs, workspaces) = exact_inputs(intent, 145);
        let recipe = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap();
        let target = recipe.workspace_composition().workspace_plans().target();
        let positions = target
            .range(M1StepWorkspaceRangeRole::DraftPositionIds)
            .unwrap();
        let contexts = target
            .range(M1StepWorkspaceRangeRole::DraftContextLengths)
            .unwrap();
        for iteration in 1..4_u8 {
            let binding = recipe
                .workspace_composition()
                .segment_binding(iteration)
                .unwrap();
            let position = binding.draft_position_ids_subrange().unwrap();
            let context = binding.draft_context_lengths_subrange().unwrap();
            assert_eq!(position.range().byte_len(), 32);
            assert_eq!(context.range().byte_len(), 32);
            assert_eq!(
                position.range().offset(),
                positions.offset() + u64::from(iteration) * 32
            );
            assert_eq!(
                context.range().offset(),
                contexts.offset() + u64::from(iteration) * 32
            );
            assert!(recipe.rows().iter().any(|row| {
                row.stage() == M1StepDispatchStage::DraftDecode { iteration }
                    && row.buffers().iter().any(|buffer| {
                        buffer.source()
                            == M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
                                workspace: M1FullStepWorkspaceRole::Target,
                                range: M1StepWorkspaceRangeRole::DraftPositionIds,
                                draft_segment: iteration,
                                iteration,
                            }
                    })
            }));
            assert!(recipe.rows().iter().any(|row| {
                row.stage() == M1StepDispatchStage::DraftDecode { iteration }
                    && row.buffers().iter().any(|buffer| {
                        buffer.source()
                            == M1PhysicalBufferSourceV1::SpeculativeDraftIterationMetadata {
                                workspace: M1FullStepWorkspaceRole::Target,
                                range: M1StepWorkspaceRangeRole::DraftContextLengths,
                                draft_segment: iteration,
                                iteration,
                            }
                    })
            }));
        }
    }

    #[test]
    fn hostile_assembly_order_profile_and_source_fail_closed() {
        let intent = M1StepDispatchIntent::SpeculativeRound(target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ));

        let (kernargs, workspaces) = exact_inputs(intent, 150);
        let recipe = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap();
        let (kernargs, workspaces, rows) = recipe.into_parts();
        let mut rows = rows.into_vec();
        let assembly = rows
            .iter()
            .position(|row| row.program == M1PhysicalProgramV1::SpeculativeTokenAssembly)
            .unwrap();
        rows.swap(assembly - 1, assembly);
        assert_eq!(
            super::AddresslessM1PhysicalBufferRecipeV1::from_parts(
                kernargs,
                workspaces,
                rows.into_boxed_slice(),
            )
            .revalidate(),
            Err(M1PhysicalBufferRecipeErrorV1::RetainedRows)
        );

        let (kernargs, workspaces) = exact_inputs(intent, 152);
        let recipe = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap();
        let (kernargs, workspaces, rows) = recipe.into_parts();
        let mut rows = rows.into_vec();
        let assembly = rows
            .iter()
            .position(|row| row.program == M1PhysicalProgramV1::SpeculativeTokenAssembly)
            .unwrap();
        rows[assembly].profile = rows[assembly + 1].profile;
        assert_eq!(
            super::AddresslessM1PhysicalBufferRecipeV1::from_parts(
                kernargs,
                workspaces,
                rows.into_boxed_slice(),
            )
            .revalidate(),
            Err(M1PhysicalBufferRecipeErrorV1::RetainedRows)
        );

        let (kernargs, workspaces) = exact_inputs(intent, 154);
        let recipe = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap();
        let (kernargs, workspaces, rows) = recipe.into_parts();
        let mut rows = rows.into_vec();
        let assembly = rows
            .iter()
            .position(|row| row.program == M1PhysicalProgramV1::SpeculativeTokenAssembly)
            .unwrap();
        rows[assembly].buffers[0].source = M1PhysicalBufferSourceV1::Workspace {
            workspace: M1FullStepWorkspaceRole::Draft,
            range: M1StepWorkspaceRangeRole::TokenIds,
        };
        assert_eq!(
            super::AddresslessM1PhysicalBufferRecipeV1::from_parts(
                kernargs,
                workspaces,
                rows.into_boxed_slice(),
            )
            .revalidate(),
            Err(M1PhysicalBufferRecipeErrorV1::RetainedRows)
        );
    }

    #[test]
    fn rms_and_non_speculative_compact_use_exact_nonempty_sentinels() {
        let intent = M1StepDispatchIntent::TargetOnly(target(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        ));
        let (kernargs, workspaces) = exact_inputs(intent, 90);
        let recipe = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap();
        assert!(!recipe.requires_future_materialization());
        for row in recipe
            .rows()
            .iter()
            .filter(|row| row.program() == M1PhysicalProgramV1::RmsNorm)
        {
            assert_eq!(
                row.buffers()[1].source(),
                M1PhysicalBufferSourceV1::WorkspaceSentinel {
                    workspace: M1FullStepWorkspaceRole::Target,
                    range: M1StepWorkspaceRangeRole::TokenIds,
                    purpose: M1PhysicalBufferSentinelV1::RmsInactiveResidual,
                }
            );
            assert_eq!(
                row.buffers()[3].source(),
                M1PhysicalBufferSourceV1::WorkspaceSentinel {
                    workspace: M1FullStepWorkspaceRole::Target,
                    range: M1StepWorkspaceRangeRole::PositionIds,
                    purpose: M1PhysicalBufferSentinelV1::RmsInactiveFusedOutput,
                }
            );
        }
        let compact = recipe
            .rows()
            .iter()
            .find(|row| row.program() == M1PhysicalProgramV1::LogitsCompact)
            .unwrap();
        assert_eq!(
            compact.buffers()[1].source(),
            M1PhysicalBufferSourceV1::WorkspaceSentinel {
                workspace: M1FullStepWorkspaceRole::Target,
                range: M1StepWorkspaceRangeRole::PositionIds,
                purpose: M1PhysicalBufferSentinelV1::CompactNoDraftTokens,
            }
        );
    }

    #[test]
    fn mismatched_compositions_fail_and_recover_both_linear_inputs() {
        let (kernargs, _) = exact_inputs(
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            )),
            100,
        );
        let (_, workspaces) = exact_inputs(
            M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
            )),
            101,
        );
        let physical_id = kernargs.source_recipe().composition_id();
        let workspace_id = workspaces.dispatch_plan().composition_id();
        let failure = derive_m1_physical_buffer_recipe_v1(kernargs, workspaces).unwrap_err();
        assert_eq!(
            failure.error(),
            M1PhysicalBufferRecipeErrorV1::CompositionIdentity
        );
        let (_, recovered_kernargs, recovered_workspaces) = failure.into_parts();
        assert_eq!(
            recovered_kernargs.source_recipe().composition_id(),
            physical_id
        );
        assert_eq!(
            recovered_workspaces.dispatch_plan().composition_id(),
            workspace_id
        );
    }

    fn rms_input() -> MappingInput {
        let selected = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let (ordinal, step) = (0..plan_step_count(selected.role))
            .find_map(|ordinal| {
                let step = expected_step(selected.role, selected.mode, selected.bucket, ordinal)?;
                (step.operator == Qwen3Operator::InputRmsNorm).then_some((ordinal, step))
            })
            .unwrap();
        MappingInput {
            dispatch_index: ordinal,
            segment_index: 0,
            stage: M1StepDispatchStage::TargetOnly,
            selection: selected,
            logical_ordinal: ordinal,
            operator: step.operator,
            layer: step.layer,
            kind: M1OperationDispatchKind::WholeOperation,
            program: M1PhysicalProgramV1::RmsNorm,
            workspace: M1FullStepWorkspaceRole::Target,
            draft_choice_subrange: None,
            token_input: None,
        }
    }

    #[test]
    fn hostile_program_ordinal_access_role_sentinel_layer_and_row_drift_fail_closed() {
        let input = rms_input();
        let (kernargs, workspaces) =
            exact_inputs(M1StepDispatchIntent::TargetOnly(input.selection), 110);
        let profile = kernargs.source_recipe().rows()
            [usize::try_from(input.dispatch_index).unwrap()]
        .profile();
        let exact_buffers = expected_buffers(input, &workspaces).unwrap();
        let mut row = M1PhysicalBufferRecipeRowV1 {
            dispatch_index: input.dispatch_index,
            segment_index: input.segment_index,
            stage: input.stage,
            selection: input.selection,
            profile,
            program: input.program,
            buffers: exact_buffers,
        };
        validate_buffer_row(&row, input, &workspaces).unwrap();

        let mut hostile_program = input;
        hostile_program.program = M1PhysicalProgramV1::Rope;
        assert!(matches!(
            validate_mapping_input(hostile_program),
            Err(M1PhysicalBufferRecipeErrorV1::Program { .. })
        ));

        row.buffers[0].explicit_argument_index = 2;
        assert!(matches!(
            validate_buffer_row(&row, input, &workspaces),
            Err(M1PhysicalBufferRecipeErrorV1::ArgumentOrdinal { .. })
        ));
        row.buffers = expected_buffers(input, &workspaces).unwrap();
        row.buffers[0].access = M1PhysicalBufferAccessV1::WriteOnly;
        assert!(matches!(
            validate_buffer_row(&row, input, &workspaces),
            Err(M1PhysicalBufferRecipeErrorV1::Access { .. })
        ));
        row.buffers = expected_buffers(input, &workspaces).unwrap();
        row.buffers[0].source = M1PhysicalBufferSourceV1::Workspace {
            workspace: M1FullStepWorkspaceRole::Draft,
            range: M1StepWorkspaceRangeRole::ResidualHidden,
        };
        assert!(matches!(
            validate_buffer_row(&row, input, &workspaces),
            Err(M1PhysicalBufferRecipeErrorV1::Source { .. })
        ));
        row.buffers = expected_buffers(input, &workspaces).unwrap();
        row.buffers[1].source = M1PhysicalBufferSourceV1::WorkspaceSentinel {
            workspace: M1FullStepWorkspaceRole::Target,
            range: M1StepWorkspaceRangeRole::PositionIds,
            purpose: M1PhysicalBufferSentinelV1::CompactNoDraftTokens,
        };
        assert!(matches!(
            validate_buffer_row(&row, input, &workspaces),
            Err(M1PhysicalBufferRecipeErrorV1::Source { .. })
        ));
        assert!(matches!(
            validate_source(
                input.dispatch_index,
                M1PhysicalBufferSourceV1::ModelWeight {
                    role: Qwen3ModelRole::Target8B,
                    kind: Qwen3TensorKind::InputLayerNorm,
                    layer: Qwen3ModelRole::Target8B.layers(),
                },
                &workspaces,
            ),
            Err(M1PhysicalBufferRecipeErrorV1::Layer { .. })
        ));
        row.buffers = expected_buffers(input, &workspaces).unwrap();
        row.dispatch_index += 1;
        assert!(matches!(
            validate_buffer_row(&row, input, &workspaces),
            Err(M1PhysicalBufferRecipeErrorV1::PhysicalRow { .. })
        ));

        let (kernargs, ordered_workspaces) =
            exact_inputs(M1StepDispatchIntent::TargetOnly(input.selection), 112);
        let mut rows = derive_rows(&kernargs, &ordered_workspaces)
            .unwrap()
            .into_vec();
        rows.swap(0, 1);
        assert!(matches!(
            validate_recipe_order(&rows),
            Err(M1PhysicalBufferRecipeErrorV1::DispatchOrder { .. })
        ));
    }
}
