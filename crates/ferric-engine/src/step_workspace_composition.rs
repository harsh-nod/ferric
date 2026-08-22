//! Addressless workspace composition for one complete M1 dispatch step.
//!
//! This layer joins an exact checked dispatch composition to the exact checked
//! workspace plans required by its intent. Speculative draft decodes reuse one
//! draft decode workspace, while each draft segment names one disjoint row of
//! the target `DraftChoices [K,S]` range where its argmax is to be preserved.
//!
//! The resulting declarations remain addressless. They grant no allocation,
//! mapping, packet, queue, launch, completion, readback, content-authentication,
//! or refinement authority.

use core::fmt;

use ferric_build::{
    AddresslessM1StepWorkspacePlan, M1StepWorkspaceRange, M1StepWorkspaceRangeRole,
};
use ferric_spec::{
    Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
};

use crate::{
    AddresslessM1StepDispatchPlan, M1StepDispatchDependency, M1StepDispatchIntent,
    M1StepDispatchStage,
};

const U32_BYTES: u64 = 4;

/// Exact workspace-input shape supplied for a complete step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1FullStepWorkspaceInputKind {
    /// One target workspace for a target-only prefill or decode.
    TargetOnly,
    /// One draft prefill workspace and one target prefill workspace.
    PairedPrefill,
    /// One reusable draft decode workspace and one target speculative workspace.
    SpeculativeRound,
}

/// Model workspace selected for one full-step dispatch segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1FullStepWorkspaceRole {
    /// The exact target workspace plan.
    Target,
    /// The exact draft workspace plan.
    Draft,
}

/// Linear workspace-plan inputs for one complete step.
///
/// Each contained plan intentionally remains non-`Clone` and is boxed only to
/// keep success and rejection values compact.
#[must_use = "the exact linear workspace plans must remain retained"]
#[derive(Debug, Eq, PartialEq)]
pub enum M1FullStepWorkspacePlans {
    /// One target plan for [`M1StepDispatchIntent::TargetOnly`].
    TargetOnly {
        /// Exact target prefill or decode workspace plan.
        target: Box<AddresslessM1StepWorkspacePlan>,
    },
    /// Draft and target prefill plans for [`M1StepDispatchIntent::PairedPrefill`].
    PairedPrefill {
        /// Exact draft prefill workspace plan.
        draft: Box<AddresslessM1StepWorkspacePlan>,
        /// Exact target prefill workspace plan.
        target: Box<AddresslessM1StepWorkspacePlan>,
    },
    /// Reusable draft decode plus target verification plans.
    SpeculativeRound {
        /// Exact draft decode workspace reused for every autoregressive iteration.
        draft_decode: Box<AddresslessM1StepWorkspacePlan>,
        /// Exact target speculative workspace containing `DraftChoices [K,S]`.
        target_speculative: Box<AddresslessM1StepWorkspacePlan>,
    },
}

impl M1FullStepWorkspacePlans {
    /// Wraps the one exact target-only plan.
    #[must_use = "the target workspace plan remains retained"]
    pub fn target_only(target: AddresslessM1StepWorkspacePlan) -> Self {
        Self::TargetOnly {
            target: Box::new(target),
        }
    }

    /// Wraps the exact draft and target prefill plans.
    #[must_use = "the draft and target workspace plans remain retained"]
    pub fn paired_prefill(
        draft: AddresslessM1StepWorkspacePlan,
        target: AddresslessM1StepWorkspacePlan,
    ) -> Self {
        Self::PairedPrefill {
            draft: Box::new(draft),
            target: Box::new(target),
        }
    }

    /// Wraps the exact reusable draft decode and target speculative plans.
    #[must_use = "the draft decode and target speculative plans remain retained"]
    pub fn speculative_round(
        draft_decode: AddresslessM1StepWorkspacePlan,
        target_speculative: AddresslessM1StepWorkspacePlan,
    ) -> Self {
        Self::SpeculativeRound {
            draft_decode: Box::new(draft_decode),
            target_speculative: Box::new(target_speculative),
        }
    }

    /// Returns the supplied finite input shape.
    #[must_use]
    pub const fn kind(&self) -> M1FullStepWorkspaceInputKind {
        match self {
            Self::TargetOnly { .. } => M1FullStepWorkspaceInputKind::TargetOnly,
            Self::PairedPrefill { .. } => M1FullStepWorkspaceInputKind::PairedPrefill,
            Self::SpeculativeRound { .. } => M1FullStepWorkspaceInputKind::SpeculativeRound,
        }
    }

    /// Returns the exact target workspace plan.
    #[must_use]
    pub fn target(&self) -> &AddresslessM1StepWorkspacePlan {
        match self {
            Self::TargetOnly { target } | Self::PairedPrefill { target, .. } => target,
            Self::SpeculativeRound {
                target_speculative, ..
            } => target_speculative,
        }
    }

    /// Returns the exact draft workspace plan when the intent requires one.
    #[must_use]
    pub fn draft(&self) -> Option<&AddresslessM1StepWorkspacePlan> {
        match self {
            Self::TargetOnly { .. } => None,
            Self::PairedPrefill { draft, .. } => Some(draft),
            Self::SpeculativeRound { draft_decode, .. } => Some(draft_decode),
        }
    }

    /// Returns the exact workspace plan selected by a segment role.
    #[must_use]
    pub fn workspace(
        &self,
        role: M1FullStepWorkspaceRole,
    ) -> Option<&AddresslessM1StepWorkspacePlan> {
        match role {
            M1FullStepWorkspaceRole::Target => Some(self.target()),
            M1FullStepWorkspaceRole::Draft => self.draft(),
        }
    }
}

/// One checked target `DraftChoices [K,S]` iteration row.
///
/// This addressless row is the declared preservation destination for the named
/// draft segment's argmax values. It does not establish that any value was
/// written, retained, consumed, or read back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeDraftChoiceSubrange {
    producer_segment: u8,
    iteration: u8,
    sequence_count: u32,
    target_workspace_id: Identity,
    target_allocation_id: Identity,
    range: M1StepWorkspaceRange,
}

impl M1SpeculativeDraftChoiceSubrange {
    /// Draft segment that produces the choices intended for this row.
    #[must_use]
    pub const fn producer_segment(self) -> u8 {
        self.producer_segment
    }

    /// Zero-based iteration-major row index in target `DraftChoices [K,S]`.
    #[must_use]
    pub const fn iteration(self) -> u8 {
        self.iteration
    }

    /// Exact sequence width `S` of this row.
    #[must_use]
    pub const fn sequence_count(self) -> u32 {
        self.sequence_count
    }

    /// Identity of the exact target speculative workspace plan.
    #[must_use]
    pub const fn target_workspace_id(self) -> Identity {
        self.target_workspace_id
    }

    /// Inert identity of the target plan's declared future allocation.
    #[must_use]
    pub const fn target_allocation_id(self) -> Identity {
        self.target_allocation_id
    }

