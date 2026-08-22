//! Runtime binding for one checked M1 step-workspace layout.
//!
//! Workspace semantics and roster selection remain in Ferric. This module joins
//! that addressless declaration to the generic device-workspace sublease API and
//! retains both values together. It constructs no native address or packet and
//! performs no queue operation, publication, completion, readback, hardware
//! qualification, or performance measurement.

use core::fmt;

use fe2o3_service_host::{
    DeviceLocalAllocationV1, DeviceWorkspaceRoleV1, ServiceAllocationErrorV1,
    ServiceAllocationKeyV1, ServiceAllocationSessionV1, ServiceAllocationSubleaseSetV1,
    ServiceDeviceDispatchRangeV1,
};
use ferric_build::{
    m1_step_workspace_requirements, AddresslessM1StepWorkspacePlan,
    DeclaredM1StepWorkspaceAllocation, M1StepWorkspacePlanError, M1StepWorkspaceRange,
    M1StepWorkspaceRangeRole,
};
use ferric_spec::{Identity, Qwen3PlanSelection};

/// Number of workspace members in every draft selection.
pub const M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1: usize = 24;
/// Number of workspace members in a non-speculative target selection.
pub const M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1: usize = 29;
/// Number of workspace members in a speculative target selection.
pub const M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1: usize = 32;

/// Fail-closed workspace-to-sublease binding error.
#[derive(Debug)]
pub enum M1StepWorkspaceSubleaseBindingError {
    /// The plan names another model role.
    SelectionRoleDrift,
    /// The plan names another execution mode.
    SelectionModeDrift,
    /// The plan names another finite bucket.
    SelectionBucketDrift,
    /// The expected selection could not produce deterministic workspace requirements.
    Requirements(M1StepWorkspacePlanError),
    /// The caller-selected allocation identity is absent.
    MissingSelectedAllocationIdentity,
    /// The caller-selected allocation identity differs from the plan declaration.
    AllocationIdentityDrift,
    /// The plan allocation length differs from its deterministic requirement.
    PlanAllocationLengthDrift {
        /// Deterministic required bytes.
        expected: u64,
        /// Rejected plan bytes.
        actual: u64,
    },
    /// The plan allocation alignment differs from its deterministic requirement.
    PlanAllocationAlignmentDrift {
        /// Deterministic required alignment.
        expected: u64,
        /// Rejected plan alignment.
        actual: u64,
    },
    /// The selected runtime allocation has a different exact length.
    RuntimeAllocationLengthDrift {
        /// Planned bytes.
        expected: u64,
        /// Rejected runtime allocation bytes.
        actual: u64,
    },
    /// The selected runtime allocation has a different base alignment.
    RuntimeAllocationAlignmentDrift {
        /// Planned base alignment.
        expected: u64,
        /// Rejected runtime allocation alignment.
        actual: u64,
    },
    /// The plan roster differs in length from deterministic requirements.
    PlanRangeCountDrift {
        /// Deterministic member count.
        expected: usize,
        /// Rejected plan member count.
        actual: usize,
    },
    /// The compile-time member count differs from the exact plan roster.
    ConstMemberCountDrift {
        /// Exact plan member count.
        expected: usize,
        /// Rejected compile-time count.
        actual: usize,
    },
    /// A plan member is empty.
    EmptyRange {
        /// Rejected roster position.
        index: usize,
        /// Rejected workspace role.
        role: M1StepWorkspaceRangeRole,
    },
    /// A plan member has an invalid or unsatisfied alignment.
    RangeAlignmentDrift {
        /// Rejected roster position.
        index: usize,
        /// Rejected workspace role.
        role: M1StepWorkspaceRangeRole,
    },
    /// A plan member end overflowed `u64`.
    RangeOverflow {
        /// Rejected roster position.
        index: usize,
        /// Rejected workspace role.
        role: M1StepWorkspaceRangeRole,
    },
    /// A plan member exceeds the selected runtime allocation.
    RangeOutOfBounds {
        /// Rejected roster position.
        index: usize,
        /// Rejected workspace role.
        role: M1StepWorkspaceRangeRole,
    },
    /// Two plan members overlap.
    RangeAlias {
        /// Earlier roster position.
        left: usize,
        /// Later roster position.
        right: usize,
    },
    /// A roster position names the wrong semantic role.
    RangeRoleDrift {
        /// Rejected roster position.
        index: usize,
        /// Required role.
        expected: M1StepWorkspaceRangeRole,
        /// Rejected role.
        actual: M1StepWorkspaceRangeRole,
    },
    /// A roster member starts at the wrong byte offset.
    RangeOffsetDrift {
        /// Rejected roster position.
        index: usize,
        /// Required offset.
        expected: u64,
        /// Rejected offset.
        actual: u64,
    },
    /// A roster member has the wrong exact byte length.
    RangeLengthDrift {
        /// Rejected roster position.
        index: usize,
        /// Required bytes.
        expected: u64,
        /// Rejected bytes.
        actual: u64,
    },
    /// The generic allocation owner rejected the atomic reservation.
    Allocation(ServiceAllocationErrorV1),
}

