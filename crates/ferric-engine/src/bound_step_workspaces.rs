//! Owner-checked workspace custody for one complete M1 dispatch step.
//!
//! This layer joins an addressless full-step composition to already-bound
//! target and draft workspace partitions. It retains every linear input and
//! exposes only generic owner-checked dispatch ranges. It constructs no native
//! address, packet, queue publication, launch, completion, readback, content
//! authentication, hardware result, performance result, or refinement claim.

use core::fmt;

use fe2o3_service_host::{
    ServiceAllocationErrorV1, ServiceAllocationSessionV1, ServiceDeviceDispatchRangeV1,
};
use ferric_build::{AddresslessM1StepWorkspacePlan, M1StepWorkspaceRangeRole};

use crate::{
    AddresslessM1FullStepWorkspaceComposition, BoundM1StepWorkspaceSubleases,
    M1FullStepWorkspaceInputKind, M1FullStepWorkspaceRole, M1FullStepWorkspaceSegmentBinding,
    M1SpeculativeDraftChoiceSubrange, M1SpeculativeDraftMetadataSubrange, M1StepDispatchStage,
    M1StepWorkspaceDispatchRangeError, M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
    M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
    M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
};

type DraftWorkspaceOwner = BoundM1StepWorkspaceSubleases<M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1>;
type TargetWorkspaceOwner =
    BoundM1StepWorkspaceSubleases<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>;
type TargetSpeculativeWorkspaceOwner =
    BoundM1StepWorkspaceSubleases<M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1>;

const U32_BYTES: u64 = 4;

/// Exact already-bound workspace-owner input shape for one complete M1 step.
///
/// The const cardinalities stay tied to the exported canonical roster constants
/// so later reviewed workspace-role additions cannot silently retain an old
/// literal shape.
#[must_use = "all exact workspace owners must remain retained"]
#[derive(Debug)]
pub enum M1FullStepWorkspaceSubleaseOwners {
    /// One ordinary target workspace for target-only prefill or decode.
    TargetOnly {
        /// Exact target owner.
        target: Box<TargetWorkspaceOwner>,
    },
    /// Draft and target prefill workspace owners.
    PairedPrefill {
        /// Exact draft prefill owner.
        draft: Box<DraftWorkspaceOwner>,
        /// Exact target prefill owner.
        target: Box<TargetWorkspaceOwner>,
    },
    /// Reusable draft decode and target speculative workspace owners.
    SpeculativeRound {
        /// Exact reusable draft decode owner.
        draft_decode: Box<DraftWorkspaceOwner>,
        /// Exact target speculative owner.
        target_speculative: Box<TargetSpeculativeWorkspaceOwner>,
    },
}

impl M1FullStepWorkspaceSubleaseOwners {
    /// Wraps one exact ordinary target owner.
    #[must_use = "the target workspace owner remains retained"]
    pub fn target_only(target: TargetWorkspaceOwner) -> Self {
        Self::TargetOnly {
            target: Box::new(target),
        }
    }

    /// Wraps exact draft and target prefill owners.
    #[must_use = "the draft and target workspace owners remain retained"]
    pub fn paired_prefill(draft: DraftWorkspaceOwner, target: TargetWorkspaceOwner) -> Self {
        Self::PairedPrefill {
            draft: Box::new(draft),
            target: Box::new(target),
        }
    }

    /// Wraps exact draft-decode and target-speculative owners.
    #[must_use = "the draft and target workspace owners remain retained"]
    pub fn speculative_round(
        draft_decode: DraftWorkspaceOwner,
        target_speculative: TargetSpeculativeWorkspaceOwner,
    ) -> Self {
        Self::SpeculativeRound {
            draft_decode: Box::new(draft_decode),
            target_speculative: Box::new(target_speculative),
        }
    }

    /// Returns the exact finite owner-input shape.
    #[must_use]
    pub const fn kind(&self) -> M1FullStepWorkspaceInputKind {
        match self {
            Self::TargetOnly { .. } => M1FullStepWorkspaceInputKind::TargetOnly,
            Self::PairedPrefill { .. } => M1FullStepWorkspaceInputKind::PairedPrefill,
            Self::SpeculativeRound { .. } => M1FullStepWorkspaceInputKind::SpeculativeRound,
        }
    }

    fn metadata(&self) -> M1FullStepWorkspaceOwnerMetadata<'_> {
        match self {
            Self::TargetOnly { target } => M1FullStepWorkspaceOwnerMetadata::TargetOnly {
                target: WorkspaceOwnerMetadata::new(target.plan(), target.member_count()),
            },
            Self::PairedPrefill { draft, target } => {
                M1FullStepWorkspaceOwnerMetadata::PairedPrefill {
                    draft: WorkspaceOwnerMetadata::new(draft.plan(), draft.member_count()),
                    target: WorkspaceOwnerMetadata::new(target.plan(), target.member_count()),
                }
            }
            Self::SpeculativeRound {
                draft_decode,
                target_speculative,
            } => M1FullStepWorkspaceOwnerMetadata::SpeculativeRound {
                draft: WorkspaceOwnerMetadata::new(
                    draft_decode.plan(),
                    draft_decode.member_count(),
                ),
                target: WorkspaceOwnerMetadata::new(
                    target_speculative.plan(),
                    target_speculative.member_count(),
                ),
            },
        }
    }

    fn revalidate(
        &self,
        allocations: &ServiceAllocationSessionV1,
    ) -> Result<(), M1FullStepWorkspaceSubleaseBindingError> {
        match self {
            Self::TargetOnly { target } => target
                .revalidate_dispatch_ranges(allocations)
                .map_err(|error| allocation_error(M1FullStepWorkspaceRole::Target, error)),
            Self::PairedPrefill { draft, target } => {
                draft
                    .revalidate_dispatch_ranges(allocations)
                    .map_err(|error| allocation_error(M1FullStepWorkspaceRole::Draft, error))?;
                target
                    .revalidate_dispatch_ranges(allocations)
                    .map_err(|error| allocation_error(M1FullStepWorkspaceRole::Target, error))
            }
            Self::SpeculativeRound {
                draft_decode,
                target_speculative,
            } => {
                draft_decode
                    .revalidate_dispatch_ranges(allocations)
                    .map_err(|error| allocation_error(M1FullStepWorkspaceRole::Draft, error))?;
                target_speculative
                    .revalidate_dispatch_ranges(allocations)
                    .map_err(|error| allocation_error(M1FullStepWorkspaceRole::Target, error))
            }
        }
    }
}