    /// Exact checked addressless row within target `DraftChoices [K,S]`.
    #[must_use]
    pub const fn range(self) -> M1StepWorkspaceRange {
        self.range
    }

    /// Returns one sequence element's checked byte offset within the target allocation.
    #[must_use]
    pub fn sequence_byte_offset(self, sequence_index: u32) -> Option<u64> {
        if sequence_index >= self.sequence_count {
            return None;
        }
        let within_row = u64::from(sequence_index).checked_mul(U32_BYTES)?;
        let offset = self.range.offset().checked_add(within_row)?;
        match (offset.checked_add(U32_BYTES), self.range.checked_end()) {
            (Some(end), Some(range_end)) if end <= range_end => Some(offset),
            _ => None,
        }
    }

    /// This metadata carries no native address or authenticated contents.
    #[must_use]
    pub const fn authenticates_preserved_choice(self) -> bool {
        false
    }
}

/// Checked association between one dispatch segment and one workspace plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1FullStepWorkspaceSegmentBinding {
    segment_index: u8,
    workspace_role: M1FullStepWorkspaceRole,
    workspace_id: Identity,
    workspace_selection: Qwen3PlanSelection,
    draft_choice_subrange: Option<M1SpeculativeDraftChoiceSubrange>,
}

impl M1FullStepWorkspaceSegmentBinding {
    /// Zero-based dispatch-composition segment index.
    #[must_use]
    pub const fn segment_index(self) -> u8 {
        self.segment_index
    }

    /// Whether the segment uses the retained draft or target workspace plan.
    #[must_use]
    pub const fn workspace_role(self) -> M1FullStepWorkspaceRole {
        self.workspace_role
    }

    /// Identity of the exact selected addressless workspace plan.
    #[must_use]
    pub const fn workspace_id(self) -> Identity {
        self.workspace_id
    }

    /// Exact role, mode, and bucket of the selected workspace plan.
    #[must_use]
    pub const fn workspace_selection(self) -> Qwen3PlanSelection {
        self.workspace_selection
    }

    /// Target `DraftChoices` row for this speculative draft iteration.
    ///
    /// Non-draft segments and non-speculative steps return `None`.
    #[must_use]
    pub const fn draft_choice_subrange(self) -> Option<M1SpeculativeDraftChoiceSubrange> {
        self.draft_choice_subrange
    }

    /// A structural workspace association grants no execution authority.
    #[must_use]
    pub const fn grants_execution_authority(self) -> bool {
        false
    }
}

/// Complete checked addressless association of dispatch and workspace plans.
///
/// This owner intentionally does not implement `Clone`.
///
/// ```compile_fail
/// use ferric_engine::AddresslessM1FullStepWorkspaceComposition;
/// fn require_clone<T: Clone>() {}
/// require_clone::<AddresslessM1FullStepWorkspaceComposition>();
/// ```
#[must_use = "the exact dispatch and workspace plans must remain retained"]
#[derive(Debug, Eq, PartialEq)]
pub struct AddresslessM1FullStepWorkspaceComposition {
    dispatch_plan: Box<AddresslessM1StepDispatchPlan>,
    workspace_plans: M1FullStepWorkspacePlans,
    segment_bindings: Box<[M1FullStepWorkspaceSegmentBinding]>,
}

impl AddresslessM1FullStepWorkspaceComposition {
    /// Returns the retained exact dispatch plan.
    #[must_use]
    pub const fn dispatch_plan(&self) -> &AddresslessM1StepDispatchPlan {
        &self.dispatch_plan
    }

    /// Returns all retained exact workspace plans.
    #[must_use = "the exact linear workspace plans remain retained"]
    pub const fn workspace_plans(&self) -> &M1FullStepWorkspacePlans {
        &self.workspace_plans
    }

    /// Returns the exact segment-to-workspace associations in segment order.
    #[must_use]
    pub fn segment_bindings(&self) -> &[M1FullStepWorkspaceSegmentBinding] {
        &self.segment_bindings
    }

    /// Returns the checked association for one segment index.
    #[must_use]
    pub fn segment_binding(&self, segment_index: u8) -> Option<M1FullStepWorkspaceSegmentBinding> {
        self.segment_bindings
            .get(usize::from(segment_index))
            .copied()
            .filter(|binding| binding.segment_index == segment_index)
    }

    /// Recovers every exact linear input from the structural composition.
    #[must_use = "the exact dispatch and workspace plans remain retained"]
    pub fn into_plans(self) -> (AddresslessM1StepDispatchPlan, M1FullStepWorkspacePlans) {
        (*self.dispatch_plan, self.workspace_plans)
    }

    /// The addressless association grants no allocation or mapping authority.
    #[must_use]
    pub const fn grants_address_authority(&self) -> bool {
        false
    }

    /// The addressless association grants no packet, queue, or launch authority.
    #[must_use]
    pub const fn grants_execution_authority(&self) -> bool {
        false
    }

    /// The addressless association authenticates no workspace contents.
    #[must_use]
    pub const fn authenticates_workspace_contents(&self) -> bool {
        false
    }

    /// The addressless association establishes no completion or readback.
    #[must_use]
    pub const fn proves_completion(&self) -> bool {
        false
    }

    /// Structural compatibility alone proves no operator or machine refinement.
    #[must_use]
    pub const fn proves_refinement(&self) -> bool {
        false
    }
}