impl fmt::Display for M1StepWorkspaceSubleaseBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 step workspace sublease binding rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1StepWorkspaceSubleaseBindingError {}

/// Failure while resolving one exact role-contained workspace dispatch range.
#[derive(Debug)]
pub enum M1StepWorkspaceDispatchRangeError {
    /// The requested semantic role is inactive in the retained workspace plan.
    InactiveRole {
        /// Requested inactive role.
        role: M1StepWorkspaceRangeRole,
    },
    /// The requested subrange names another semantic role.
    RangeRoleDrift {
        /// Required role.
        expected: M1StepWorkspaceRangeRole,
        /// Rejected role.
        actual: M1StepWorkspaceRangeRole,
    },
    /// The requested subrange is empty.
    EmptyRange {
        /// Requested role.
        role: M1StepWorkspaceRangeRole,
    },
    /// The requested subrange has invalid or unsatisfied alignment.
    RangeAlignment {
        /// Requested role.
        role: M1StepWorkspaceRangeRole,
    },
    /// The requested subrange or retained parent range overflowed `u64`.
    RangeOverflow {
        /// Requested role.
        role: M1StepWorkspaceRangeRole,
    },
    /// The requested subrange is not wholly contained in the retained role member.
    RangeOutOfBounds {
        /// Requested role.
        role: M1StepWorkspaceRangeRole,
    },
    /// The generic allocation owner rejected the retained member or generation.
    Allocation(ServiceAllocationErrorV1),
}

impl fmt::Display for M1StepWorkspaceDispatchRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 step workspace dispatch range rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1StepWorkspaceDispatchRangeError {}

/// Rejected binding retaining the exact unchanged addressless plan.
#[must_use = "the rejected addressless workspace plan remains recoverable"]
#[derive(Debug)]
pub struct M1StepWorkspaceSubleaseBindingFailure {
    error: M1StepWorkspaceSubleaseBindingError,
    plan: Box<AddresslessM1StepWorkspacePlan>,
}

impl M1StepWorkspaceSubleaseBindingFailure {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> &M1StepWorkspaceSubleaseBindingError {
        &self.error
    }

    /// Recovers the diagnostic and exact unchanged addressless plan.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        M1StepWorkspaceSubleaseBindingError,
        AddresslessM1StepWorkspacePlan,
    ) {
        (self.error, *self.plan)
    }
}

/// Ferric custody of one exact plan and its generic logical sublease layout.
///
/// This value intentionally does not implement `Clone`. Native allocation
/// ownership remains in the service allocation session and can still move whole
/// into its queue ledger.
///
/// ```compile_fail
/// use ferric_engine::BoundM1StepWorkspaceSubleases;
/// fn require_clone<T: Clone>() {}
/// require_clone::<BoundM1StepWorkspaceSubleases<24>>();
/// ```
#[must_use = "the addressless workspace plan and logical subleases must remain retained"]
#[derive(Debug)]
pub struct BoundM1StepWorkspaceSubleases<const N: usize> {
    plan: AddresslessM1StepWorkspacePlan,
    roles: [M1StepWorkspaceRangeRole; N],
    subleases: ServiceAllocationSubleaseSetV1<DeviceWorkspaceRoleV1, DeviceLocalAllocationV1, N>,
}

impl<const N: usize> BoundM1StepWorkspaceSubleases<N> {
    /// Returns the retained addressless plan.
    #[must_use]
    pub const fn plan(&self) -> &AddresslessM1StepWorkspacePlan {
        &self.plan
    }