/// Fail-closed full-step workspace-owner binding error.
#[derive(Debug)]
pub enum M1FullStepWorkspaceSubleaseBindingError {
    /// The supplied finite owner shape differs from the composition shape.
    OwnerInputKind {
        /// Exact required shape.
        expected: M1FullStepWorkspaceInputKind,
        /// Rejected owner shape.
        actual: M1FullStepWorkspaceInputKind,
    },
    /// An owner retains the wrong exact workspace selection.
    WorkspaceSelection {
        /// Workspace position being validated.
        workspace: M1FullStepWorkspaceRole,
    },
    /// An owner retains another workspace-plan identity.
    WorkspaceIdentity {
        /// Workspace position being validated.
        workspace: M1FullStepWorkspaceRole,
    },
    /// An owner retains another declared allocation identity.
    WorkspaceAllocationIdentity {
        /// Workspace position being validated.
        workspace: M1FullStepWorkspaceRole,
    },
    /// An owner retains a nonidentical addressless workspace plan.
    WorkspacePlan {
        /// Workspace position being validated.
        workspace: M1FullStepWorkspaceRole,
    },
    /// An owner's compile-time roster does not match its exact retained plan.
    WorkspaceMemberCount {
        /// Workspace position being validated.
        workspace: M1FullStepWorkspaceRole,
        /// Exact retained-plan member count.
        expected: usize,
        /// Rejected owner member count.
        actual: usize,
    },
    /// Dispatch segments and retained workspace bindings differ in length.
    SegmentCount {
        /// Exact dispatch segment count.
        expected: usize,
        /// Rejected workspace-binding count.
        actual: usize,
    },
    /// A segment or binding carries the wrong exact index.
    SegmentIndex {
        /// Slice position being validated.
        position: usize,
    },
    /// A segment is associated with the wrong primary workspace role.
    SegmentWorkspaceRole {
        /// Slice position being validated.
        position: usize,
    },
    /// A segment binding retains another workspace identity.
    SegmentWorkspaceIdentity {
        /// Slice position being validated.
        position: usize,
    },
    /// A segment binding retains another exact selection.
    SegmentSelection {
        /// Slice position being validated.
        position: usize,
    },
    /// A non-draft segment unexpectedly retains speculative choice-row metadata.
    UnexpectedDraftChoiceSubrange {
        /// Slice position being validated.
        position: usize,
    },
    /// A speculative draft segment is missing its exact target choice row.
    MissingDraftChoiceSubrange {
        /// Slice position being validated.
        position: usize,
    },
    /// A speculative target choice row has wrong identity, role, iteration, or geometry.
    DraftChoiceSubrange {
        /// Slice position being validated.
        position: usize,
    },
    /// A non-draft segment unexpectedly retains speculative metadata-row declarations.
    UnexpectedDraftMetadataSubrange {
        /// Slice position being validated.
        position: usize,
        /// Unexpected target metadata role.
        role: M1StepWorkspaceRangeRole,
    },
    /// A speculative draft segment is missing one exact target metadata row.
    MissingDraftMetadataSubrange {
        /// Slice position being validated.
        position: usize,
        /// Missing target metadata role.
        role: M1StepWorkspaceRangeRole,
    },
    /// A speculative target metadata row has wrong identity, selection, role, iteration, or geometry.
    DraftMetadataSubrange {
        /// Slice position being validated.
        position: usize,
        /// Rejected target metadata role.
        role: M1StepWorkspaceRangeRole,
    },
    /// The generic allocation owner rejected a retained partition or generation.
    Allocation {
        /// Workspace position being revalidated.
        workspace: M1FullStepWorkspaceRole,
        /// Generic owner diagnostic.
        error: ServiceAllocationErrorV1,
    },
}

impl fmt::Display for M1FullStepWorkspaceSubleaseBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 full-step workspace sublease binding rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1FullStepWorkspaceSubleaseBindingError {}

/// Rejected join retaining the exact unchanged composition and all owners.
#[must_use = "all rejected linear workspace inputs remain recoverable"]
#[derive(Debug)]
pub struct M1FullStepWorkspaceSubleaseBindingFailure {
    error: M1FullStepWorkspaceSubleaseBindingError,
    composition: Box<AddresslessM1FullStepWorkspaceComposition>,
    owners: Box<M1FullStepWorkspaceSubleaseOwners>,
}

impl M1FullStepWorkspaceSubleaseBindingFailure {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> &M1FullStepWorkspaceSubleaseBindingError {
        &self.error
    }

    /// Recovers the diagnostic and every exact unchanged linear input.
    #[must_use = "the exact composition and workspace owners remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1FullStepWorkspaceSubleaseBindingError,
        AddresslessM1FullStepWorkspaceComposition,
        M1FullStepWorkspaceSubleaseOwners,
    ) {
        (self.error, *self.composition, *self.owners)
    }
}

/// Failure while resolving a segment-scoped workspace dispatch range.
#[derive(Debug)]
pub enum M1FullStepWorkspaceDispatchRangeError {
    /// The requested segment index is absent.
    SegmentIndex { segment_index: u8 },
    /// The requested workspace is not available to this exact segment.
    WorkspaceUnavailable {
        /// Requested segment.
        segment_index: u8,
        /// Requested workspace position.
        workspace: M1FullStepWorkspaceRole,
    },
    /// The segment has no speculative target choice row.
    DraftChoiceSubrangeUnavailable { segment_index: u8 },
    /// The segment has no speculative target draft-position row.
    DraftPositionSubrangeUnavailable { segment_index: u8 },
    /// The segment has no speculative target draft-context row.
    DraftContextSubrangeUnavailable { segment_index: u8 },
    /// The exact role-contained range resolver rejected the request.
    Range {
        /// Workspace position being resolved.
        workspace: M1FullStepWorkspaceRole,
        /// Exact role-contained range diagnostic.
        error: M1StepWorkspaceDispatchRangeError,
    },
}

impl fmt::Display for M1FullStepWorkspaceDispatchRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 full-step workspace dispatch range rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1FullStepWorkspaceDispatchRangeError {}

/// Move-only custody of a full-step composition and every exact workspace owner.
///
/// ```compile_fail
/// use ferric_engine::BoundM1FullStepWorkspaceSubleases;
/// fn require_clone<T: Clone>() {}
/// require_clone::<BoundM1FullStepWorkspaceSubleases>();
/// ```
#[must_use = "the exact composition and workspace owners must remain retained"]
#[derive(Debug)]
pub struct BoundM1FullStepWorkspaceSubleases {
    composition: AddresslessM1FullStepWorkspaceComposition,
    owners: M1FullStepWorkspaceSubleaseOwners,
}

impl BoundM1FullStepWorkspaceSubleases {
    /// Returns the exact retained addressless composition.
    #[must_use = "the retained full-step composition must remain associated with its owners"]
    pub const fn composition(&self) -> &AddresslessM1FullStepWorkspaceComposition {
        &self.composition
    }

    /// Returns the exact finite owner shape.
    #[must_use]
    pub const fn input_kind(&self) -> M1FullStepWorkspaceInputKind {
        self.owners.kind()
    }