/// Fail-closed full-step workspace-composition error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1FullStepWorkspaceCompositionError {
    /// The intent must name the target model role.
    IntentRole {
        /// Required target role.
        expected: Qwen3ModelRole,
        /// Rejected role.
        actual: Qwen3ModelRole,
    },
    /// The intent mode is incompatible with its full-step shape.
    IntentMode {
        /// Full-step shape being validated.
        intent: M1FullStepWorkspaceInputKind,
        /// Rejected mode.
        actual: Qwen3ExecutionMode,
    },
    /// The intent bucket is incompatible with its role and mode.
    IntentBucket {
        /// Full-step shape being validated.
        intent: M1FullStepWorkspaceInputKind,
        /// Rejected bucket.
        actual: Qwen3PlanBucket,
    },
    /// The caller supplied the wrong finite workspace-plan input shape.
    WorkspaceInputKind {
        /// Shape required by the dispatch intent.
        expected: M1FullStepWorkspaceInputKind,
        /// Rejected supplied shape.
        actual: M1FullStepWorkspaceInputKind,
    },
    /// One workspace plan has the wrong exact role, mode, or bucket.
    WorkspaceSelection {
        /// Workspace position being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Exact required selection.
        expected: Qwen3PlanSelection,
        /// Rejected selection.
        actual: Qwen3PlanSelection,
    },
    /// Draft and target plans name the same future allocation identity.
    WorkspaceAllocationAlias {
        /// Rejected shared inert allocation identity.
        allocation_id: Identity,
    },
    /// The dispatch plan contains the wrong number of segments.
    SegmentCount {
        /// Exact count required by the intent.
        expected: usize,
        /// Rejected count.
        actual: usize,
    },
    /// A dispatch segment carries the wrong exact index.
    SegmentIndex {
        /// Segment slice position.
        position: usize,
        /// Required index.
        expected: u8,
        /// Rejected index.
        actual: u8,
    },
    /// A dispatch segment carries the wrong exact stage.
    SegmentStage {
        /// Segment slice position.
        position: usize,
        /// Required stage.
        expected: M1StepDispatchStage,
        /// Rejected stage.
        actual: M1StepDispatchStage,
    },
    /// A dispatch segment carries the wrong exact dependency.
    SegmentDependency {
        /// Segment slice position.
        position: usize,
        /// Required dependency.
        expected: M1StepDispatchDependency,
        /// Rejected dependency.
        actual: M1StepDispatchDependency,
    },
    /// A dispatch segment carries the wrong exact role, mode, or bucket.
    SegmentSelection {
        /// Segment slice position.
        position: usize,
        /// Required selection.
        expected: Qwen3PlanSelection,
        /// Rejected selection.
        actual: Qwen3PlanSelection,
    },
    /// An exact workspace range required by speculative composition is absent.
    MissingWorkspaceRange {
        /// Workspace plan being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Required semantic range.
        role: M1StepWorkspaceRangeRole,
    },
    /// A supplied range names another semantic role.
    WorkspaceRangeRole {
        /// Workspace plan being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Required semantic range.
        expected: M1StepWorkspaceRangeRole,
        /// Rejected semantic range.
        actual: M1StepWorkspaceRangeRole,
    },
    /// A required range declares the wrong exact alignment.
    WorkspaceRangeAlignment {
        /// Workspace plan being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Required semantic range.
        role: M1StepWorkspaceRangeRole,
        /// Exact required alignment.
        expected: u64,
        /// Rejected declared alignment.
        actual: u64,
    },
    /// A required range starts at an offset that does not satisfy its alignment.
    WorkspaceRangeOffsetAlignment {
        /// Workspace plan being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Required semantic range.
        role: M1StepWorkspaceRangeRole,
        /// Exact required alignment.
        alignment: u64,
        /// Rejected byte offset.
        offset: u64,
    },
    /// A required range has the wrong exact byte length.
    WorkspaceRangeLength {
        /// Workspace plan being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Required semantic range.
        role: M1StepWorkspaceRangeRole,
        /// Exact required byte length.
        expected: u64,
        /// Rejected byte length.
        actual: u64,
    },
    /// A required range exclusive end overflowed `u64`.
    WorkspaceRangeOverflow {
        /// Workspace plan being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Required semantic range.
        role: M1StepWorkspaceRangeRole,
    },
    /// A required range exceeds its declared future allocation.
    WorkspaceRangeOutOfBounds {
        /// Workspace plan being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Required semantic range.
        role: M1StepWorkspaceRangeRole,
    },
    /// Checked row-size or offset arithmetic overflowed.
    ArithmeticOverflow,
}

impl fmt::Display for M1FullStepWorkspaceCompositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 full-step workspace composition rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1FullStepWorkspaceCompositionError {}

/// Rejected composition retaining every exact linear input.
#[must_use = "all rejected dispatch and workspace plans remain recoverable"]
#[derive(Debug, Eq, PartialEq)]
pub struct M1FullStepWorkspaceCompositionFailure {
    error: M1FullStepWorkspaceCompositionError,
    dispatch_plan: Box<AddresslessM1StepDispatchPlan>,
    workspace_plans: M1FullStepWorkspacePlans,
}

impl M1FullStepWorkspaceCompositionFailure {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> M1FullStepWorkspaceCompositionError {
        self.error
    }

    /// Recovers the diagnostic and every exact unchanged input plan.
    #[must_use = "all rejected dispatch and workspace plans remain recoverable"]
    pub fn into_parts(
        self,
    ) -> (
        M1FullStepWorkspaceCompositionError,
        AddresslessM1StepDispatchPlan,
        M1FullStepWorkspacePlans,
    ) {
        (self.error, *self.dispatch_plan, self.workspace_plans)
    }
}