    /// Returns the exact finite selection.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.plan.selection()
    }

    /// Returns the plan's addressless workspace identity.
    #[must_use]
    pub const fn workspace_id(&self) -> Identity {
        self.plan.workspace_id()
    }

    /// Returns the fixed exact member count.
    #[must_use]
    pub const fn member_count(&self) -> usize {
        N
    }

    /// Returns the canonical role at one member index.
    #[must_use]
    pub fn member_role(&self, index: usize) -> Option<M1StepWorkspaceRangeRole> {
        self.roles.get(index).copied()
    }

    /// Returns the canonical member index for one active semantic role.
    #[must_use]
    pub fn member_index(&self, role: M1StepWorkspaceRangeRole) -> Option<usize> {
        self.roles.iter().position(|candidate| *candidate == role)
    }

    /// Revalidates and returns one addressless device dispatch range by role.
    ///
    /// An inactive role returns `Ok(None)`. Allocation or generation drift is
    /// returned unchanged from the generic allocation owner.
    ///
    /// # Errors
    ///
    /// Returns the generic allocation error if the retained sublease layout or
    /// allocation binding no longer matches the supplied owner.
    pub fn dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        role: M1StepWorkspaceRangeRole,
    ) -> Result<Option<ServiceDeviceDispatchRangeV1>, ServiceAllocationErrorV1> {
        let Some(index) = self.member_index(role) else {
            return Ok(None);
        };
        let ranges = allocations.sublease_ranges(&self.subleases)?;
        let Some(range) = ranges.into_iter().nth(index) else {
            return Ok(None);
        };
        allocations.device_dispatch_range(range).map(Some)
    }

    /// Revalidates and resolves one exact absolute subrange inside a role member.
    ///
    /// The returned generic range retains the exact allocation generation and
    /// logical member index recorded by this owner. The requested range is
    /// addressless and its offset is relative to the workspace allocation, not
    /// to the role member.
    ///
    /// # Errors
    ///
    /// Returns [`M1StepWorkspaceDispatchRangeError`] if the role is inactive,
    /// the requested interval is not nonempty, aligned, and wholly contained in
    /// that exact role member, or the generic allocation owner rejects the
    /// retained partition or allocation generation.
    pub fn dispatch_subrange(
        &self,
        allocations: &ServiceAllocationSessionV1,
        role: M1StepWorkspaceRangeRole,
        requested: M1StepWorkspaceRange,
    ) -> Result<ServiceDeviceDispatchRangeV1, M1StepWorkspaceDispatchRangeError> {
        let member_index = self
            .member_index(role)
            .ok_or(M1StepWorkspaceDispatchRangeError::InactiveRole { role })?;
        let parent = self
            .plan
            .range(role)
            .ok_or(M1StepWorkspaceDispatchRangeError::InactiveRole { role })?;
        let relative_offset = validate_role_contained_subrange(role, parent, requested)?;
        let range = allocations
            .sublease_range(
                &self.subleases,
                member_index,
                relative_offset,
                requested.byte_len(),
                requested.alignment(),
            )
            .map_err(M1StepWorkspaceDispatchRangeError::Allocation)?;
        allocations
            .device_dispatch_range(range)
            .map_err(M1StepWorkspaceDispatchRangeError::Allocation)
    }

    pub(crate) fn revalidate_dispatch_ranges(
        &self,
        allocations: &ServiceAllocationSessionV1,
    ) -> Result<(), ServiceAllocationErrorV1> {
        for range in allocations.sublease_ranges(&self.subleases)? {
            allocations.device_dispatch_range(range)?;
        }
        Ok(())
    }
}

fn validate_role_contained_subrange(
    role: M1StepWorkspaceRangeRole,
    parent: M1StepWorkspaceRange,
    requested: M1StepWorkspaceRange,
) -> Result<u64, M1StepWorkspaceDispatchRangeError> {
    if requested.role() != role {
        return Err(M1StepWorkspaceDispatchRangeError::RangeRoleDrift {
            expected: role,
            actual: requested.role(),
        });
    }
    if requested.byte_len() == 0 {
        return Err(M1StepWorkspaceDispatchRangeError::EmptyRange { role });
    }
    if requested.alignment() == 0
        || !requested.alignment().is_power_of_two()
        || requested.alignment() != parent.alignment()
        || !requested.offset().is_multiple_of(requested.alignment())
    {
        return Err(M1StepWorkspaceDispatchRangeError::RangeAlignment { role });
    }
    let parent_end = parent
        .checked_end()
        .ok_or(M1StepWorkspaceDispatchRangeError::RangeOverflow { role })?;
    let requested_end = requested
        .checked_end()
        .ok_or(M1StepWorkspaceDispatchRangeError::RangeOverflow { role })?;
    if requested.offset() < parent.offset() || requested_end > parent_end {
        return Err(M1StepWorkspaceDispatchRangeError::RangeOutOfBounds { role });
    }
    requested
        .offset()
        .checked_sub(parent.offset())
        .ok_or(M1StepWorkspaceDispatchRangeError::RangeOutOfBounds { role })
}

#[derive(Debug)]
struct ValidatedM1StepWorkspaceSubleaseRoster<const N: usize> {
    roles: [M1StepWorkspaceRangeRole; N],
    members: [(u64, u64, u64); N],
}