    /// Resolves a role from a workspace available to one exact segment.
    ///
    /// Every segment may resolve its primary workspace. Speculative draft
    /// segments may additionally resolve the target workspace because their
    /// argmax output and position/context inputs use target-owned
    /// iteration-major state.
    ///
    /// # Errors
    ///
    /// Returns [`M1FullStepWorkspaceDispatchRangeError`] for an absent segment,
    /// unavailable cross-workspace request, inactive role, invalid subrange, or
    /// generic allocation-owner rejection.
    pub fn segment_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        segment_index: u8,
        workspace: M1FullStepWorkspaceRole,
        role: M1StepWorkspaceRangeRole,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        let binding = self
            .composition
            .segment_binding(segment_index)
            .ok_or(M1FullStepWorkspaceDispatchRangeError::SegmentIndex { segment_index })?;
        let stage = self
            .composition
            .dispatch_plan()
            .segments()
            .get(usize::from(segment_index))
            .filter(|segment| segment.segment_index() == segment_index)
            .ok_or(M1FullStepWorkspaceDispatchRangeError::SegmentIndex { segment_index })?
            .stage();
        let available = binding.workspace_role() == workspace
            || matches!(stage, M1StepDispatchStage::DraftDecode { .. })
                && workspace == M1FullStepWorkspaceRole::Target;
        if !available {
            return Err(
                M1FullStepWorkspaceDispatchRangeError::WorkspaceUnavailable {
                    segment_index,
                    workspace,
                },
            );
        }
        self.whole_workspace_dispatch_range(allocations, workspace, role)
            .map_err(|error| M1FullStepWorkspaceDispatchRangeError::Range { workspace, error })
    }

    /// Returns the exact addressless target `DraftChoices` row for one draft segment.
    #[must_use]
    pub fn speculative_draft_choice_subrange(
        &self,
        producer_segment: u8,
    ) -> Option<M1SpeculativeDraftChoiceSubrange> {
        self.composition
            .segment_binding(producer_segment)?
            .draft_choice_subrange()
    }

    /// Resolves the exact target-owned `DraftChoices` row for one draft segment.
    ///
    /// The returned range retains the target allocation generation and the
    /// target `DraftChoices` member index even though the producing segment's
    /// primary workspace is the reusable draft workspace.
    ///
    /// # Errors
    ///
    /// Returns [`M1FullStepWorkspaceDispatchRangeError`] for an absent or
    /// non-draft segment, hostile row metadata, or generic owner rejection.
    pub fn speculative_draft_choice_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        producer_segment: u8,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        let row = self
            .speculative_draft_choice_subrange(producer_segment)
            .ok_or(
                M1FullStepWorkspaceDispatchRangeError::DraftChoiceSubrangeUnavailable {
                    segment_index: producer_segment,
                },
            )?;
        self.workspace_dispatch_subrange(
            allocations,
            M1FullStepWorkspaceRole::Target,
            M1StepWorkspaceRangeRole::DraftChoices,
            row.range(),
        )
        .map_err(|error| M1FullStepWorkspaceDispatchRangeError::Range {
            workspace: M1FullStepWorkspaceRole::Target,
            error,
        })
    }

    /// Resolves the exact target-owned `DraftPositionIds` row for one draft segment.
    ///
    /// The returned range retains the target member index and allocation
    /// generation while narrowing to the current iteration's exact row.
    ///
    /// # Errors
    ///
    /// Returns [`M1FullStepWorkspaceDispatchRangeError`] for an absent or
    /// non-draft segment, hostile row metadata, or generic owner rejection.
    pub fn speculative_draft_position_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        draft_segment: u8,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        let row = self
            .composition
            .segment_binding(draft_segment)
            .and_then(M1FullStepWorkspaceSegmentBinding::draft_position_ids_subrange)
            .ok_or(
                M1FullStepWorkspaceDispatchRangeError::DraftPositionSubrangeUnavailable {
                    segment_index: draft_segment,
                },
            )?;
        self.speculative_draft_metadata_dispatch_range(allocations, row)
    }

    /// Resolves the exact target-owned `DraftContextLengths` row for one draft segment.
    ///
    /// The returned range retains the target member index and allocation
    /// generation while narrowing to the current iteration's exact row.
    ///
    /// # Errors
    ///
    /// Returns [`M1FullStepWorkspaceDispatchRangeError`] for an absent or
    /// non-draft segment, hostile row metadata, or generic owner rejection.
    pub fn speculative_draft_context_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        draft_segment: u8,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        let row = self
            .composition
            .segment_binding(draft_segment)
            .and_then(M1FullStepWorkspaceSegmentBinding::draft_context_lengths_subrange)
            .ok_or(
                M1FullStepWorkspaceDispatchRangeError::DraftContextSubrangeUnavailable {
                    segment_index: draft_segment,
                },
            )?;
        self.speculative_draft_metadata_dispatch_range(allocations, row)
    }

    /// Recovers the exact retained composition and all workspace owners.
    #[must_use = "the exact composition and workspace owners remain retained"]
    pub fn into_parts(
        self,
    ) -> (
        AddresslessM1FullStepWorkspaceComposition,
        M1FullStepWorkspaceSubleaseOwners,
    ) {
        (self.composition, self.owners)
    }

    /// This custody grants no packet, queue, launch, or completion authority.
    #[must_use]
    pub const fn grants_execution_authority(&self) -> bool {
        false
    }

    fn whole_workspace_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        workspace: M1FullStepWorkspaceRole,
        role: M1StepWorkspaceRangeRole,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1StepWorkspaceDispatchRangeError> {
        let owner_plan = self
            .owner_plan(workspace)
            .ok_or(M1StepWorkspaceDispatchRangeError::InactiveRole { role })?;
        let range = owner_plan
            .range(role)
            .ok_or(M1StepWorkspaceDispatchRangeError::InactiveRole { role })?;
        self.workspace_dispatch_subrange(allocations, workspace, role, range)
    }

    fn speculative_draft_metadata_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        row: M1SpeculativeDraftMetadataSubrange,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1FullStepWorkspaceDispatchRangeError> {
        self.workspace_dispatch_subrange(
            allocations,
            M1FullStepWorkspaceRole::Target,
            row.range().role(),
            row.range(),
        )
        .map_err(|error| M1FullStepWorkspaceDispatchRangeError::Range {
            workspace: M1FullStepWorkspaceRole::Target,
            error,
        })
    }

    fn workspace_dispatch_subrange(
        &self,
        allocations: &ServiceAllocationSessionV1,
        workspace: M1FullStepWorkspaceRole,
        role: M1StepWorkspaceRangeRole,
        range: ferric_build::M1StepWorkspaceRange,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1StepWorkspaceDispatchRangeError> {
        match (&self.owners, workspace) {
            (
                M1FullStepWorkspaceSubleaseOwners::TargetOnly { target }
                | M1FullStepWorkspaceSubleaseOwners::PairedPrefill { target, .. },
                M1FullStepWorkspaceRole::Target,
            ) => target.dispatch_subrange(allocations, role, range),
            (
                M1FullStepWorkspaceSubleaseOwners::PairedPrefill { draft, .. },
                M1FullStepWorkspaceRole::Draft,
            ) => draft.dispatch_subrange(allocations, role, range),
            (
                M1FullStepWorkspaceSubleaseOwners::SpeculativeRound { draft_decode, .. },
                M1FullStepWorkspaceRole::Draft,
            ) => draft_decode.dispatch_subrange(allocations, role, range),
            (
                M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                    target_speculative, ..
                },
                M1FullStepWorkspaceRole::Target,
            ) => target_speculative.dispatch_subrange(allocations, role, range),
            _ => Err(M1StepWorkspaceDispatchRangeError::InactiveRole { role }),
        }
    }

    fn owner_plan(
        &self,
        workspace: M1FullStepWorkspaceRole,
    ) -> Option<&AddresslessM1StepWorkspacePlan> {
        match (&self.owners, workspace) {
            (
                M1FullStepWorkspaceSubleaseOwners::TargetOnly { target }
                | M1FullStepWorkspaceSubleaseOwners::PairedPrefill { target, .. },
                M1FullStepWorkspaceRole::Target,
            ) => Some(target.plan()),
            (
                M1FullStepWorkspaceSubleaseOwners::PairedPrefill { draft, .. },
                M1FullStepWorkspaceRole::Draft,
            ) => Some(draft.plan()),
            (
                M1FullStepWorkspaceSubleaseOwners::SpeculativeRound { draft_decode, .. },
                M1FullStepWorkspaceRole::Draft,
            ) => Some(draft_decode.plan()),
            (
                M1FullStepWorkspaceSubleaseOwners::SpeculativeRound {
                    target_speculative, ..
                },
                M1FullStepWorkspaceRole::Target,
            ) => Some(target_speculative.plan()),
            _ => None,
        }
    }
}