/// Linear result of one full-step workspace composition attempt.
#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub enum M1FullStepWorkspaceCompositionOutcome {
    /// Every exact intent, segment, dependency, and workspace check succeeded.
    Composed(AddresslessM1FullStepWorkspaceComposition),
    /// Validation failed and every exact input remains recoverable.
    Rejected(M1FullStepWorkspaceCompositionFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct M1FullStepWorkspaceContract {
    kind: M1FullStepWorkspaceInputKind,
    target_selection: Qwen3PlanSelection,
    draft_selection: Option<Qwen3PlanSelection>,
    speculative_shape: Option<(u32, u8)>,
}

/// Composes exact checked dispatch and workspace plans for one complete M1 step.
///
/// Validation rechecks the intent, every segment index/stage/dependency and
/// selection, every required workspace selection, and speculative `[K,S]`
/// preservation slices. Rejection retains all linear inputs unchanged.
pub fn compose_addressless_m1_full_step_workspaces(
    dispatch_plan: AddresslessM1StepDispatchPlan,
    workspace_plans: M1FullStepWorkspacePlans,
) -> M1FullStepWorkspaceCompositionOutcome {
    match validate_composition(&dispatch_plan, &workspace_plans) {
        Ok(segment_bindings) => M1FullStepWorkspaceCompositionOutcome::Composed(
            AddresslessM1FullStepWorkspaceComposition {
                dispatch_plan: Box::new(dispatch_plan),
                workspace_plans,
                segment_bindings,
            },
        ),
        Err(error) => {
            M1FullStepWorkspaceCompositionOutcome::Rejected(M1FullStepWorkspaceCompositionFailure {
                error,
                dispatch_plan: Box::new(dispatch_plan),
                workspace_plans,
            })
        }
    }
}

fn validate_composition(
    dispatch_plan: &AddresslessM1StepDispatchPlan,
    workspace_plans: &M1FullStepWorkspacePlans,
) -> Result<Box<[M1FullStepWorkspaceSegmentBinding]>, M1FullStepWorkspaceCompositionError> {
    let contract = intent_contract(dispatch_plan.intent())?;
    validate_input_kind(contract.kind, workspace_plans.kind())?;
    validate_workspace_selection(
        M1FullStepWorkspaceRole::Target,
        contract.target_selection,
        workspace_plans.target().selection(),
    )?;
    if let Some(expected_draft) = contract.draft_selection {
        let draft = workspace_plans.draft().ok_or(
            M1FullStepWorkspaceCompositionError::WorkspaceInputKind {
                expected: contract.kind,
                actual: workspace_plans.kind(),
            },
        )?;
        validate_workspace_selection(
            M1FullStepWorkspaceRole::Draft,
            expected_draft,
            draft.selection(),
        )?;
        if draft.allocation().allocation_id()
            == workspace_plans.target().allocation().allocation_id()
        {
            return Err(
                M1FullStepWorkspaceCompositionError::WorkspaceAllocationAlias {
                    allocation_id: draft.allocation().allocation_id(),
                },
            );
        }
    }

    let expected_segment_count = match contract.kind {
        M1FullStepWorkspaceInputKind::TargetOnly => 1,
        M1FullStepWorkspaceInputKind::PairedPrefill => 2,
        M1FullStepWorkspaceInputKind::SpeculativeRound => {
            let (_, iterations) = contract
                .speculative_shape
                .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
            usize::from(iterations)
                .checked_add(1)
                .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?
        }
    };
    validate_segment_count(expected_segment_count, dispatch_plan.segments().len())?;

    match contract.kind {
        M1FullStepWorkspaceInputKind::TargetOnly => {
            let segment = &dispatch_plan.segments()[0];
            validate_segment_fields(
                0,
                segment.segment_index(),
                segment.stage(),
                segment.dependency(),
                segment.selection(),
                0,
                M1StepDispatchStage::TargetOnly,
                M1StepDispatchDependency::ExternalInputs,
                contract.target_selection,
            )?;
            Ok(vec![segment_binding(
                segment.segment_index(),
                M1FullStepWorkspaceRole::Target,
                workspace_plans.target(),
                None,
            )]
            .into_boxed_slice())
        }
        M1FullStepWorkspaceInputKind::PairedPrefill => {
            let draft_selection = contract
                .draft_selection
                .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
            let draft = workspace_plans
                .draft()
                .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
            let expected = [
                (
                    M1StepDispatchStage::DraftPrefill,
                    M1StepDispatchDependency::ExternalInputs,
                    draft_selection,
                    M1FullStepWorkspaceRole::Draft,
                    draft,
                ),
                (
                    M1StepDispatchStage::TargetPrefill,
                    M1StepDispatchDependency::ExternalInputs,
                    contract.target_selection,
                    M1FullStepWorkspaceRole::Target,
                    workspace_plans.target(),
                ),
            ];
            let mut bindings = Vec::with_capacity(2);
            for (position, (stage, dependency, selection, role, workspace)) in
                expected.into_iter().enumerate()
            {
                let segment = &dispatch_plan.segments()[position];
                let expected_index = u8::try_from(position)
                    .map_err(|_| M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
                validate_segment_fields(
                    position,
                    segment.segment_index(),
                    segment.stage(),
                    segment.dependency(),
                    segment.selection(),
                    expected_index,
                    stage,
                    dependency,
                    selection,
                )?;
                bindings.push(segment_binding(
                    segment.segment_index(),
                    role,
                    workspace,
                    None,
                ));
            }
            Ok(bindings.into_boxed_slice())
        }
        M1FullStepWorkspaceInputKind::SpeculativeRound => {
            validate_speculative_segments(dispatch_plan, workspace_plans, contract)
        }
    }
}

fn validate_speculative_segments(
    dispatch_plan: &AddresslessM1StepDispatchPlan,
    workspace_plans: &M1FullStepWorkspacePlans,
    contract: M1FullStepWorkspaceContract,
) -> Result<Box<[M1FullStepWorkspaceSegmentBinding]>, M1FullStepWorkspaceCompositionError> {
    let draft = workspace_plans
        .draft()
        .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
    let draft_selection = contract
        .draft_selection
        .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
    let (sequences, iterations) = contract
        .speculative_shape
        .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
    let choice_subranges =
        validate_speculative_choice_ranges(workspace_plans.target(), draft, sequences, iterations)?;
    let mut bindings = Vec::with_capacity(dispatch_plan.segments().len());
    for iteration in 0..iterations {
        let position = usize::from(iteration);
        let segment = &dispatch_plan.segments()[position];
        let dependency = if iteration == 0 {
            M1StepDispatchDependency::ExternalInputs
        } else {
            M1StepDispatchDependency::PriorDraftArgmax {
                producer_segment: iteration - 1,
            }
        };
        validate_segment_fields(
            position,
            segment.segment_index(),
            segment.stage(),
            segment.dependency(),
            segment.selection(),
            iteration,
            M1StepDispatchStage::DraftDecode { iteration },
            dependency,
            draft_selection,
        )?;
        bindings.push(segment_binding(
            iteration,
            M1FullStepWorkspaceRole::Draft,
            draft,
            Some(choice_subranges[position]),
        ));
    }

    let target_position = usize::from(iterations);
    let target_segment = &dispatch_plan.segments()[target_position];
    validate_segment_fields(
        target_position,
        target_segment.segment_index(),
        target_segment.stage(),
        target_segment.dependency(),
        target_segment.selection(),
        iterations,
        M1StepDispatchStage::TargetVerification {
            draft_iterations: iterations,
        },
        M1StepDispatchDependency::DraftChoicePrefix {
            first_segment: 0,
            segment_count: iterations,
        },
        contract.target_selection,
    )?;
    bindings.push(segment_binding(
        iterations,
        M1FullStepWorkspaceRole::Target,
        workspace_plans.target(),
        None,
    ));
    Ok(bindings.into_boxed_slice())
}

fn segment_binding(
    segment_index: u8,
    workspace_role: M1FullStepWorkspaceRole,
    workspace: &AddresslessM1StepWorkspacePlan,
    draft_choice_subrange: Option<M1SpeculativeDraftChoiceSubrange>,
) -> M1FullStepWorkspaceSegmentBinding {
    M1FullStepWorkspaceSegmentBinding {
        segment_index,
        workspace_role,
        workspace_id: workspace.workspace_id(),
        workspace_selection: workspace.selection(),
        draft_choice_subrange,
    }
}

fn intent_contract(
    intent: M1StepDispatchIntent,
) -> Result<M1FullStepWorkspaceContract, M1FullStepWorkspaceCompositionError> {
    let target = intent.target_selection();
    if target.role != Qwen3ModelRole::Target8B {
        return Err(M1FullStepWorkspaceCompositionError::IntentRole {
            expected: Qwen3ModelRole::Target8B,
            actual: target.role,
        });
    }
    match intent {
        M1StepDispatchIntent::TargetOnly(selection) => {
            if !matches!(
                selection.mode,
                Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode
            ) {
                return Err(M1FullStepWorkspaceCompositionError::IntentMode {
                    intent: M1FullStepWorkspaceInputKind::TargetOnly,
                    actual: selection.mode,
                });
            }
            if !valid_bucket_for_mode(selection.mode, selection.bucket) {
                return Err(M1FullStepWorkspaceCompositionError::IntentBucket {
                    intent: M1FullStepWorkspaceInputKind::TargetOnly,
                    actual: selection.bucket,
                });
            }
            Ok(M1FullStepWorkspaceContract {
                kind: M1FullStepWorkspaceInputKind::TargetOnly,
                target_selection: selection,
                draft_selection: None,
                speculative_shape: None,
            })
        }
        M1StepDispatchIntent::PairedPrefill(selection) => {
            if selection.mode != Qwen3ExecutionMode::Prefill {
                return Err(M1FullStepWorkspaceCompositionError::IntentMode {
                    intent: M1FullStepWorkspaceInputKind::PairedPrefill,
                    actual: selection.mode,
                });
            }
            if !valid_bucket_for_mode(selection.mode, selection.bucket) {
                return Err(M1FullStepWorkspaceCompositionError::IntentBucket {
                    intent: M1FullStepWorkspaceInputKind::PairedPrefill,
                    actual: selection.bucket,
                });
            }
            Ok(M1FullStepWorkspaceContract {
                kind: M1FullStepWorkspaceInputKind::PairedPrefill,
                target_selection: selection,
                draft_selection: Some(Qwen3PlanSelection {
                    role: Qwen3ModelRole::Draft06B,
                    mode: Qwen3ExecutionMode::Prefill,
                    bucket: selection.bucket,
                }),
                speculative_shape: None,
            })
        }
        M1StepDispatchIntent::SpeculativeRound(selection) => {
            if selection.mode != Qwen3ExecutionMode::Speculative {
                return Err(M1FullStepWorkspaceCompositionError::IntentMode {
                    intent: M1FullStepWorkspaceInputKind::SpeculativeRound,
                    actual: selection.mode,
                });
            }
            let (draft_bucket, sequences, iterations) = match selection.bucket {
                Qwen3PlanBucket::SpeculativeS1K4C8192 => (Qwen3PlanBucket::DecodeS1C8192, 1, 4),
                Qwen3PlanBucket::SpeculativeS8K4C8192 => (Qwen3PlanBucket::DecodeS8C8192, 8, 4),
                Qwen3PlanBucket::SpeculativeS1K8C8192 => (Qwen3PlanBucket::DecodeS1C8192, 1, 8),
                Qwen3PlanBucket::SpeculativeS1K16C8192 => (Qwen3PlanBucket::DecodeS1C8192, 1, 16),
                _ => {
                    return Err(M1FullStepWorkspaceCompositionError::IntentBucket {
                        intent: M1FullStepWorkspaceInputKind::SpeculativeRound,
                        actual: selection.bucket,
                    });
                }
            };
            Ok(M1FullStepWorkspaceContract {
                kind: M1FullStepWorkspaceInputKind::SpeculativeRound,
                target_selection: selection,
                draft_selection: Some(Qwen3PlanSelection {
                    role: Qwen3ModelRole::Draft06B,
                    mode: Qwen3ExecutionMode::Decode,
                    bucket: draft_bucket,
                }),
                speculative_shape: Some((sequences, iterations)),
            })
        }
    }
}

const fn valid_bucket_for_mode(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> bool {
    matches!(
        (mode, bucket),
        (
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128
                | Qwen3PlanBucket::PrefillS8T128
                | Qwen3PlanBucket::PrefillS1T512
                | Qwen3PlanBucket::PrefillS1T2048
        ) | (
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192
                | Qwen3PlanBucket::DecodeS8C8192
                | Qwen3PlanBucket::DecodeS32C8192
        )
    )
}

fn validate_input_kind(
    expected: M1FullStepWorkspaceInputKind,
    actual: M1FullStepWorkspaceInputKind,
) -> Result<(), M1FullStepWorkspaceCompositionError> {
    if actual != expected {
        return Err(M1FullStepWorkspaceCompositionError::WorkspaceInputKind { expected, actual });
    }
    Ok(())
}

fn validate_workspace_selection(
    workspace: M1FullStepWorkspaceRole,
    expected: Qwen3PlanSelection,
    actual: Qwen3PlanSelection,
) -> Result<(), M1FullStepWorkspaceCompositionError> {
    if actual != expected {
        return Err(M1FullStepWorkspaceCompositionError::WorkspaceSelection {
            workspace,
            expected,
            actual,
        });
    }
    Ok(())
}

fn validate_segment_count(
    expected: usize,
    actual: usize,
) -> Result<(), M1FullStepWorkspaceCompositionError> {
    if actual != expected {
        return Err(M1FullStepWorkspaceCompositionError::SegmentCount { expected, actual });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_segment_fields(
    position: usize,
    actual_index: u8,
    actual_stage: M1StepDispatchStage,
    actual_dependency: M1StepDispatchDependency,
    actual_selection: Qwen3PlanSelection,
    expected_index: u8,
    expected_stage: M1StepDispatchStage,
    expected_dependency: M1StepDispatchDependency,
    expected_selection: Qwen3PlanSelection,
) -> Result<(), M1FullStepWorkspaceCompositionError> {
    if actual_index != expected_index {
        return Err(M1FullStepWorkspaceCompositionError::SegmentIndex {
            position,
            expected: expected_index,
            actual: actual_index,
        });
    }
    if actual_stage != expected_stage {
        return Err(M1FullStepWorkspaceCompositionError::SegmentStage {
            position,
            expected: expected_stage,
            actual: actual_stage,
        });
    }
    if actual_dependency != expected_dependency {
        return Err(M1FullStepWorkspaceCompositionError::SegmentDependency {
            position,
            expected: expected_dependency,
            actual: actual_dependency,
        });
    }
    if actual_selection != expected_selection {
        return Err(M1FullStepWorkspaceCompositionError::SegmentSelection {
            position,
            expected: expected_selection,
            actual: actual_selection,
        });
    }
    Ok(())
}

fn validate_speculative_choice_ranges(
    target: &AddresslessM1StepWorkspacePlan,
    draft: &AddresslessM1StepWorkspacePlan,
    sequences: u32,
    iterations: u8,
) -> Result<Box<[M1SpeculativeDraftChoiceSubrange]>, M1FullStepWorkspaceCompositionError> {
    let target_range = target.range(M1StepWorkspaceRangeRole::DraftChoices).ok_or(
        M1FullStepWorkspaceCompositionError::MissingWorkspaceRange {
            workspace: M1FullStepWorkspaceRole::Target,
            role: M1StepWorkspaceRangeRole::DraftChoices,
        },
    )?;
    let draft_range = draft.range(M1StepWorkspaceRangeRole::Choices).ok_or(
        M1FullStepWorkspaceCompositionError::MissingWorkspaceRange {
            workspace: M1FullStepWorkspaceRole::Draft,
            role: M1StepWorkspaceRangeRole::Choices,
        },
    )?;
    validate_speculative_choice_range_geometry(
        target.workspace_id(),
        target.allocation().allocation_id(),
        target.allocation().byte_len(),
        target_range,
        draft.allocation().byte_len(),
        draft_range,
        sequences,
        iterations,
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_speculative_choice_range_geometry(
    target_workspace_id: Identity,
    target_allocation_id: Identity,
    target_allocation_len: u64,
    target_range: M1StepWorkspaceRange,
    draft_allocation_len: u64,
    draft_range: M1StepWorkspaceRange,
    sequences: u32,
    iterations: u8,
) -> Result<Box<[M1SpeculativeDraftChoiceSubrange]>, M1FullStepWorkspaceCompositionError> {
    validate_range_role(
        M1FullStepWorkspaceRole::Target,
        M1StepWorkspaceRangeRole::DraftChoices,
        target_range,
    )?;
    validate_range_role(
        M1FullStepWorkspaceRole::Draft,
        M1StepWorkspaceRangeRole::Choices,
        draft_range,
    )?;
    validate_range_alignment(M1FullStepWorkspaceRole::Target, target_range)?;
    validate_range_alignment(M1FullStepWorkspaceRole::Draft, draft_range)?;

    let row_bytes = u64::from(sequences)
        .checked_mul(U32_BYTES)
        .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
    let target_bytes = row_bytes
        .checked_mul(u64::from(iterations))
        .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
    validate_range_length(M1FullStepWorkspaceRole::Draft, row_bytes, draft_range)?;
    validate_range_length(M1FullStepWorkspaceRole::Target, target_bytes, target_range)?;
    let target_end = validate_range_bounds(
        M1FullStepWorkspaceRole::Target,
        target_allocation_len,
        target_range,
    )?;
    validate_range_bounds(
        M1FullStepWorkspaceRole::Draft,
        draft_allocation_len,
        draft_range,
    )?;

    let mut subranges = Vec::with_capacity(usize::from(iterations));
    for iteration in 0..iterations {
        let iteration_offset = u64::from(iteration)
            .checked_mul(row_bytes)
            .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
        let offset = target_range
            .offset()
            .checked_add(iteration_offset)
            .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
        let end = offset
            .checked_add(row_bytes)
            .ok_or(M1FullStepWorkspaceCompositionError::ArithmeticOverflow)?;
        if end > target_end || end > target_allocation_len {
            return Err(
                M1FullStepWorkspaceCompositionError::WorkspaceRangeOutOfBounds {
                    workspace: M1FullStepWorkspaceRole::Target,
                    role: M1StepWorkspaceRangeRole::DraftChoices,
                },
            );
        }
        subranges.push(M1SpeculativeDraftChoiceSubrange {
            producer_segment: iteration,
            iteration,
            sequence_count: sequences,
            target_workspace_id,
            target_allocation_id,
            range: M1StepWorkspaceRange::new(
                M1StepWorkspaceRangeRole::DraftChoices,
                offset,
                row_bytes,
                U32_BYTES,
            ),
        });
    }
    Ok(subranges.into_boxed_slice())
}

fn validate_range_role(
    workspace: M1FullStepWorkspaceRole,
    expected: M1StepWorkspaceRangeRole,
    range: M1StepWorkspaceRange,
) -> Result<(), M1FullStepWorkspaceCompositionError> {
    if range.role() != expected {
        return Err(M1FullStepWorkspaceCompositionError::WorkspaceRangeRole {
            workspace,
            expected,
            actual: range.role(),
        });
    }
    Ok(())
}

fn validate_range_alignment(
    workspace: M1FullStepWorkspaceRole,
    range: M1StepWorkspaceRange,
) -> Result<(), M1FullStepWorkspaceCompositionError> {
    if range.alignment() != U32_BYTES {
        return Err(
            M1FullStepWorkspaceCompositionError::WorkspaceRangeAlignment {
                workspace,
                role: range.role(),
                expected: U32_BYTES,
                actual: range.alignment(),
            },
        );
    }
    if !range.offset().is_multiple_of(U32_BYTES) {
        return Err(
            M1FullStepWorkspaceCompositionError::WorkspaceRangeOffsetAlignment {
                workspace,
                role: range.role(),
                alignment: U32_BYTES,
                offset: range.offset(),
            },
        );
    }
    Ok(())
}

fn validate_range_length(
    workspace: M1FullStepWorkspaceRole,
    expected: u64,
    range: M1StepWorkspaceRange,
) -> Result<(), M1FullStepWorkspaceCompositionError> {
    if range.byte_len() != expected {
        return Err(M1FullStepWorkspaceCompositionError::WorkspaceRangeLength {
            workspace,
            role: range.role(),
            expected,
            actual: range.byte_len(),
        });
    }
    Ok(())
}

fn validate_range_bounds(
    workspace: M1FullStepWorkspaceRole,
    allocation_len: u64,
    range: M1StepWorkspaceRange,
) -> Result<u64, M1FullStepWorkspaceCompositionError> {
    let end = range.checked_end().ok_or(
        M1FullStepWorkspaceCompositionError::WorkspaceRangeOverflow {
            workspace,
            role: range.role(),
        },
    )?;
    if end > allocation_len {
        return Err(
            M1FullStepWorkspaceCompositionError::WorkspaceRangeOutOfBounds {
                workspace,
                role: range.role(),
            },
        );
    }
    Ok(end)
}

#[cfg(test)]
mod tests {
    use ferric_build::{
        m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
        AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace,
        DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
        M1StepWorkspaceRange, M1StepWorkspaceRangeRole,
    };
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
    };

    use super::{
        compose_addressless_m1_full_step_workspaces, intent_contract, validate_segment_count,
        validate_segment_fields, validate_speculative_choice_range_geometry,
        M1FullStepWorkspaceCompositionError, M1FullStepWorkspaceCompositionOutcome,
        M1FullStepWorkspaceInputKind, M1FullStepWorkspacePlans, M1FullStepWorkspaceRole,
    };
    use crate::operation_kernel_plan::tests::public_operation_kernel_plan_fixture;
    use crate::{
        derive_m1_step_dispatch_plan, M1StepDispatchDependency, M1StepDispatchIntent,
        M1StepDispatchStage,
    };

    const PREFILL_BUCKETS: [Qwen3PlanBucket; 4] = [
        Qwen3PlanBucket::PrefillS1T128,
        Qwen3PlanBucket::PrefillS8T128,
        Qwen3PlanBucket::PrefillS1T512,
        Qwen3PlanBucket::PrefillS1T2048,
    ];
    const DECODE_BUCKETS: [Qwen3PlanBucket; 3] = [
        Qwen3PlanBucket::DecodeS1C8192,
        Qwen3PlanBucket::DecodeS8C8192,
        Qwen3PlanBucket::DecodeS32C8192,
    ];
    const SPECULATIVE_CASES: [(Qwen3PlanBucket, Qwen3PlanBucket, u32, u8); 4] = [
        (
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3PlanBucket::DecodeS1C8192,
            1,
            4,
        ),
        (
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3PlanBucket::DecodeS8C8192,
            8,
            4,
        ),
        (
            Qwen3PlanBucket::SpeculativeS1K8C8192,
            Qwen3PlanBucket::DecodeS1C8192,
            1,
            8,
        ),
        (
            Qwen3PlanBucket::SpeculativeS1K16C8192,
            Qwen3PlanBucket::DecodeS1C8192,
            1,
            16,
        ),
    ];

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

    fn composed(
        outcome: M1FullStepWorkspaceCompositionOutcome,
    ) -> super::AddresslessM1FullStepWorkspaceComposition {
        match outcome {
            M1FullStepWorkspaceCompositionOutcome::Composed(composition) => composition,
            M1FullStepWorkspaceCompositionOutcome::Rejected(failure) => {
                panic!("exact composition rejected: {:?}", failure.error())
            }
        }
    }

    #[test]
    fn every_target_only_prefill_and_decode_bucket_selects_one_target_workspace() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let cases = PREFILL_BUCKETS
            .into_iter()
            .map(|bucket| target(Qwen3ExecutionMode::Prefill, bucket))
            .chain(
                DECODE_BUCKETS
                    .into_iter()
                    .map(|bucket| target(Qwen3ExecutionMode::Decode, bucket)),
            );
        for (case, target_selection) in cases.enumerate() {
            let case = u8::try_from(case).unwrap();
            let dispatch = derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::TargetOnly(target_selection),
            )
            .unwrap();
            let target_plan = exact_workspace_plan(target_selection, 10 + case);
            let target_id = target_plan.workspace_id();
            let composition = composed(compose_addressless_m1_full_step_workspaces(
                dispatch,
                M1FullStepWorkspacePlans::target_only(target_plan),
            ));
            assert_eq!(composition.segment_bindings().len(), 1);
            let binding = composition.segment_binding(0).unwrap();
            assert_eq!(binding.workspace_role(), M1FullStepWorkspaceRole::Target);
            assert_eq!(binding.workspace_id(), target_id);
            assert_eq!(binding.workspace_selection(), target_selection);
            assert_eq!(binding.draft_choice_subrange(), None);
        }
    }

    #[test]
    fn every_paired_prefill_bucket_selects_exact_draft_then_target_workspaces() {
        let operation_plan = public_operation_kernel_plan_fixture();
        for (case, bucket) in PREFILL_BUCKETS.into_iter().enumerate() {
            let case = u8::try_from(case).unwrap();
            let target_selection = target(Qwen3ExecutionMode::Prefill, bucket);
            let draft_selection = selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Prefill,
                bucket,
            );
            let dispatch = derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::PairedPrefill(target_selection),
            )
            .unwrap();
            let draft_plan = exact_workspace_plan(draft_selection, 30 + case);
            let target_plan = exact_workspace_plan(target_selection, 40 + case);
            let draft_id = draft_plan.workspace_id();
            let target_id = target_plan.workspace_id();
            let composition = composed(compose_addressless_m1_full_step_workspaces(
                dispatch,
                M1FullStepWorkspacePlans::paired_prefill(draft_plan, target_plan),
            ));
            assert_eq!(composition.segment_bindings().len(), 2);
            assert_eq!(
                composition.segment_binding(0).unwrap().workspace_role(),
                M1FullStepWorkspaceRole::Draft
            );
            assert_eq!(
                composition.segment_binding(0).unwrap().workspace_id(),
                draft_id
            );
            assert_eq!(
                composition.segment_binding(1).unwrap().workspace_role(),
                M1FullStepWorkspaceRole::Target
            );
            assert_eq!(
                composition.segment_binding(1).unwrap().workspace_id(),
                target_id
            );
        }
    }

    #[test]
    fn every_speculative_bucket_reuses_draft_and_slices_target_choices_iteration_major() {
        let operation_plan = public_operation_kernel_plan_fixture();
        for (case, (target_bucket, draft_bucket, sequences, iterations)) in
            SPECULATIVE_CASES.into_iter().enumerate()
        {
            let case = u8::try_from(case).unwrap();
            let target_selection = target(Qwen3ExecutionMode::Speculative, target_bucket);
            let draft_selection = selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                draft_bucket,
            );
            let dispatch = derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::SpeculativeRound(target_selection),
            )
            .unwrap();
            let draft_plan = exact_workspace_plan(draft_selection, 50 + case);
            let target_plan = exact_workspace_plan(target_selection, 60 + case);
            let draft_id = draft_plan.workspace_id();
            let target_id = target_plan.workspace_id();
            let target_allocation_id = target_plan.allocation().allocation_id();
            let whole = target_plan
                .range(M1StepWorkspaceRangeRole::DraftChoices)
                .unwrap();
            let composition = composed(compose_addressless_m1_full_step_workspaces(
                dispatch,
                M1FullStepWorkspacePlans::speculative_round(draft_plan, target_plan),
            ));
            assert_eq!(
                composition.segment_bindings().len(),
                usize::from(iterations) + 1
            );
            let row_bytes = u64::from(sequences) * 4;
            for iteration in 0..iterations {
                let binding = composition.segment_binding(iteration).unwrap();
                assert_eq!(binding.workspace_role(), M1FullStepWorkspaceRole::Draft);
                assert_eq!(binding.workspace_id(), draft_id);
                assert_eq!(binding.workspace_selection(), draft_selection);
                let subrange = binding.draft_choice_subrange().unwrap();
                assert_eq!(subrange.producer_segment(), iteration);
                assert_eq!(subrange.iteration(), iteration);
                assert_eq!(subrange.sequence_count(), sequences);
                assert_eq!(subrange.target_workspace_id(), target_id);
                assert_eq!(subrange.target_allocation_id(), target_allocation_id);
                assert_eq!(
                    subrange.range().role(),
                    M1StepWorkspaceRangeRole::DraftChoices
                );
                assert_eq!(
                    subrange.range().offset(),
                    whole.offset() + u64::from(iteration) * row_bytes
                );
                assert_eq!(subrange.range().byte_len(), row_bytes);
                assert_eq!(subrange.range().alignment(), 4);
                assert_eq!(
                    subrange.sequence_byte_offset(0),
                    Some(subrange.range().offset())
                );
                assert_eq!(
                    subrange.sequence_byte_offset(sequences - 1),
                    Some(subrange.range().offset() + row_bytes - 4)
                );
                assert_eq!(subrange.sequence_byte_offset(sequences), None);
                assert!(!subrange.authenticates_preserved_choice());
            }
            let target_binding = composition.segment_binding(iterations).unwrap();
            assert_eq!(
                target_binding.workspace_role(),
                M1FullStepWorkspaceRole::Target
            );
            assert_eq!(target_binding.workspace_id(), target_id);
            assert_eq!(target_binding.draft_choice_subrange(), None);
            assert!(!composition.grants_address_authority());
            assert!(!composition.grants_execution_authority());
            assert!(!composition.authenticates_workspace_contents());
            assert!(!composition.proves_completion());
            assert!(!composition.proves_refinement());
        }
    }

    #[test]
    fn wrong_input_shape_rejects_and_recovers_all_linear_plans() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let target_selection = target(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let draft_selection = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let dispatch = derive_m1_step_dispatch_plan(
            &operation_plan,
            M1StepDispatchIntent::TargetOnly(target_selection),
        )
        .unwrap();
        let dispatch_id = dispatch.composition_id();
        let draft = exact_workspace_plan(draft_selection, 71);
        let target = exact_workspace_plan(target_selection, 72);
        let draft_id = draft.workspace_id();
        let target_id = target.workspace_id();
        let outcome = compose_addressless_m1_full_step_workspaces(
            dispatch,
            M1FullStepWorkspacePlans::paired_prefill(draft, target),
        );
        let M1FullStepWorkspaceCompositionOutcome::Rejected(failure) = outcome else {
            panic!("hostile input shape composed");
        };
        assert_eq!(
            failure.error(),
            M1FullStepWorkspaceCompositionError::WorkspaceInputKind {
                expected: M1FullStepWorkspaceInputKind::TargetOnly,
                actual: M1FullStepWorkspaceInputKind::PairedPrefill,
            }
        );
        let (_, recovered_dispatch, recovered_workspaces) = failure.into_parts();
        assert_eq!(recovered_dispatch.composition_id(), dispatch_id);
        assert_eq!(
            recovered_workspaces.draft().unwrap().workspace_id(),
            draft_id
        );
        assert_eq!(recovered_workspaces.target().workspace_id(), target_id);
    }

    #[test]
    fn hostile_workspace_role_mode_bucket_and_allocation_substitutions_fail_closed() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let expected = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let substitutions = [
            selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            target(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128),
            target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192),
        ];
        for (case, hostile) in substitutions.into_iter().enumerate() {
            let case = u8::try_from(case).unwrap();
            let dispatch = derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::TargetOnly(expected),
            )
            .unwrap();
            let outcome = compose_addressless_m1_full_step_workspaces(
                dispatch,
                M1FullStepWorkspacePlans::target_only(exact_workspace_plan(hostile, 80 + case)),
            );
            let M1FullStepWorkspaceCompositionOutcome::Rejected(failure) = outcome else {
                panic!("hostile workspace selection composed");
            };
            assert_eq!(
                failure.error(),
                M1FullStepWorkspaceCompositionError::WorkspaceSelection {
                    workspace: M1FullStepWorkspaceRole::Target,
                    expected,
                    actual: hostile,
                }
            );
        }

        let target_selection = target(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let draft_selection = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS1T128,
        );
        let dispatch = derive_m1_step_dispatch_plan(
            &operation_plan,
            M1StepDispatchIntent::PairedPrefill(target_selection),
        )
        .unwrap();
        let draft = exact_workspace_plan(draft_selection, 90);
        let target = exact_workspace_plan(target_selection, 90);
        let outcome = compose_addressless_m1_full_step_workspaces(
            dispatch,
            M1FullStepWorkspacePlans::paired_prefill(draft, target),
        );
        let M1FullStepWorkspaceCompositionOutcome::Rejected(failure) = outcome else {
            panic!("aliased future allocation composed");
        };
        assert!(matches!(
            failure.error(),
            M1FullStepWorkspaceCompositionError::WorkspaceAllocationAlias { .. }
        ));
    }

    #[test]
    fn hostile_intent_segment_and_dependency_fields_are_rejected_exactly() {
        let draft = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        assert!(matches!(
            intent_contract(M1StepDispatchIntent::TargetOnly(draft)),
            Err(M1FullStepWorkspaceCompositionError::IntentRole { .. })
        ));
        assert!(matches!(
            intent_contract(M1StepDispatchIntent::PairedPrefill(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ))),
            Err(M1FullStepWorkspaceCompositionError::IntentMode { .. })
        ));
        assert!(matches!(
            intent_contract(M1StepDispatchIntent::TargetOnly(target(
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::PrefillS1T128,
            ))),
            Err(M1FullStepWorkspaceCompositionError::IntentBucket { .. })
        ));
        assert_eq!(
            validate_segment_count(5, 4),
            Err(M1FullStepWorkspaceCompositionError::SegmentCount {
                expected: 5,
                actual: 4,
            })
        );

        let exact_selection = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let exact_stage = M1StepDispatchStage::DraftDecode { iteration: 1 };
        let exact_dependency = M1StepDispatchDependency::PriorDraftArgmax {
            producer_segment: 0,
        };
        let check = |actual_index, actual_stage, actual_dependency, actual_selection| {
            validate_segment_fields(
                1,
                actual_index,
                actual_stage,
                actual_dependency,
                actual_selection,
                1,
                exact_stage,
                exact_dependency,
                exact_selection,
            )
        };
        assert!(matches!(
            check(2, exact_stage, exact_dependency, exact_selection),
            Err(M1FullStepWorkspaceCompositionError::SegmentIndex { .. })
        ));
        assert!(matches!(
            check(
                1,
                M1StepDispatchStage::TargetOnly,
                exact_dependency,
                exact_selection
            ),
            Err(M1FullStepWorkspaceCompositionError::SegmentStage { .. })
        ));
        assert!(matches!(
            check(
                1,
                exact_stage,
                M1StepDispatchDependency::ExternalInputs,
                exact_selection
            ),
            Err(M1FullStepWorkspaceCompositionError::SegmentDependency { .. })
        ));
        assert!(matches!(
            check(1, exact_stage, exact_dependency, draft),
            Err(M1FullStepWorkspaceCompositionError::SegmentSelection { .. })
        ));
    }

    #[test]
    fn hostile_speculative_choice_geometry_rejects_role_alignment_length_overflow_and_bounds() {
        let target_id = Identity::new([101; 32]);
        let allocation_id = Identity::new([102; 32]);
        let exact_target =
            M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 64, 16, 4);
        let exact_draft = M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::Choices, 32, 4, 4);
        let validate = |target_range, target_len, draft_range, draft_len| {
            validate_speculative_choice_range_geometry(
                target_id,
                allocation_id,
                target_len,
                target_range,
                draft_len,
                draft_range,
                1,
                4,
            )
        };
        assert!(matches!(
            validate(
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::Choices, 64, 16, 4),
                80,
                exact_draft,
                36,
            ),
            Err(M1FullStepWorkspaceCompositionError::WorkspaceRangeRole { .. })
        ));
        assert!(matches!(
            validate(
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 66, 16, 4),
                82,
                exact_draft,
                36,
            ),
            Err(M1FullStepWorkspaceCompositionError::WorkspaceRangeOffsetAlignment { .. })
        ));
        assert!(matches!(
            validate(
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 64, 16, 8,),
                80,
                exact_draft,
                36,
            ),
            Err(M1FullStepWorkspaceCompositionError::WorkspaceRangeAlignment { .. })
        ));
        assert!(matches!(
            validate(
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 64, 12, 4),
                80,
                exact_draft,
                36,
            ),
            Err(M1FullStepWorkspaceCompositionError::WorkspaceRangeLength { .. })
        ));
        assert!(matches!(
            validate(
                M1StepWorkspaceRange::new(
                    M1StepWorkspaceRangeRole::DraftChoices,
                    u64::MAX - 15,
                    16,
                    4,
                ),
                u64::MAX,
                exact_draft,
                36,
            ),
            Err(M1FullStepWorkspaceCompositionError::WorkspaceRangeOverflow { .. })
        ));
        assert!(matches!(
            validate(exact_target, 79, exact_draft, 36),
            Err(M1FullStepWorkspaceCompositionError::WorkspaceRangeOutOfBounds { .. })
        ));
    }
}