/// Binds one exact addressless M1 workspace plan to generic typed subleases.
///
/// Selection, caller-selected allocation identity, runtime allocation geometry,
/// deterministic roster, and `N` are checked before the allocation owner is
/// mutated. Rejection returns the unchanged plan. The generic reservation is one
/// atomic call and preserves the whole native allocation owner for queue transfer.
///
/// # Errors
///
/// Returns [`M1StepWorkspaceSubleaseBindingFailure`] for any selection,
/// allocation, roster, cardinality, or generic reservation rejection. The
/// failure retains the unchanged addressless plan.
pub fn bind_addressless_m1_step_workspace_subleases<const N: usize>(
    expected_selection: Qwen3PlanSelection,
    selected_allocation_id: Identity,
    plan: AddresslessM1StepWorkspacePlan,
    allocations: &mut ServiceAllocationSessionV1,
    workspace_key: ServiceAllocationKeyV1<DeviceWorkspaceRoleV1, DeviceLocalAllocationV1>,
) -> Result<BoundM1StepWorkspaceSubleases<N>, M1StepWorkspaceSubleaseBindingFailure> {
    let (plan, roster) = preflight_m1_step_workspace_subleases::<N>(
        expected_selection,
        selected_allocation_id,
        workspace_key.extent_bytes(),
        workspace_key.alignment(),
        plan,
    )?;
    match allocations.reserve_disjoint_subleases(workspace_key, roster.members) {
        Ok(subleases) => Ok(BoundM1StepWorkspaceSubleases {
            plan,
            roles: roster.roles,
            subleases,
        }),
        Err(error) => Err(M1StepWorkspaceSubleaseBindingFailure {
            error: M1StepWorkspaceSubleaseBindingError::Allocation(error),
            plan: Box::new(plan),
        }),
    }
}

fn preflight_m1_step_workspace_subleases<const N: usize>(
    expected_selection: Qwen3PlanSelection,
    selected_allocation_id: Identity,
    runtime_byte_len: u64,
    runtime_alignment: u64,
    plan: AddresslessM1StepWorkspacePlan,
) -> Result<
    (
        AddresslessM1StepWorkspacePlan,
        ValidatedM1StepWorkspaceSubleaseRoster<N>,
    ),
    M1StepWorkspaceSubleaseBindingFailure,
> {
    match validate_m1_step_workspace_sublease_roster::<N>(
        expected_selection,
        selected_allocation_id,
        runtime_byte_len,
        runtime_alignment,
        plan.selection(),
        plan.allocation(),
        plan.ranges(),
    ) {
        Ok(roster) => Ok((plan, roster)),
        Err(error) => Err(M1StepWorkspaceSubleaseBindingFailure {
            error,
            plan: Box::new(plan),
        }),
    }
}