/// Joins one full-step composition to exact already-bound workspace owners.
///
/// All composition, owner-shape, plan, segment, selection, identity, and
/// speculative-row checks complete before generic allocation generations are
/// revalidated. Rejection returns the exact unchanged composition and owners.
///
/// # Errors
///
/// Returns [`M1FullStepWorkspaceSubleaseBindingFailure`] for any structural,
/// identity, selection, subrange, retained-partition, or allocation-generation
/// mismatch. The failure retains every exact unchanged linear input.
pub fn bind_addressless_m1_full_step_workspace_subleases(
    composition: AddresslessM1FullStepWorkspaceComposition,
    owners: M1FullStepWorkspaceSubleaseOwners,
    allocations: &ServiceAllocationSessionV1,
) -> Result<BoundM1FullStepWorkspaceSubleases, M1FullStepWorkspaceSubleaseBindingFailure> {
    let result = validate_bound_full_step_workspace_metadata(&composition, owners.metadata())
        .and_then(|()| owners.revalidate(allocations));
    match result {
        Ok(()) => Ok(BoundM1FullStepWorkspaceSubleases {
            composition,
            owners,
        }),
        Err(error) => Err(M1FullStepWorkspaceSubleaseBindingFailure {
            error,
            composition: Box::new(composition),
            owners: Box::new(owners),
        }),
    }
}

#[derive(Clone, Copy)]
struct WorkspaceOwnerMetadata<'a> {
    plan: &'a AddresslessM1StepWorkspacePlan,
    member_count: usize,
}

impl<'a> WorkspaceOwnerMetadata<'a> {
    const fn new(plan: &'a AddresslessM1StepWorkspacePlan, member_count: usize) -> Self {
        Self { plan, member_count }
    }
}

#[derive(Clone, Copy)]
enum M1FullStepWorkspaceOwnerMetadata<'a> {
    TargetOnly {
        target: WorkspaceOwnerMetadata<'a>,
    },
    PairedPrefill {
        draft: WorkspaceOwnerMetadata<'a>,
        target: WorkspaceOwnerMetadata<'a>,
    },
    SpeculativeRound {
        draft: WorkspaceOwnerMetadata<'a>,
        target: WorkspaceOwnerMetadata<'a>,
    },
}

impl M1FullStepWorkspaceOwnerMetadata<'_> {
    const fn kind(self) -> M1FullStepWorkspaceInputKind {
        match self {
            Self::TargetOnly { .. } => M1FullStepWorkspaceInputKind::TargetOnly,
            Self::PairedPrefill { .. } => M1FullStepWorkspaceInputKind::PairedPrefill,
            Self::SpeculativeRound { .. } => M1FullStepWorkspaceInputKind::SpeculativeRound,
        }
    }
}

fn validate_bound_full_step_workspace_metadata(
    composition: &AddresslessM1FullStepWorkspaceComposition,
    owners: M1FullStepWorkspaceOwnerMetadata<'_>,
) -> Result<(), M1FullStepWorkspaceSubleaseBindingError> {
    let expected_kind = composition.workspace_plans().kind();
    let actual_kind = owners.kind();
    if expected_kind != actual_kind {
        return Err(M1FullStepWorkspaceSubleaseBindingError::OwnerInputKind {
            expected: expected_kind,
            actual: actual_kind,
        });
    }

    match owners {
        M1FullStepWorkspaceOwnerMetadata::TargetOnly { target } => validate_workspace_owner(
            M1FullStepWorkspaceRole::Target,
            composition.workspace_plans().target(),
            target,
        )?,
        M1FullStepWorkspaceOwnerMetadata::PairedPrefill { draft, target }
        | M1FullStepWorkspaceOwnerMetadata::SpeculativeRound { draft, target } => {
            validate_workspace_owner(
                M1FullStepWorkspaceRole::Draft,
                composition.workspace_plans().draft().ok_or(
                    M1FullStepWorkspaceSubleaseBindingError::OwnerInputKind {
                        expected: expected_kind,
                        actual: actual_kind,
                    },
                )?,
                draft,
            )?;
            validate_workspace_owner(
                M1FullStepWorkspaceRole::Target,
                composition.workspace_plans().target(),
                target,
            )?;
        }
    }

    let segments = composition.dispatch_plan().segments();
    let bindings = composition.segment_bindings();
    if segments.len() != bindings.len() {
        return Err(M1FullStepWorkspaceSubleaseBindingError::SegmentCount {
            expected: segments.len(),
            actual: bindings.len(),
        });
    }
    for (position, (segment, binding)) in segments.iter().zip(bindings).enumerate() {
        let owner = match binding.workspace_role() {
            M1FullStepWorkspaceRole::Target => owners_target(owners),
            M1FullStepWorkspaceRole::Draft => owners_draft(owners).ok_or(
                M1FullStepWorkspaceSubleaseBindingError::SegmentWorkspaceRole { position },
            )?,
        };
        validate_segment_workspace_binding(
            position,
            segment.segment_index(),
            segment.stage(),
            segment.selection(),
            binding.segment_index(),
            binding.workspace_role(),
            binding.workspace_id(),
            binding.workspace_selection(),
            binding.draft_choice_subrange(),
            binding.draft_position_ids_subrange(),
            binding.draft_context_lengths_subrange(),
            owner,
            Some(owners_target(owners)),
        )?;
    }
    Ok(())
}

fn owners_target(owners: M1FullStepWorkspaceOwnerMetadata<'_>) -> WorkspaceOwnerMetadata<'_> {
    match owners {
        M1FullStepWorkspaceOwnerMetadata::TargetOnly { target }
        | M1FullStepWorkspaceOwnerMetadata::PairedPrefill { target, .. }
        | M1FullStepWorkspaceOwnerMetadata::SpeculativeRound { target, .. } => target,
    }
}

fn owners_draft(
    owners: M1FullStepWorkspaceOwnerMetadata<'_>,
) -> Option<WorkspaceOwnerMetadata<'_>> {
    match owners {
        M1FullStepWorkspaceOwnerMetadata::TargetOnly { .. } => None,
        M1FullStepWorkspaceOwnerMetadata::PairedPrefill { draft, .. }
        | M1FullStepWorkspaceOwnerMetadata::SpeculativeRound { draft, .. } => Some(draft),
    }
}

fn validate_workspace_owner(
    workspace: M1FullStepWorkspaceRole,
    expected: &AddresslessM1StepWorkspacePlan,
    actual: WorkspaceOwnerMetadata<'_>,
) -> Result<(), M1FullStepWorkspaceSubleaseBindingError> {
    if actual.plan.selection() != expected.selection() {
        return Err(M1FullStepWorkspaceSubleaseBindingError::WorkspaceSelection { workspace });
    }
    if actual.plan.workspace_id() != expected.workspace_id() {
        return Err(M1FullStepWorkspaceSubleaseBindingError::WorkspaceIdentity { workspace });
    }
    if actual.plan.allocation().allocation_id() != expected.allocation().allocation_id() {
        return Err(
            M1FullStepWorkspaceSubleaseBindingError::WorkspaceAllocationIdentity { workspace },
        );
    }
    if actual.plan != expected {
        return Err(M1FullStepWorkspaceSubleaseBindingError::WorkspacePlan { workspace });
    }
    if actual.member_count != expected.ranges().len() {
        return Err(
            M1FullStepWorkspaceSubleaseBindingError::WorkspaceMemberCount {
                workspace,
                expected: expected.ranges().len(),
                actual: actual.member_count,
            },
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_segment_workspace_binding(
    position: usize,
    segment_index: u8,
    stage: M1StepDispatchStage,
    segment_selection: ferric_spec::Qwen3PlanSelection,
    binding_index: u8,
    binding_role: M1FullStepWorkspaceRole,
    binding_workspace_id: ferric_spec::Identity,
    binding_selection: ferric_spec::Qwen3PlanSelection,
    draft_choice: Option<M1SpeculativeDraftChoiceSubrange>,
    draft_position_ids: Option<M1SpeculativeDraftMetadataSubrange>,
    draft_context_lengths: Option<M1SpeculativeDraftMetadataSubrange>,
    owner: WorkspaceOwnerMetadata<'_>,
    target: Option<WorkspaceOwnerMetadata<'_>>,
) -> Result<(), M1FullStepWorkspaceSubleaseBindingError> {
    let expected_index = u8::try_from(position)
        .map_err(|_| M1FullStepWorkspaceSubleaseBindingError::SegmentIndex { position })?;
    if segment_index != expected_index || binding_index != expected_index {
        return Err(M1FullStepWorkspaceSubleaseBindingError::SegmentIndex { position });
    }
    let expected_role = match stage {
        M1StepDispatchStage::TargetOnly
        | M1StepDispatchStage::TargetPrefill
        | M1StepDispatchStage::TargetVerification { .. } => M1FullStepWorkspaceRole::Target,
        M1StepDispatchStage::DraftPrefill | M1StepDispatchStage::DraftDecode { .. } => {
            M1FullStepWorkspaceRole::Draft
        }
    };
    if binding_role != expected_role {
        return Err(M1FullStepWorkspaceSubleaseBindingError::SegmentWorkspaceRole { position });
    }
    if binding_workspace_id != owner.plan.workspace_id() {
        return Err(M1FullStepWorkspaceSubleaseBindingError::SegmentWorkspaceIdentity { position });
    }
    if segment_selection != owner.plan.selection() || binding_selection != segment_selection {
        return Err(M1FullStepWorkspaceSubleaseBindingError::SegmentSelection { position });
    }
    if let M1StepDispatchStage::DraftDecode { iteration } = stage {
        let choice = draft_choice.ok_or(
            M1FullStepWorkspaceSubleaseBindingError::MissingDraftChoiceSubrange { position },
        )?;
        let position_ids = draft_position_ids.ok_or(
            M1FullStepWorkspaceSubleaseBindingError::MissingDraftMetadataSubrange {
                position,
                role: M1StepWorkspaceRangeRole::DraftPositionIds,
            },
        )?;
        let context_lengths = draft_context_lengths.ok_or(
            M1FullStepWorkspaceSubleaseBindingError::MissingDraftMetadataSubrange {
                position,
                role: M1StepWorkspaceRangeRole::DraftContextLengths,
            },
        )?;
        validate_draft_choice_subrange(position, segment_index, iteration, choice, target)?;
        validate_draft_metadata_subrange(
            position,
            segment_index,
            iteration,
            M1StepWorkspaceRangeRole::DraftPositionIds,
            position_ids,
            target,
        )?;
        return validate_draft_metadata_subrange(
            position,
            segment_index,
            iteration,
            M1StepWorkspaceRangeRole::DraftContextLengths,
            context_lengths,
            target,
        );
    }
    if draft_choice.is_some() {
        return Err(
            M1FullStepWorkspaceSubleaseBindingError::UnexpectedDraftChoiceSubrange { position },
        );
    }
    if draft_position_ids.is_some() {
        return Err(
            M1FullStepWorkspaceSubleaseBindingError::UnexpectedDraftMetadataSubrange {
                position,
                role: M1StepWorkspaceRangeRole::DraftPositionIds,
            },
        );
    }
    if draft_context_lengths.is_some() {
        return Err(
            M1FullStepWorkspaceSubleaseBindingError::UnexpectedDraftMetadataSubrange {
                position,
                role: M1StepWorkspaceRangeRole::DraftContextLengths,
            },
        );
    }
    Ok(())
}

fn validate_draft_choice_subrange(
    position: usize,
    segment_index: u8,
    iteration: u8,
    row: M1SpeculativeDraftChoiceSubrange,
    target: Option<WorkspaceOwnerMetadata<'_>>,
) -> Result<(), M1FullStepWorkspaceSubleaseBindingError> {
    let target =
        target.ok_or(M1FullStepWorkspaceSubleaseBindingError::DraftChoiceSubrange { position })?;
    let parent = target
        .plan
        .range(M1StepWorkspaceRangeRole::DraftChoices)
        .ok_or(M1FullStepWorkspaceSubleaseBindingError::DraftChoiceSubrange { position })?;
    validate_draft_choice_subrange_fields(
        position,
        segment_index,
        iteration,
        row.producer_segment(),
        row.iteration(),
        row.target_workspace_id(),
        row.target_allocation_id(),
        row.range(),
        target,
        parent,
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_draft_choice_subrange_fields(
    position: usize,
    segment_index: u8,
    iteration: u8,
    producer_segment: u8,
    row_iteration: u8,
    target_workspace_id: ferric_spec::Identity,
    target_allocation_id: ferric_spec::Identity,
    range: ferric_build::M1StepWorkspaceRange,
    target: WorkspaceOwnerMetadata<'_>,
    parent: ferric_build::M1StepWorkspaceRange,
) -> Result<(), M1FullStepWorkspaceSubleaseBindingError> {
    let parent_end = parent
        .checked_end()
        .ok_or(M1FullStepWorkspaceSubleaseBindingError::DraftChoiceSubrange { position })?;
    let range_end = range
        .checked_end()
        .ok_or(M1FullStepWorkspaceSubleaseBindingError::DraftChoiceSubrange { position })?;
    if producer_segment != segment_index
        || row_iteration != iteration
        || target_workspace_id != target.plan.workspace_id()
        || target_allocation_id != target.plan.allocation().allocation_id()
        || range.role() != M1StepWorkspaceRangeRole::DraftChoices
        || range.byte_len() == 0
        || range.alignment() != parent.alignment()
        || !range.offset().is_multiple_of(range.alignment())
        || range.offset() < parent.offset()
        || range_end > parent_end
    {
        return Err(M1FullStepWorkspaceSubleaseBindingError::DraftChoiceSubrange { position });
    }
    Ok(())
}

fn validate_draft_metadata_subrange(
    position: usize,
    segment_index: u8,
    iteration: u8,
    expected_role: M1StepWorkspaceRangeRole,
    row: M1SpeculativeDraftMetadataSubrange,
    target: Option<WorkspaceOwnerMetadata<'_>>,
) -> Result<(), M1FullStepWorkspaceSubleaseBindingError> {
    let error = || M1FullStepWorkspaceSubleaseBindingError::DraftMetadataSubrange {
        position,
        role: expected_role,
    };
    let target = target.ok_or_else(error)?;
    let parent = target.plan.range(expected_role).ok_or_else(error)?;
    let target_selection = target.plan.selection();
    let dimensions = target_selection
        .bucket
        .dimensions(target_selection.role, target_selection.mode)
        .ok_or_else(error)?;
    let draft_iterations = dimensions.active_tokens.checked_sub(1).ok_or_else(error)?;
    u32::from(iteration)
        .checked_add(1)
        .filter(|current| *current <= draft_iterations)
        .ok_or_else(error)?;
    let row_bytes = u64::from(dimensions.sequences)
        .checked_mul(U32_BYTES)
        .ok_or_else(error)?;
    let expected_offset = parent
        .offset()
        .checked_add(
            u64::from(iteration)
                .checked_mul(row_bytes)
                .ok_or_else(error)?,
        )
        .ok_or_else(error)?;
    let expected_total = row_bytes
        .checked_mul(u64::from(draft_iterations))
        .ok_or_else(error)?;
    let range = row.range();
    let range_end = range.checked_end().ok_or_else(error)?;
    let parent_end = parent.checked_end().ok_or_else(error)?;
    if target_selection.role != ferric_spec::Qwen3ModelRole::Target8B
        || target_selection.mode != ferric_spec::Qwen3ExecutionMode::Speculative
        || row.draft_segment() != segment_index
        || row.iteration() != iteration
        || row.sequence_count() != dimensions.sequences
        || row.target_workspace_id() != target.plan.workspace_id()
        || row.target_workspace_selection() != target_selection
        || row.target_allocation_id() != target.plan.allocation().allocation_id()
        || parent.byte_len() != expected_total
        || parent.alignment() != U32_BYTES
        || range.role() != expected_role
        || range.offset() != expected_offset
        || range.byte_len() != row_bytes
        || range.alignment() != U32_BYTES
        || !range.offset().is_multiple_of(U32_BYTES)
        || range.offset() < parent.offset()
        || range_end > parent_end
        || range_end > target.plan.allocation().byte_len()
    {
        return Err(error());
    }
    Ok(())
}

fn allocation_error(
    workspace: M1FullStepWorkspaceRole,
    error: ServiceAllocationErrorV1,
) -> M1FullStepWorkspaceSubleaseBindingError {
    M1FullStepWorkspaceSubleaseBindingError::Allocation { workspace, error }
}

#[cfg(test)]
mod tests {
    use ferric_build::{
        m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
        AvailableM1StepWorkspace, DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration,
        M1StepWorkspacePlanOutcome, M1StepWorkspaceRange,
    };
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
    };

    use super::*;
    use crate::operation_kernel_plan::tests::public_operation_kernel_plan_fixture;
    use crate::{
        compose_addressless_m1_full_step_workspaces, derive_m1_step_dispatch_plan,
        M1FullStepWorkspaceCompositionOutcome, M1FullStepWorkspacePlans, M1StepDispatchIntent,
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
    const SPECULATIVE_CASES: [(Qwen3PlanBucket, Qwen3PlanBucket); 4] = [
        (
            Qwen3PlanBucket::SpeculativeS1K4C8192,
            Qwen3PlanBucket::DecodeS1C8192,
        ),
        (
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3PlanBucket::DecodeS8C8192,
        ),
        (
            Qwen3PlanBucket::SpeculativeS1K8C8192,
            Qwen3PlanBucket::DecodeS1C8192,
        ),
        (
            Qwen3PlanBucket::SpeculativeS1K16C8192,
            Qwen3PlanBucket::DecodeS1C8192,
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
        let declaration = M1StepWorkspaceDeclaration::new(
            selection,
            DeclaredM1StepWorkspaceAllocation::new(
                Identity::new([identity_byte; 32]),
                requirements.allocation_byte_len(),
                requirements.allocation_alignment(),
            ),
            requirements.ranges().to_vec().into_boxed_slice(),
        );
        match plan_addressless_m1_step_workspace(
            selection,
            AvailableM1StepWorkspace::new(declaration),
        ) {
            M1StepWorkspacePlanOutcome::Planned(plan) => plan,
            M1StepWorkspacePlanOutcome::Rejected(_) => panic!("exact workspace fixture rejected"),
        }
    }

    fn composed(
        outcome: M1FullStepWorkspaceCompositionOutcome,
    ) -> AddresslessM1FullStepWorkspaceComposition {
        match outcome {
            M1FullStepWorkspaceCompositionOutcome::Composed(composition) => composition,
            M1FullStepWorkspaceCompositionOutcome::Rejected(failure) => {
                panic!("exact composition rejected: {:?}", failure.error())
            }
        }
    }

    #[test]
    fn all_15_complete_intent_bucket_shapes_join_exact_owner_metadata() {
        let operation_plan = public_operation_kernel_plan_fixture();

        let target_only = PREFILL_BUCKETS
            .into_iter()
            .map(|bucket| target(Qwen3ExecutionMode::Prefill, bucket))
            .chain(
                DECODE_BUCKETS
                    .into_iter()
                    .map(|bucket| target(Qwen3ExecutionMode::Decode, bucket)),
            );
        let mut case_count = 0;
        for target_selection in target_only {
            let identity_byte = 10 + case_count;
            let dispatch = derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::TargetOnly(target_selection),
            )
            .unwrap();
            let composition = composed(compose_addressless_m1_full_step_workspaces(
                dispatch,
                M1FullStepWorkspacePlans::target_only(exact_workspace_plan(
                    target_selection,
                    identity_byte,
                )),
            ));
            let target_owner = exact_workspace_plan(target_selection, identity_byte);
            validate_bound_full_step_workspace_metadata(
                &composition,
                M1FullStepWorkspaceOwnerMetadata::TargetOnly {
                    target: WorkspaceOwnerMetadata::new(
                        &target_owner,
                        M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
                    ),
                },
            )
            .unwrap();
            case_count += 1;
        }

        for bucket in PREFILL_BUCKETS {
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
            let composition = composed(compose_addressless_m1_full_step_workspaces(
                dispatch,
                M1FullStepWorkspacePlans::paired_prefill(
                    exact_workspace_plan(draft_selection, 40),
                    exact_workspace_plan(target_selection, 41),
                ),
            ));
            let draft_owner = exact_workspace_plan(draft_selection, 40);
            let target_owner = exact_workspace_plan(target_selection, 41);
            validate_bound_full_step_workspace_metadata(
                &composition,
                M1FullStepWorkspaceOwnerMetadata::PairedPrefill {
                    draft: WorkspaceOwnerMetadata::new(
                        &draft_owner,
                        M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
                    ),
                    target: WorkspaceOwnerMetadata::new(
                        &target_owner,
                        M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
                    ),
                },
            )
            .unwrap();
            case_count += 1;
        }

        for (target_bucket, draft_bucket) in SPECULATIVE_CASES {
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
            let composition = composed(compose_addressless_m1_full_step_workspaces(
                dispatch,
                M1FullStepWorkspacePlans::speculative_round(
                    exact_workspace_plan(draft_selection, 50),
                    exact_workspace_plan(target_selection, 51),
                ),
            ));
            let draft_owner = exact_workspace_plan(draft_selection, 50);
            let target_owner = exact_workspace_plan(target_selection, 51);
            let dimensions = target_bucket
                .dimensions(Qwen3ModelRole::Target8B, Qwen3ExecutionMode::Speculative)
                .unwrap();
            let iterations = u8::try_from(dimensions.active_tokens - 1).unwrap();
            let row_bytes = u64::from(dimensions.sequences) * U32_BYTES;
            let position_parent = target_owner
                .range(M1StepWorkspaceRangeRole::DraftPositionIds)
                .unwrap();
            let context_parent = target_owner
                .range(M1StepWorkspaceRangeRole::DraftContextLengths)
                .unwrap();
            let target_metadata = WorkspaceOwnerMetadata::new(
                &target_owner,
                M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            );
            for iteration in 0..iterations {
                let binding = composition.segment_binding(iteration).unwrap();
                let position_ids = binding.draft_position_ids_subrange().unwrap();
                let context_lengths = binding.draft_context_lengths_subrange().unwrap();
                for (role, row, parent) in [
                    (
                        M1StepWorkspaceRangeRole::DraftPositionIds,
                        position_ids,
                        position_parent,
                    ),
                    (
                        M1StepWorkspaceRangeRole::DraftContextLengths,
                        context_lengths,
                        context_parent,
                    ),
                ] {
                    validate_draft_metadata_subrange(
                        usize::from(iteration),
                        iteration,
                        iteration,
                        role,
                        row,
                        Some(target_metadata),
                    )
                    .unwrap();
                    assert_eq!(row.range().role(), role);
                    assert_eq!(row.range().byte_len(), row_bytes);
                    assert_eq!(
                        row.range().offset(),
                        parent.offset() + u64::from(iteration) * row_bytes
                    );
                }
            }
            if target_bucket == Qwen3PlanBucket::SpeculativeS8K4C8192 {
                assert_eq!(iterations, 4);
                assert_eq!(row_bytes, 32);
                assert_eq!(position_parent.byte_len(), 128);
                assert_eq!(context_parent.byte_len(), 128);
            }
            validate_bound_full_step_workspace_metadata(
                &composition,
                M1FullStepWorkspaceOwnerMetadata::SpeculativeRound {
                    draft: WorkspaceOwnerMetadata::new(
                        &draft_owner,
                        M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
                    ),
                    target: WorkspaceOwnerMetadata::new(
                        &target_owner,
                        M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
                    ),
                },
            )
            .unwrap();
            case_count += 1;
        }
        assert_eq!(case_count, 15);
    }

    #[test]
    fn wrong_owner_shape_selection_identity_and_member_count_fail_closed() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let expected = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let dispatch = derive_m1_step_dispatch_plan(
            &operation_plan,
            M1StepDispatchIntent::TargetOnly(expected),
        )
        .unwrap();
        let composition = composed(compose_addressless_m1_full_step_workspaces(
            dispatch,
            M1FullStepWorkspacePlans::target_only(exact_workspace_plan(expected, 60)),
        ));
        let exact_target = exact_workspace_plan(expected, 60);
        let draft = exact_workspace_plan(
            selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            61,
        );
        assert!(matches!(
            validate_bound_full_step_workspace_metadata(
                &composition,
                M1FullStepWorkspaceOwnerMetadata::PairedPrefill {
                    draft: WorkspaceOwnerMetadata::new(
                        &draft,
                        M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1
                    ),
                    target: WorkspaceOwnerMetadata::new(
                        &exact_target,
                        M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1
                    ),
                },
            ),
            Err(M1FullStepWorkspaceSubleaseBindingError::OwnerInputKind { .. })
        ));

        let wrong_selection = exact_workspace_plan(
            target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS8C8192),
            60,
        );
        let wrong_identity = exact_workspace_plan(expected, 62);
        for (case, owner) in [
            WorkspaceOwnerMetadata::new(
                &wrong_selection,
                M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            ),
            WorkspaceOwnerMetadata::new(
                &wrong_identity,
                M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            ),
            WorkspaceOwnerMetadata::new(
                &exact_target,
                M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1 - 1,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let error = validate_bound_full_step_workspace_metadata(
                &composition,
                M1FullStepWorkspaceOwnerMetadata::TargetOnly { target: owner },
            )
            .unwrap_err();
            assert!(matches!(
                (case, error),
                (
                    0,
                    M1FullStepWorkspaceSubleaseBindingError::WorkspaceSelection { .. }
                ) | (
                    1,
                    M1FullStepWorkspaceSubleaseBindingError::WorkspaceIdentity { .. }
                ) | (
                    2,
                    M1FullStepWorkspaceSubleaseBindingError::WorkspaceMemberCount { .. }
                )
            ));
        }
    }

    #[test]
    fn hostile_segment_identity_selection_role_and_index_are_rejected() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let target_selection = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let dispatch = derive_m1_step_dispatch_plan(
            &operation_plan,
            M1StepDispatchIntent::TargetOnly(target_selection),
        )
        .unwrap();
        let composition = composed(compose_addressless_m1_full_step_workspaces(
            dispatch,
            M1FullStepWorkspacePlans::target_only(exact_workspace_plan(target_selection, 70)),
        ));
        let owner_plan = exact_workspace_plan(target_selection, 70);
        let owner =
            WorkspaceOwnerMetadata::new(&owner_plan, M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1);
        let segment = &composition.dispatch_plan().segments()[0];
        let binding = composition.segment_bindings()[0];
        let exact = || {
            validate_segment_workspace_binding(
                0,
                segment.segment_index(),
                segment.stage(),
                segment.selection(),
                binding.segment_index(),
                binding.workspace_role(),
                binding.workspace_id(),
                binding.workspace_selection(),
                binding.draft_choice_subrange(),
                binding.draft_position_ids_subrange(),
                binding.draft_context_lengths_subrange(),
                owner,
                Some(owner),
            )
        };
        exact().unwrap();
        assert!(matches!(
            validate_segment_workspace_binding(
                0,
                1,
                segment.stage(),
                segment.selection(),
                binding.segment_index(),
                binding.workspace_role(),
                binding.workspace_id(),
                binding.workspace_selection(),
                None,
                None,
                None,
                owner,
                Some(owner),
            ),
            Err(M1FullStepWorkspaceSubleaseBindingError::SegmentIndex { .. })
        ));
        assert!(matches!(
            validate_segment_workspace_binding(
                0,
                0,
                segment.stage(),
                segment.selection(),
                0,
                M1FullStepWorkspaceRole::Draft,
                binding.workspace_id(),
                binding.workspace_selection(),
                None,
                None,
                None,
                owner,
                Some(owner),
            ),
            Err(M1FullStepWorkspaceSubleaseBindingError::SegmentWorkspaceRole { .. })
        ));
        assert!(matches!(
            validate_segment_workspace_binding(
                0,
                0,
                segment.stage(),
                segment.selection(),
                0,
                binding.workspace_role(),
                Identity::new([99; 32]),
                binding.workspace_selection(),
                None,
                None,
                None,
                owner,
                Some(owner),
            ),
            Err(M1FullStepWorkspaceSubleaseBindingError::SegmentWorkspaceIdentity { .. })
        ));
        assert!(matches!(
            validate_segment_workspace_binding(
                0,
                0,
                segment.stage(),
                segment.selection(),
                0,
                binding.workspace_role(),
                binding.workspace_id(),
                selection(
                    Qwen3ModelRole::Draft06B,
                    Qwen3ExecutionMode::Decode,
                    Qwen3PlanBucket::DecodeS1C8192,
                ),
                None,
                None,
                None,
                owner,
                Some(owner),
            ),
            Err(M1FullStepWorkspaceSubleaseBindingError::SegmentSelection { .. })
        ));
    }

    #[test]
    fn speculative_choice_rows_reject_wrong_iteration_identity_role_and_bounds() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let target_selection = target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let draft_selection = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let dispatch = derive_m1_step_dispatch_plan(
            &operation_plan,
            M1StepDispatchIntent::SpeculativeRound(target_selection),
        )
        .unwrap();
        let composition = composed(compose_addressless_m1_full_step_workspaces(
            dispatch,
            M1FullStepWorkspacePlans::speculative_round(
                exact_workspace_plan(draft_selection, 80),
                exact_workspace_plan(target_selection, 81),
            ),
        ));
        let target_plan = exact_workspace_plan(target_selection, 81);
        let target_owner = WorkspaceOwnerMetadata::new(
            &target_plan,
            M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
        );
        let row = composition.segment_bindings()[0]
            .draft_choice_subrange()
            .unwrap();
        let parent = target_plan
            .range(M1StepWorkspaceRangeRole::DraftChoices)
            .unwrap();
        let validate =
            |producer_segment, iteration, target_workspace_id, range: M1StepWorkspaceRange| {
                validate_draft_choice_subrange_fields(
                    0,
                    0,
                    0,
                    producer_segment,
                    iteration,
                    target_workspace_id,
                    row.target_allocation_id(),
                    range,
                    target_owner,
                    parent,
                )
            };
        validate(0, 0, row.target_workspace_id(), row.range()).unwrap();
        for (case, result) in [
            validate(1, 0, row.target_workspace_id(), row.range()),
            validate(0, 1, row.target_workspace_id(), row.range()),
            validate(0, 0, Identity::new([82; 32]), row.range()),
            validate(
                0,
                0,
                row.target_workspace_id(),
                M1StepWorkspaceRange::new(
                    M1StepWorkspaceRangeRole::Choices,
                    row.range().offset(),
                    row.range().byte_len(),
                    row.range().alignment(),
                ),
            ),
            validate(
                0,
                0,
                row.target_workspace_id(),
                M1StepWorkspaceRange::new(
                    M1StepWorkspaceRangeRole::DraftChoices,
                    parent.checked_end().unwrap(),
                    row.range().byte_len(),
                    row.range().alignment(),
                ),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                matches!(
                    result,
                    Err(
                        M1FullStepWorkspaceSubleaseBindingError::DraftChoiceSubrange {
                            position: 0
                        }
                    )
                ),
                "hostile row case {case} unexpectedly passed"
            );
        }
    }

    #[test]
    fn speculative_metadata_rows_reject_missing_and_swapped_roles() {
        let operation_plan = public_operation_kernel_plan_fixture();
        let target_selection = target(
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS8K4C8192,
        );
        let draft_selection = selection(
            Qwen3ModelRole::Draft06B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        );
        let composition = composed(compose_addressless_m1_full_step_workspaces(
            derive_m1_step_dispatch_plan(
                &operation_plan,
                M1StepDispatchIntent::SpeculativeRound(target_selection),
            )
            .unwrap(),
            M1FullStepWorkspacePlans::speculative_round(
                exact_workspace_plan(draft_selection, 91),
                exact_workspace_plan(target_selection, 92),
            ),
        ));
        let draft_plan = exact_workspace_plan(draft_selection, 91);
        let target_plan = exact_workspace_plan(target_selection, 92);
        let draft_owner =
            WorkspaceOwnerMetadata::new(&draft_plan, M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1);
        let target_owner = WorkspaceOwnerMetadata::new(
            &target_plan,
            M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
        );
        let segment = &composition.dispatch_plan().segments()[0];
        let binding = composition.segment_bindings()[0];
        let choice = binding.draft_choice_subrange();
        let position_ids = binding.draft_position_ids_subrange();
        let context_lengths = binding.draft_context_lengths_subrange();
        let validate = |position_ids, context_lengths| {
            validate_segment_workspace_binding(
                0,
                segment.segment_index(),
                segment.stage(),
                segment.selection(),
                binding.segment_index(),
                binding.workspace_role(),
                binding.workspace_id(),
                binding.workspace_selection(),
                choice,
                position_ids,
                context_lengths,
                draft_owner,
                Some(target_owner),
            )
        };
        validate(position_ids, context_lengths).unwrap();
        assert!(matches!(
            validate(None, context_lengths),
            Err(
                M1FullStepWorkspaceSubleaseBindingError::MissingDraftMetadataSubrange {
                    position: 0,
                    role: M1StepWorkspaceRangeRole::DraftPositionIds,
                }
            )
        ));
        assert!(matches!(
            validate(context_lengths, position_ids),
            Err(
                M1FullStepWorkspaceSubleaseBindingError::DraftMetadataSubrange {
                    position: 0,
                    role: M1StepWorkspaceRangeRole::DraftPositionIds,
                }
            )
        ));
    }

    #[test]
    fn stale_allocation_generation_is_attributed_to_exact_workspace() {
        let error = allocation_error(
            M1FullStepWorkspaceRole::Target,
            ServiceAllocationErrorV1::AllocationGenerationMismatch,
        );
        assert!(matches!(
            error,
            M1FullStepWorkspaceSubleaseBindingError::Allocation {
                workspace: M1FullStepWorkspaceRole::Target,
                error: ServiceAllocationErrorV1::AllocationGenerationMismatch,
            }
        ));
        let dispatch_error = M1FullStepWorkspaceDispatchRangeError::Range {
            workspace: M1FullStepWorkspaceRole::Target,
            error: M1StepWorkspaceDispatchRangeError::Allocation(
                ServiceAllocationErrorV1::AllocationGenerationMismatch,
            ),
        };
        assert!(matches!(
            dispatch_error,
            M1FullStepWorkspaceDispatchRangeError::Range {
                workspace: M1FullStepWorkspaceRole::Target,
                error: M1StepWorkspaceDispatchRangeError::Allocation(
                    ServiceAllocationErrorV1::AllocationGenerationMismatch
                ),
            }
        ));
    }
}