fn validate_m1_step_workspace_sublease_roster<const N: usize>(
    expected_selection: Qwen3PlanSelection,
    selected_allocation_id: Identity,
    runtime_byte_len: u64,
    runtime_alignment: u64,
    plan_selection: Qwen3PlanSelection,
    plan_allocation: DeclaredM1StepWorkspaceAllocation,
    ranges: &[M1StepWorkspaceRange],
) -> Result<ValidatedM1StepWorkspaceSubleaseRoster<N>, M1StepWorkspaceSubleaseBindingError> {
    if plan_selection.role != expected_selection.role {
        return Err(M1StepWorkspaceSubleaseBindingError::SelectionRoleDrift);
    }
    if plan_selection.mode != expected_selection.mode {
        return Err(M1StepWorkspaceSubleaseBindingError::SelectionModeDrift);
    }
    if plan_selection.bucket != expected_selection.bucket {
        return Err(M1StepWorkspaceSubleaseBindingError::SelectionBucketDrift);
    }
    let requirements = m1_step_workspace_requirements(expected_selection)
        .map_err(M1StepWorkspaceSubleaseBindingError::Requirements)?;
    if !selected_allocation_id.is_present() {
        return Err(M1StepWorkspaceSubleaseBindingError::MissingSelectedAllocationIdentity);
    }
    if selected_allocation_id != plan_allocation.allocation_id() {
        return Err(M1StepWorkspaceSubleaseBindingError::AllocationIdentityDrift);
    }
    if plan_allocation.byte_len() != requirements.allocation_byte_len() {
        return Err(
            M1StepWorkspaceSubleaseBindingError::PlanAllocationLengthDrift {
                expected: requirements.allocation_byte_len(),
                actual: plan_allocation.byte_len(),
            },
        );
    }
    if plan_allocation.alignment() != requirements.allocation_alignment() {
        return Err(
            M1StepWorkspaceSubleaseBindingError::PlanAllocationAlignmentDrift {
                expected: requirements.allocation_alignment(),
                actual: plan_allocation.alignment(),
            },
        );
    }
    if runtime_byte_len != plan_allocation.byte_len() {
        return Err(
            M1StepWorkspaceSubleaseBindingError::RuntimeAllocationLengthDrift {
                expected: plan_allocation.byte_len(),
                actual: runtime_byte_len,
            },
        );
    }
    if runtime_alignment != plan_allocation.alignment() {
        return Err(
            M1StepWorkspaceSubleaseBindingError::RuntimeAllocationAlignmentDrift {
                expected: plan_allocation.alignment(),
                actual: runtime_alignment,
            },
        );
    }
    if ranges.len() != requirements.ranges().len() {
        return Err(M1StepWorkspaceSubleaseBindingError::PlanRangeCountDrift {
            expected: requirements.ranges().len(),
            actual: ranges.len(),
        });
    }
    if N != ranges.len() {
        return Err(M1StepWorkspaceSubleaseBindingError::ConstMemberCountDrift {
            expected: ranges.len(),
            actual: N,
        });
    }

    let mut ends = [0_u64; N];
    for (index, range) in ranges.iter().copied().enumerate() {
        if range.byte_len() == 0 {
            return Err(M1StepWorkspaceSubleaseBindingError::EmptyRange {
                index,
                role: range.role(),
            });
        }
        if range.alignment() == 0
            || !range.alignment().is_power_of_two()
            || range.alignment() > runtime_alignment
            || !range.offset().is_multiple_of(range.alignment())
        {
            return Err(M1StepWorkspaceSubleaseBindingError::RangeAlignmentDrift {
                index,
                role: range.role(),
            });
        }
        let end =
            range
                .checked_end()
                .ok_or(M1StepWorkspaceSubleaseBindingError::RangeOverflow {
                    index,
                    role: range.role(),
                })?;
        if end > runtime_byte_len {
            return Err(M1StepWorkspaceSubleaseBindingError::RangeOutOfBounds {
                index,
                role: range.role(),
            });
        }
        ends[index] = end;
    }
    for left in 0..ranges.len() {
        for right in (left + 1)..ranges.len() {
            if ranges[left].offset() < ends[right] && ranges[right].offset() < ends[left] {
                return Err(M1StepWorkspaceSubleaseBindingError::RangeAlias { left, right });
            }
        }
    }
    for (index, (actual, expected)) in ranges
        .iter()
        .copied()
        .zip(requirements.ranges().iter().copied())
        .enumerate()
    {
        if actual.role() != expected.role() {
            return Err(M1StepWorkspaceSubleaseBindingError::RangeRoleDrift {
                index,
                expected: expected.role(),
                actual: actual.role(),
            });
        }
        if actual.offset() != expected.offset() {
            return Err(M1StepWorkspaceSubleaseBindingError::RangeOffsetDrift {
                index,
                expected: expected.offset(),
                actual: actual.offset(),
            });
        }
        if actual.byte_len() != expected.byte_len() {
            return Err(M1StepWorkspaceSubleaseBindingError::RangeLengthDrift {
                index,
                expected: expected.byte_len(),
                actual: actual.byte_len(),
            });
        }
    }

    Ok(ValidatedM1StepWorkspaceSubleaseRoster {
        roles: core::array::from_fn(|index| ranges[index].role()),
        members: core::array::from_fn(|index| {
            let range = ranges[index];
            (range.offset(), range.byte_len(), range.alignment())
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_build::{
        plan_addressless_m1_step_workspace, AvailableM1StepWorkspace, M1StepWorkspaceDeclaration,
        M1StepWorkspacePlanOutcome,
    };
    use ferric_spec::{Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket};

    const BUCKETS: [(Qwen3ExecutionMode, Qwen3PlanBucket); 11] = [
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

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn identity(seed: u8) -> Identity {
        Identity::new([seed; 32])
    }

    fn exact_plan(selection: Qwen3PlanSelection) -> AddresslessM1StepWorkspacePlan {
        let requirements = m1_step_workspace_requirements(selection).unwrap();
        let available = AvailableM1StepWorkspace::new(M1StepWorkspaceDeclaration::new(
            selection,
            DeclaredM1StepWorkspaceAllocation::new(
                identity(9),
                requirements.allocation_byte_len(),
                requirements.allocation_alignment(),
            ),
            requirements.ranges().to_vec().into_boxed_slice(),
        ));
        let M1StepWorkspacePlanOutcome::Planned(plan) =
            plan_addressless_m1_step_workspace(selection, available)
        else {
            panic!("exact fixture must plan")
        };
        plan
    }

    fn validate<const N: usize>(
        expected_selection: Qwen3PlanSelection,
        selected_allocation_id: Identity,
        runtime_byte_len: u64,
        runtime_alignment: u64,
        plan_selection: Qwen3PlanSelection,
        plan_allocation: DeclaredM1StepWorkspaceAllocation,
        ranges: &[M1StepWorkspaceRange],
    ) -> Result<ValidatedM1StepWorkspaceSubleaseRoster<N>, M1StepWorkspaceSubleaseBindingError>
    {
        validate_m1_step_workspace_sublease_roster::<N>(
            expected_selection,
            selected_allocation_id,
            runtime_byte_len,
            runtime_alignment,
            plan_selection,
            plan_allocation,
            ranges,
        )
    }

    #[test]
    fn exact_target_draft_and_speculative_rosters_have_frozen_cardinality() {
        for (selection, expected) in [
            (
                selection(
                    Qwen3ModelRole::Draft06B,
                    Qwen3ExecutionMode::Decode,
                    Qwen3PlanBucket::DecodeS1C8192,
                ),
                M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            ),
            (
                selection(
                    Qwen3ModelRole::Target8B,
                    Qwen3ExecutionMode::Decode,
                    Qwen3PlanBucket::DecodeS8C8192,
                ),
                M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            ),
            (
                selection(
                    Qwen3ModelRole::Target8B,
                    Qwen3ExecutionMode::Speculative,
                    Qwen3PlanBucket::SpeculativeS1K16C8192,
                ),
                M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            ),
        ] {
            assert_eq!(
                m1_step_workspace_requirements(selection)
                    .unwrap()
                    .ranges()
                    .len(),
                expected
            );
        }
    }

    #[test]
    fn all_22_finite_workspace_plans_cross_exact_bridge_preflight() {
        for role in [Qwen3ModelRole::Target8B, Qwen3ModelRole::Draft06B] {
            for (mode, bucket) in BUCKETS {
                let selection = selection(role, mode, bucket);
                let plan = exact_plan(selection);
                let identity = plan.allocation().allocation_id();
                let byte_len = plan.allocation().byte_len();
                let alignment = plan.allocation().alignment();
                let result = match (role, mode) {
                    (Qwen3ModelRole::Target8B, Qwen3ExecutionMode::Speculative) => {
                        validate::<M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                            selection,
                            identity,
                            byte_len,
                            alignment,
                            plan.selection(),
                            plan.allocation(),
                            plan.ranges(),
                        )
                        .is_ok()
                    }
                    (Qwen3ModelRole::Target8B, _) => {
                        validate::<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                            selection,
                            identity,
                            byte_len,
                            alignment,
                            plan.selection(),
                            plan.allocation(),
                            plan.ranges(),
                        )
                        .is_ok()
                    }
                    (Qwen3ModelRole::Draft06B, _) => {
                        validate::<M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                            selection,
                            identity,
                            byte_len,
                            alignment,
                            plan.selection(),
                            plan.allocation(),
                            plan.ranges(),
                        )
                        .is_ok()
                    }
                };
                assert!(result, "bridge preflight rejected {selection:?}");
            }
        }
    }

    #[test]
    fn exact_roster_preserves_ordered_role_lookup_and_member_geometry() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let plan = exact_plan(selection);
        let roster = validate::<M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
            selection,
            plan.allocation().allocation_id(),
            plan.allocation().byte_len(),
            plan.allocation().alignment(),
            plan.selection(),
            plan.allocation(),
            plan.ranges(),
        )
        .unwrap();
        for (index, range) in plan.ranges().iter().copied().enumerate() {
            assert_eq!(roster.roles[index], range.role());
            assert_eq!(
                roster.members[index],
                (range.offset(), range.byte_len(), range.alignment())
            );
        }
    }

    #[test]
    fn selection_and_const_cardinality_drift_recover_exact_plan() {
        let expected_selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let plan = exact_plan(expected_selection);
        let workspace_id = plan.workspace_id();
        let failure = preflight_m1_step_workspace_subleases::<28>(
            expected_selection,
            plan.allocation().allocation_id(),
            plan.allocation().byte_len(),
            plan.allocation().alignment(),
            plan,
        )
        .unwrap_err();
        assert!(matches!(
            failure.error(),
            M1StepWorkspaceSubleaseBindingError::ConstMemberCountDrift {
                expected: 29,
                actual: 28
            }
        ));
        let (_, recovered) = failure.into_parts();
        assert_eq!(recovered.workspace_id(), workspace_id);

        for (plan_selection, expected_error) in [
            (
                selection(
                    Qwen3ModelRole::Draft06B,
                    Qwen3ExecutionMode::Decode,
                    Qwen3PlanBucket::DecodeS1C8192,
                ),
                1,
            ),
            (
                selection(
                    Qwen3ModelRole::Target8B,
                    Qwen3ExecutionMode::Prefill,
                    Qwen3PlanBucket::PrefillS1T128,
                ),
                2,
            ),
            (
                selection(
                    Qwen3ModelRole::Target8B,
                    Qwen3ExecutionMode::Decode,
                    Qwen3PlanBucket::DecodeS8C8192,
                ),
                3,
            ),
        ] {
            let plan = exact_plan(plan_selection);
            let error = validate::<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                expected_selection,
                plan.allocation().allocation_id(),
                plan.allocation().byte_len(),
                plan.allocation().alignment(),
                plan.selection(),
                plan.allocation(),
                plan.ranges(),
            )
            .unwrap_err();
            assert!(matches!(
                (expected_error, error),
                (1, M1StepWorkspaceSubleaseBindingError::SelectionRoleDrift)
                    | (2, M1StepWorkspaceSubleaseBindingError::SelectionModeDrift)
                    | (3, M1StepWorkspaceSubleaseBindingError::SelectionBucketDrift)
            ));
        }
    }

    #[test]
    fn allocation_identity_length_and_alignment_drift_reject_before_reservation() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS1C8192,
        );
        let plan = exact_plan(selection);
        let validate_allocation = |identity, byte_len, alignment, declaration| {
            validate::<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                selection,
                identity,
                byte_len,
                alignment,
                plan.selection(),
                declaration,
                plan.ranges(),
            )
            .unwrap_err()
        };
        assert!(matches!(
            validate_allocation(
                Identity::new([0; 32]),
                plan.allocation().byte_len(),
                plan.allocation().alignment(),
                plan.allocation()
            ),
            M1StepWorkspaceSubleaseBindingError::MissingSelectedAllocationIdentity
        ));
        assert!(matches!(
            validate_allocation(
                identity(8),
                plan.allocation().byte_len(),
                plan.allocation().alignment(),
                plan.allocation()
            ),
            M1StepWorkspaceSubleaseBindingError::AllocationIdentityDrift
        ));
        assert!(matches!(
            validate_allocation(
                plan.allocation().allocation_id(),
                plan.allocation().byte_len() - 64,
                plan.allocation().alignment(),
                plan.allocation()
            ),
            M1StepWorkspaceSubleaseBindingError::RuntimeAllocationLengthDrift { .. }
        ));
        assert!(matches!(
            validate_allocation(
                plan.allocation().allocation_id(),
                plan.allocation().byte_len(),
                32,
                plan.allocation()
            ),
            M1StepWorkspaceSubleaseBindingError::RuntimeAllocationAlignmentDrift { .. }
        ));
        let wrong_plan_length = DeclaredM1StepWorkspaceAllocation::new(
            plan.allocation().allocation_id(),
            plan.allocation().byte_len() - 64,
            plan.allocation().alignment(),
        );
        assert!(matches!(
            validate_allocation(
                plan.allocation().allocation_id(),
                wrong_plan_length.byte_len(),
                wrong_plan_length.alignment(),
                wrong_plan_length
            ),
            M1StepWorkspaceSubleaseBindingError::PlanAllocationLengthDrift { .. }
        ));
        let wrong_plan_alignment = DeclaredM1StepWorkspaceAllocation::new(
            plan.allocation().allocation_id(),
            plan.allocation().byte_len(),
            32,
        );
        assert!(matches!(
            validate_allocation(
                plan.allocation().allocation_id(),
                wrong_plan_alignment.byte_len(),
                wrong_plan_alignment.alignment(),
                wrong_plan_alignment
            ),
            M1StepWorkspaceSubleaseBindingError::PlanAllocationAlignmentDrift { .. }
        ));
    }

    #[test]
    fn hostile_roster_rejects_count_empty_alignment_alias_and_exact_drift() {
        let selection = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Speculative,
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        );
        let plan = exact_plan(selection);
        let reject = |ranges: &[M1StepWorkspaceRange]| {
            validate::<M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                selection,
                plan.allocation().allocation_id(),
                plan.allocation().byte_len(),
                plan.allocation().alignment(),
                plan.selection(),
                plan.allocation(),
                ranges,
            )
            .unwrap_err()
        };

        let mut short = plan.ranges().to_vec();
        short.pop();
        assert!(matches!(
            reject(&short),
            M1StepWorkspaceSubleaseBindingError::PlanRangeCountDrift { .. }
        ));
        let mut empty = plan.ranges().to_vec();
        empty[0] =
            M1StepWorkspaceRange::new(empty[0].role(), empty[0].offset(), 0, empty[0].alignment());
        assert!(matches!(
            reject(&empty),
            M1StepWorkspaceSubleaseBindingError::EmptyRange { index: 0, .. }
        ));
        let mut alignment = plan.ranges().to_vec();
        alignment[0] = M1StepWorkspaceRange::new(
            alignment[0].role(),
            alignment[0].offset() + 1,
            alignment[0].byte_len(),
            alignment[0].alignment(),
        );
        assert!(matches!(
            reject(&alignment),
            M1StepWorkspaceSubleaseBindingError::RangeAlignmentDrift { index: 0, .. }
        ));
        let mut overflow = plan.ranges().to_vec();
        overflow[0] =
            M1StepWorkspaceRange::new(overflow[0].role(), u64::MAX - 3, 8, overflow[0].alignment());
        assert!(matches!(
            reject(&overflow),
            M1StepWorkspaceSubleaseBindingError::RangeOverflow { index: 0, .. }
        ));
        let mut out_of_bounds = plan.ranges().to_vec();
        let last = out_of_bounds.len() - 1;
        out_of_bounds[last] = M1StepWorkspaceRange::new(
            out_of_bounds[last].role(),
            plan.allocation().byte_len(),
            out_of_bounds[last].byte_len(),
            out_of_bounds[last].alignment(),
        );
        assert!(matches!(
            reject(&out_of_bounds),
            M1StepWorkspaceSubleaseBindingError::RangeOutOfBounds { index, .. }
                if index == last
        ));
        let mut alias = plan.ranges().to_vec();
        alias[1] = M1StepWorkspaceRange::new(
            alias[1].role(),
            alias[0].offset(),
            alias[1].byte_len(),
            alias[1].alignment(),
        );
        assert!(matches!(
            reject(&alias),
            M1StepWorkspaceSubleaseBindingError::RangeAlias { left: 0, right: 1 }
        ));
        let mut role = plan.ranges().to_vec();
        role.swap(0, 1);
        assert!(matches!(
            reject(&role),
            M1StepWorkspaceSubleaseBindingError::RangeRoleDrift { index: 0, .. }
        ));
        let mut offset = plan.ranges().to_vec();
        offset[0] = M1StepWorkspaceRange::new(
            offset[0].role(),
            offset[0].offset() + offset[0].alignment(),
            offset[0].byte_len(),
            offset[0].alignment(),
        );
        assert!(matches!(
            reject(&offset),
            M1StepWorkspaceSubleaseBindingError::RangeOffsetDrift { index: 0, .. }
        ));
        let mut length = plan.ranges().to_vec();
        length[0] = M1StepWorkspaceRange::new(
            length[0].role(),
            length[0].offset(),
            length[0].byte_len() - 4,
            length[0].alignment(),
        );
        assert!(matches!(
            reject(&length),
            M1StepWorkspaceSubleaseBindingError::RangeLengthDrift { index: 0, .. }
        ));
    }

    #[test]
    fn exact_role_subranges_are_absolute_contained_and_aligned() {
        let parent = M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 128, 64, 4);
        let exact = M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 144, 16, 4);
        assert_eq!(
            validate_role_contained_subrange(M1StepWorkspaceRangeRole::DraftChoices, parent, exact)
                .unwrap(),
            16
        );

        for (requested, expected) in [
            (
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::Choices, 144, 16, 4),
                1,
            ),
            (
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 144, 0, 4),
                2,
            ),
            (
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 146, 16, 4),
                3,
            ),
            (
                M1StepWorkspaceRange::new(
                    M1StepWorkspaceRangeRole::DraftChoices,
                    u64::MAX - 3,
                    8,
                    4,
                ),
                4,
            ),
            (
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 124, 16, 4),
                5,
            ),
            (
                M1StepWorkspaceRange::new(M1StepWorkspaceRangeRole::DraftChoices, 184, 16, 4),
                5,
            ),
        ] {
            let error = validate_role_contained_subrange(
                M1StepWorkspaceRangeRole::DraftChoices,
                parent,
                requested,
            )
            .unwrap_err();
            assert!(matches!(
                (expected, error),
                (1, M1StepWorkspaceDispatchRangeError::RangeRoleDrift { .. })
                    | (2, M1StepWorkspaceDispatchRangeError::EmptyRange { .. })
                    | (3, M1StepWorkspaceDispatchRangeError::RangeAlignment { .. })
                    | (4, M1StepWorkspaceDispatchRangeError::RangeOverflow { .. })
                    | (
                        5,
                        M1StepWorkspaceDispatchRangeError::RangeOutOfBounds { .. }
                    )
            ));
        }
    }
}
