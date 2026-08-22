//! Initialized allocation and exact sublease binding for full-step workspaces.
//!
//! Ferric owns the closed workspace-image shapes, their content-role namespace,
//! and the join from each complete image to its exact addressless workspace
//! plan. Generic fe2o3 owns device allocation, initialized-content validation,
//! mapping, and logical sublease custody.
//!
//! All plan, image, host-representability, and descriptor checks finish before
//! the first service allocation. Pure preflight rejection therefore returns the
//! exact plans and images. Once a service call begins, consumed images are not
//! recoverable through this API. A later allocation or binding failure can
//! leave earlier allocations and sublease partitions retained by the supplied
//! [`ServiceAllocationSessionV1`]; callers must release or quarantine that
//! session through its generic lifecycle.

use core::fmt;

use fe2o3_kfd::{
    Gfx942DeviceContentDescriptorErrorV1, Gfx942DeviceContentDescriptorV1,
    Gfx942DeviceContentRoleV1,
};
use fe2o3_service_host::{
    DeviceLocalAllocationV1, DeviceWorkspaceRoleV1, ServiceAllocationErrorV1,
    ServiceAllocationKeyV1, ServiceAllocationSessionV1,
};
use ferric_build::AddresslessM1StepWorkspacePlan;
use ferric_spec::{
    Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection,
};

use crate::step_workspace_subleases::validate_addressless_m1_step_workspace_subleases;
use crate::{
    bind_addressless_m1_step_workspace_subleases, M1FullStepWorkspaceInputKind,
    M1FullStepWorkspacePlans, M1FullStepWorkspaceSubleaseOwners,
    M1StepWorkspaceSubleaseBindingError, M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
    M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
    M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
};

/// SHA-256 of `ferric-m1-initialized-step-workspace-content-role-v1\0`.
///
/// This Ferric-owned namespace is independent of caller-selected workspace
/// identities and generic fe2o3 allocation generations.
pub const M1_INITIALIZED_STEP_WORKSPACE_CONTENT_ROLE_IDENTITY_V1: [u8; 32] = [
    215, 69, 23, 218, 99, 54, 86, 111, 203, 179, 209, 124, 123, 83, 82, 116, 213, 154, 159, 137,
    105, 90, 137, 151, 80, 235, 60, 49, 159, 219, 82, 49,
];

/// One exact slot in the closed full-step workspace-image shapes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1InitializedWorkspaceSlotV1 {
    /// Target image for a target-only prefill or decode step.
    TargetOnlyTarget,
    /// Draft image for paired prefill.
    PairedPrefillDraft,
    /// Target image for paired prefill.
    PairedPrefillTarget,
    /// Reusable draft-decode image for a speculative round.
    SpeculativeDraftDecode,
    /// Target verification image for a speculative round.
    SpeculativeTarget,
}

/// Complete initialized byte images for one exact full-step workspace shape.
///
/// This enum intentionally does not implement `Clone`: each complete image is
/// consumed exactly once by the initialized device-allocation path.
///
/// ```compile_fail
/// use ferric_engine::M1FullStepWorkspaceImagesV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1FullStepWorkspaceImagesV1>();
/// ```
#[must_use = "complete workspace images must be allocated or explicitly retained"]
#[derive(Debug, Eq, PartialEq)]
pub enum M1FullStepWorkspaceImagesV1 {
    /// One target-only image.
    TargetOnly {
        /// Complete target workspace bytes.
        target: Box<[u8]>,
    },
    /// Draft and target prefill images.
    PairedPrefill {
        /// Complete draft prefill workspace bytes.
        draft: Box<[u8]>,
        /// Complete target prefill workspace bytes.
        target: Box<[u8]>,
    },
    /// Reusable draft-decode and target-verification images.
    SpeculativeRound {
        /// Complete reusable draft-decode workspace bytes.
        draft_decode: Box<[u8]>,
        /// Complete target speculative workspace bytes.
        target_speculative: Box<[u8]>,
    },
}

impl M1FullStepWorkspaceImagesV1 {
    /// Wraps one complete target-only image.
    #[must_use = "the complete target workspace image remains retained"]
    pub fn target_only(target: Box<[u8]>) -> Self {
        Self::TargetOnly { target }
    }

    /// Wraps complete draft and target prefill images.
    #[must_use = "the complete paired workspace images remain retained"]
    pub fn paired_prefill(draft: Box<[u8]>, target: Box<[u8]>) -> Self {
        Self::PairedPrefill { draft, target }
    }

    /// Wraps complete draft-decode and target-verification images.
    #[must_use = "the complete speculative workspace images remain retained"]
    pub fn speculative_round(draft_decode: Box<[u8]>, target_speculative: Box<[u8]>) -> Self {
        Self::SpeculativeRound {
            draft_decode,
            target_speculative,
        }
    }

    /// Returns the exact closed image shape.
    #[must_use]
    pub const fn kind(&self) -> M1FullStepWorkspaceInputKind {
        match self {
            Self::TargetOnly { .. } => M1FullStepWorkspaceInputKind::TargetOnly,
            Self::PairedPrefill { .. } => M1FullStepWorkspaceInputKind::PairedPrefill,
            Self::SpeculativeRound { .. } => M1FullStepWorkspaceInputKind::SpeculativeRound,
        }
    }
}

/// Host-only rejection before any service allocation is attempted.
#[derive(Debug)]
pub enum InitializedM1FullStepWorkspacePreflightErrorV1 {
    /// Workspace plans and complete images use different closed shapes.
    InputKind {
        /// Shape selected by the plans.
        expected: M1FullStepWorkspaceInputKind,
        /// Shape supplied by the images.
        actual: M1FullStepWorkspaceInputKind,
    },
    /// A workspace selection does not satisfy its closed shape position.
    Selection {
        /// Rejected workspace-image slot.
        slot: M1InitializedWorkspaceSlotV1,
        /// Rejected exact selection.
        actual: Qwen3PlanSelection,
    },
    /// Draft and target plans name the same future allocation identity.
    AllocationAlias {
        /// Duplicated inert allocation identity.
        allocation_id: Identity,
    },
    /// An exact plan roster, selection, identity, extent, or alignment failed.
    Plan {
        /// Rejected workspace-image slot.
        slot: M1InitializedWorkspaceSlotV1,
        /// Existing exact sublease preflight diagnostic.
        source: M1StepWorkspaceSubleaseBindingError,
    },
    /// The declared complete image extent cannot be indexed on this host.
    HostImageExtent {
        /// Rejected workspace-image slot.
        slot: M1InitializedWorkspaceSlotV1,
        /// Exact declared allocation extent.
        byte_len: u64,
    },
    /// A supplied byte image is not the plan's complete allocation image.
    ImageLength {
        /// Rejected workspace-image slot.
        slot: M1InitializedWorkspaceSlotV1,
        /// Exact declared image length.
        expected: u64,
        /// Supplied host slice length.
        actual: usize,
    },
    /// A deterministic Ferric content descriptor could not be constructed.
    Descriptor {
        /// Rejected workspace-image slot.
        slot: M1InitializedWorkspaceSlotV1,
        /// Generic descriptor diagnostic.
        source: Gfx942DeviceContentDescriptorErrorV1,
    },
}

impl fmt::Display for InitializedM1FullStepWorkspacePreflightErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "initialized M1 full-step workspace preflight rejected: {self:?}"
        )
    }
}

impl std::error::Error for InitializedM1FullStepWorkspacePreflightErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Plan { source, .. } => Some(source),
            Self::Descriptor { source, .. } => Some(source),
            Self::InputKind { .. }
            | Self::Selection { .. }
            | Self::AllocationAlias { .. }
            | Self::HostImageExtent { .. }
            | Self::ImageLength { .. } => None,
        }
    }
}

/// Failure after all pure host preflight has completed.
#[derive(Debug)]
pub enum InitializedM1FullStepWorkspaceRuntimeErrorV1 {
    /// Generic initialized device allocation failed.
    Allocation {
        /// Workspace-image slot being allocated.
        slot: M1InitializedWorkspaceSlotV1,
        /// Generic allocation or KFD diagnostic.
        source: ServiceAllocationErrorV1,
    },
    /// A freshly allocated workspace could not reserve its exact subleases.
    Binding {
        /// Workspace-image slot being bound.
        slot: M1InitializedWorkspaceSlotV1,
        /// Existing exact binding diagnostic.
        source: M1StepWorkspaceSubleaseBindingError,
    },
}

impl fmt::Display for InitializedM1FullStepWorkspaceRuntimeErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "initialized M1 full-step workspace runtime failed: {self:?}"
        )
    }
}

impl std::error::Error for InitializedM1FullStepWorkspaceRuntimeErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Allocation { source, .. } => Some(source),
            Self::Binding { source, .. } => Some(source),
        }
    }
}

/// Fail-closed initialized full-step workspace allocation error.
#[derive(Debug)]
pub enum InitializedM1FullStepWorkspaceAllocationErrorV1 {
    /// No service allocation was attempted; exact inputs remain recoverable.
    Preflight(InitializedM1FullStepWorkspacePreflightErrorV1),
    /// Service allocation or exact sublease reservation failed.
    Runtime(InitializedM1FullStepWorkspaceRuntimeErrorV1),
}

impl fmt::Display for InitializedM1FullStepWorkspaceAllocationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "initialized M1 full-step workspace allocation failed: {self:?}"
        )
    }
}

impl std::error::Error for InitializedM1FullStepWorkspaceAllocationErrorV1 {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Preflight(source) => Some(source),
            Self::Runtime(source) => Some(source),
        }
    }
}

/// Rejected allocation attempt.
///
/// Pure preflight rejection retains the exact plans and images. Runtime
/// rejection cannot recover images already consumed by the generic initialized
/// allocation path; the supplied service session retains any partial native
/// allocation or sublease custody.
#[must_use = "preflight inputs or partial service-session custody require explicit handling"]
#[derive(Debug)]
pub struct InitializedM1FullStepWorkspaceAllocationFailureV1 {
    error: InitializedM1FullStepWorkspaceAllocationErrorV1,
    preflight_inputs: Option<Box<(M1FullStepWorkspacePlans, M1FullStepWorkspaceImagesV1)>>,
}

impl InitializedM1FullStepWorkspaceAllocationFailureV1 {
    /// Returns the fail-closed diagnostic.
    #[must_use]
    pub const fn error(&self) -> &InitializedM1FullStepWorkspaceAllocationErrorV1 {
        &self.error
    }

    /// Recovers exact plans and images when rejection occurred during preflight.
    ///
    /// Runtime failure returns the unchanged failure because one or more images
    /// may already have been consumed by the service allocation path.
    ///
    /// # Errors
    ///
    /// Returns the unchanged failure when service allocation or sublease
    /// binding had already begun.
    pub fn into_preflight_parts(
        self,
    ) -> Result<
        (
            InitializedM1FullStepWorkspacePreflightErrorV1,
            M1FullStepWorkspacePlans,
            M1FullStepWorkspaceImagesV1,
        ),
        Self,
    > {
        match (self.error, self.preflight_inputs) {
            (InitializedM1FullStepWorkspaceAllocationErrorV1::Preflight(error), Some(inputs)) => {
                let (plans, images) = *inputs;
                Ok((error, plans, images))
            }
            (error, preflight_inputs) => Err(Self {
                error,
                preflight_inputs,
            }),
        }
    }
}

/// Returns the deterministic Ferric content role for one closed workspace slot.
///
/// The fixed ordinals are target-only target `0`, paired-prefill draft `1`,
/// paired-prefill target `2`, speculative draft-decode `3`, and speculative
/// target `4`. This helper allocates nothing and grants no content authority.
///
/// # Errors
///
/// Returns the generic descriptor error if the fixed Ferric namespace is
/// invalid.
pub fn m1_step_workspace_content_role_v1(
    slot: M1InitializedWorkspaceSlotV1,
) -> Result<Gfx942DeviceContentRoleV1, Gfx942DeviceContentDescriptorErrorV1> {
    Gfx942DeviceContentRoleV1::new(
        M1_INITIALIZED_STEP_WORKSPACE_CONTENT_ROLE_IDENTITY_V1,
        workspace_content_ordinal(slot),
    )
}

/// Describes one complete workspace image under its deterministic Ferric role.
///
/// This helper is pure descriptor construction. It neither allocates nor maps
/// memory and cannot mint initialized-device authority.
///
/// # Errors
///
/// Returns the generic descriptor error for an invalid role or byte image.
pub fn m1_step_workspace_content_descriptor_v1(
    slot: M1InitializedWorkspaceSlotV1,
    bytes: &[u8],
) -> Result<Gfx942DeviceContentDescriptorV1, Gfx942DeviceContentDescriptorErrorV1> {
    Gfx942DeviceContentDescriptorV1::from_bytes(m1_step_workspace_content_role_v1(slot)?, bytes)
}

/// Allocates and binds complete initialized workspace images for one M1 step.
///
/// Shape and selection contracts, exact plan rosters and allocation geometry,
/// non-aliasing allocation identities, host representability, complete image
/// lengths, and all content descriptors are checked before the first service
/// call. Each image is then allocated with `DeviceWorkspaceRoleV1` using its
/// plan's exact base alignment, followed by reservation of the canonical exact
/// workspace subleases.
///
/// The returned owners carry no native allocation lease. The caller must keep
/// `allocations` alive for dispatch-range resolution and eventual generic
/// teardown or queue transfer.
///
/// # Errors
///
/// Returns [`InitializedM1FullStepWorkspaceAllocationFailureV1`] for pure
/// preflight, initialized service-allocation, or exact sublease-binding
/// rejection. Only pure preflight rejection guarantees exact recovery of both
/// input plans and complete images.
pub(crate) fn allocate_initialized_m1_full_step_workspaces_v1(
    allocations: &mut ServiceAllocationSessionV1,
    plans: M1FullStepWorkspacePlans,
    images: M1FullStepWorkspaceImagesV1,
) -> Result<M1FullStepWorkspaceSubleaseOwners, InitializedM1FullStepWorkspaceAllocationFailureV1> {
    let descriptors = match preflight_full_step_workspaces(&plans, &images) {
        Ok(descriptors) => descriptors,
        Err(error) => return Err(preflight_failure(error, plans, images)),
    };

    match (plans, images, descriptors) {
        (
            M1FullStepWorkspacePlans::TargetOnly { target },
            M1FullStepWorkspaceImagesV1::TargetOnly { target: image },
            WorkspaceDescriptors::TargetOnly { target: descriptor },
        ) => {
            let target = allocate_and_bind::<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                allocations,
                M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
                *target,
                image,
                descriptor,
            )?;
            Ok(M1FullStepWorkspaceSubleaseOwners::target_only(target))
        }
        (
            M1FullStepWorkspacePlans::PairedPrefill { draft, target },
            M1FullStepWorkspaceImagesV1::PairedPrefill {
                draft: draft_image,
                target: target_image,
            },
            WorkspaceDescriptors::PairedPrefill {
                draft: draft_descriptor,
                target: target_descriptor,
            },
        ) => {
            let draft = allocate_and_bind::<M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                allocations,
                M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
                *draft,
                draft_image,
                draft_descriptor,
            )?;
            let target = allocate_and_bind::<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                allocations,
                M1InitializedWorkspaceSlotV1::PairedPrefillTarget,
                *target,
                target_image,
                target_descriptor,
            )?;
            Ok(M1FullStepWorkspaceSubleaseOwners::paired_prefill(
                draft, target,
            ))
        }
        (
            M1FullStepWorkspacePlans::SpeculativeRound {
                draft_decode,
                target_speculative,
            },
            M1FullStepWorkspaceImagesV1::SpeculativeRound {
                draft_decode: draft_image,
                target_speculative: target_image,
            },
            WorkspaceDescriptors::SpeculativeRound {
                draft_decode: draft_descriptor,
                target_speculative: target_descriptor,
            },
        ) => {
            let draft = allocate_and_bind::<M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                allocations,
                M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
                *draft_decode,
                draft_image,
                draft_descriptor,
            )?;
            let target = allocate_and_bind::<M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                allocations,
                M1InitializedWorkspaceSlotV1::SpeculativeTarget,
                *target_speculative,
                target_image,
                target_descriptor,
            )?;
            Ok(M1FullStepWorkspaceSubleaseOwners::speculative_round(
                draft, target,
            ))
        }
        _ => unreachable!("preflight accepts only matching closed workspace shapes"),
    }
}

#[derive(Clone, Copy, Debug)]
enum WorkspaceDescriptors {
    TargetOnly {
        target: Gfx942DeviceContentDescriptorV1,
    },
    PairedPrefill {
        draft: Gfx942DeviceContentDescriptorV1,
        target: Gfx942DeviceContentDescriptorV1,
    },
    SpeculativeRound {
        draft_decode: Gfx942DeviceContentDescriptorV1,
        target_speculative: Gfx942DeviceContentDescriptorV1,
    },
}

fn preflight_full_step_workspaces(
    plans: &M1FullStepWorkspacePlans,
    images: &M1FullStepWorkspaceImagesV1,
) -> Result<WorkspaceDescriptors, InitializedM1FullStepWorkspacePreflightErrorV1> {
    if plans.kind() != images.kind() {
        return Err(InitializedM1FullStepWorkspacePreflightErrorV1::InputKind {
            expected: plans.kind(),
            actual: images.kind(),
        });
    }
    validate_selection_shape(plans)?;
    validate_distinct_allocations(plans)?;

    match (plans, images) {
        (
            M1FullStepWorkspacePlans::TargetOnly { target },
            M1FullStepWorkspaceImagesV1::TargetOnly { target: image },
        ) => Ok(WorkspaceDescriptors::TargetOnly {
            target: preflight_workspace::<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
                target,
                image,
            )?,
        }),
        (
            M1FullStepWorkspacePlans::PairedPrefill { draft, target },
            M1FullStepWorkspaceImagesV1::PairedPrefill {
                draft: draft_image,
                target: target_image,
            },
        ) => Ok(WorkspaceDescriptors::PairedPrefill {
            draft: preflight_workspace::<M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
                draft,
                draft_image,
            )?,
            target: preflight_workspace::<M1_TARGET_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                M1InitializedWorkspaceSlotV1::PairedPrefillTarget,
                target,
                target_image,
            )?,
        }),
        (
            M1FullStepWorkspacePlans::SpeculativeRound {
                draft_decode,
                target_speculative,
            },
            M1FullStepWorkspaceImagesV1::SpeculativeRound {
                draft_decode: draft_image,
                target_speculative: target_image,
            },
        ) => Ok(WorkspaceDescriptors::SpeculativeRound {
            draft_decode: preflight_workspace::<M1_DRAFT_STEP_WORKSPACE_SUBLEASE_COUNT_V1>(
                M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
                draft_decode,
                draft_image,
            )?,
            target_speculative: preflight_workspace::<
                M1_TARGET_SPECULATIVE_STEP_WORKSPACE_SUBLEASE_COUNT_V1,
            >(
                M1InitializedWorkspaceSlotV1::SpeculativeTarget,
                target_speculative,
                target_image,
            )?,
        }),
        _ => unreachable!("input-kind equality makes the closed shapes exhaustive"),
    }
}

fn validate_selection_shape(
    plans: &M1FullStepWorkspacePlans,
) -> Result<(), InitializedM1FullStepWorkspacePreflightErrorV1> {
    match plans {
        M1FullStepWorkspacePlans::TargetOnly { target } => {
            let selection = target.selection();
            if selection.role != Qwen3ModelRole::Target8B
                || !matches!(
                    selection.mode,
                    Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode
                )
                || selection
                    .bucket
                    .dimensions(selection.role, selection.mode)
                    .is_none()
            {
                return Err(selection_error(
                    M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
                    selection,
                ));
            }
        }
        M1FullStepWorkspacePlans::PairedPrefill { draft, target } => {
            let target_selection = target.selection();
            if target_selection.role != Qwen3ModelRole::Target8B
                || target_selection.mode != Qwen3ExecutionMode::Prefill
                || target_selection
                    .bucket
                    .dimensions(target_selection.role, target_selection.mode)
                    .is_none()
            {
                return Err(selection_error(
                    M1InitializedWorkspaceSlotV1::PairedPrefillTarget,
                    target_selection,
                ));
            }
            let expected_draft = Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: Qwen3ExecutionMode::Prefill,
                bucket: target_selection.bucket,
            };
            if draft.selection() != expected_draft {
                return Err(selection_error(
                    M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
                    draft.selection(),
                ));
            }
        }
        M1FullStepWorkspacePlans::SpeculativeRound {
            draft_decode,
            target_speculative,
        } => {
            let target_selection = target_speculative.selection();
            let draft_bucket = match target_selection {
                Qwen3PlanSelection {
                    role: Qwen3ModelRole::Target8B,
                    mode: Qwen3ExecutionMode::Speculative,
                    bucket:
                        Qwen3PlanBucket::SpeculativeS1K4C8192
                        | Qwen3PlanBucket::SpeculativeS1K8C8192
                        | Qwen3PlanBucket::SpeculativeS1K16C8192,
                } => Qwen3PlanBucket::DecodeS1C8192,
                Qwen3PlanSelection {
                    role: Qwen3ModelRole::Target8B,
                    mode: Qwen3ExecutionMode::Speculative,
                    bucket: Qwen3PlanBucket::SpeculativeS8K4C8192,
                } => Qwen3PlanBucket::DecodeS8C8192,
                _ => {
                    return Err(selection_error(
                        M1InitializedWorkspaceSlotV1::SpeculativeTarget,
                        target_selection,
                    ));
                }
            };
            let expected_draft = Qwen3PlanSelection {
                role: Qwen3ModelRole::Draft06B,
                mode: Qwen3ExecutionMode::Decode,
                bucket: draft_bucket,
            };
            if draft_decode.selection() != expected_draft {
                return Err(selection_error(
                    M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
                    draft_decode.selection(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_distinct_allocations(
    plans: &M1FullStepWorkspacePlans,
) -> Result<(), InitializedM1FullStepWorkspacePreflightErrorV1> {
    let Some(draft) = plans.draft() else {
        return Ok(());
    };
    let draft_id = draft.allocation().allocation_id();
    if draft_id == plans.target().allocation().allocation_id() {
        return Err(
            InitializedM1FullStepWorkspacePreflightErrorV1::AllocationAlias {
                allocation_id: draft_id,
            },
        );
    }
    Ok(())
}

fn preflight_workspace<const N: usize>(
    slot: M1InitializedWorkspaceSlotV1,
    plan: &AddresslessM1StepWorkspacePlan,
    image: &[u8],
) -> Result<Gfx942DeviceContentDescriptorV1, InitializedM1FullStepWorkspacePreflightErrorV1> {
    validate_addressless_m1_step_workspace_subleases::<N>(plan.selection(), plan)
        .map_err(|source| InitializedM1FullStepWorkspacePreflightErrorV1::Plan { slot, source })?;

    let byte_len = plan.allocation().byte_len();
    let expected = usize::try_from(byte_len).map_err(|_| {
        InitializedM1FullStepWorkspacePreflightErrorV1::HostImageExtent { slot, byte_len }
    })?;
    if image.len() != expected {
        return Err(
            InitializedM1FullStepWorkspacePreflightErrorV1::ImageLength {
                slot,
                expected: byte_len,
                actual: image.len(),
            },
        );
    }
    m1_step_workspace_content_descriptor_v1(slot, image).map_err(|source| {
        InitializedM1FullStepWorkspacePreflightErrorV1::Descriptor { slot, source }
    })
}

fn allocate_and_bind<const N: usize>(
    allocations: &mut ServiceAllocationSessionV1,
    slot: M1InitializedWorkspaceSlotV1,
    plan: AddresslessM1StepWorkspacePlan,
    image: Box<[u8]>,
    descriptor: Gfx942DeviceContentDescriptorV1,
) -> Result<
    crate::BoundM1StepWorkspaceSubleases<N>,
    InitializedM1FullStepWorkspaceAllocationFailureV1,
> {
    let selection = plan.selection();
    let allocation = plan.allocation();
    let key: ServiceAllocationKeyV1<DeviceWorkspaceRoleV1, DeviceLocalAllocationV1> = allocations
        .allocate_initialized_device_local::<DeviceWorkspaceRoleV1>(
            image,
            allocation.alignment(),
            descriptor,
        )
        .map_err(|source| {
            runtime_failure(InitializedM1FullStepWorkspaceRuntimeErrorV1::Allocation {
                slot,
                source,
            })
        })?;
    bind_addressless_m1_step_workspace_subleases::<N>(
        selection,
        allocation.allocation_id(),
        plan,
        allocations,
        key,
    )
    .map_err(|failure| {
        let (source, _) = failure.into_parts();
        runtime_failure(InitializedM1FullStepWorkspaceRuntimeErrorV1::Binding { slot, source })
    })
}

const fn workspace_content_ordinal(slot: M1InitializedWorkspaceSlotV1) -> u32 {
    match slot {
        M1InitializedWorkspaceSlotV1::TargetOnlyTarget => 0,
        M1InitializedWorkspaceSlotV1::PairedPrefillDraft => 1,
        M1InitializedWorkspaceSlotV1::PairedPrefillTarget => 2,
        M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode => 3,
        M1InitializedWorkspaceSlotV1::SpeculativeTarget => 4,
    }
}

fn selection_error(
    slot: M1InitializedWorkspaceSlotV1,
    actual: Qwen3PlanSelection,
) -> InitializedM1FullStepWorkspacePreflightErrorV1 {
    InitializedM1FullStepWorkspacePreflightErrorV1::Selection { slot, actual }
}

fn preflight_failure(
    error: InitializedM1FullStepWorkspacePreflightErrorV1,
    plans: M1FullStepWorkspacePlans,
    images: M1FullStepWorkspaceImagesV1,
) -> InitializedM1FullStepWorkspaceAllocationFailureV1 {
    InitializedM1FullStepWorkspaceAllocationFailureV1 {
        error: InitializedM1FullStepWorkspaceAllocationErrorV1::Preflight(error),
        preflight_inputs: Some(Box::new((plans, images))),
    }
}

fn runtime_failure(
    error: InitializedM1FullStepWorkspaceRuntimeErrorV1,
) -> InitializedM1FullStepWorkspaceAllocationFailureV1 {
    InitializedM1FullStepWorkspaceAllocationFailureV1 {
        error: InitializedM1FullStepWorkspaceAllocationErrorV1::Runtime(error),
        preflight_inputs: None,
    }
}

#[cfg(test)]
mod tests {
    use ferric_build::{
        m1_step_workspace_requirements, plan_addressless_m1_step_workspace,
        AddresslessM1StepWorkspacePlan, AvailableM1StepWorkspace,
        DeclaredM1StepWorkspaceAllocation, M1StepWorkspaceDeclaration, M1StepWorkspacePlanOutcome,
    };
    use sha2::{Digest, Sha256};

    use super::*;

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    fn exact_plan(
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
        let M1StepWorkspacePlanOutcome::Planned(plan) =
            plan_addressless_m1_step_workspace(selection, available)
        else {
            panic!("exact workspace fixture rejected")
        };
        plan
    }

    fn exact_image(plan: &AddresslessM1StepWorkspacePlan, fill: u8) -> Box<[u8]> {
        vec![fill; usize::try_from(plan.allocation().byte_len()).unwrap()].into_boxed_slice()
    }

    #[test]
    fn content_namespace_and_all_five_ordinals_are_frozen() {
        assert_eq!(
            M1_INITIALIZED_STEP_WORKSPACE_CONTENT_ROLE_IDENTITY_V1,
            Sha256::digest(b"ferric-m1-initialized-step-workspace-content-role-v1\0").as_slice()
        );
        for (ordinal, slot) in [
            M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
            M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
            M1InitializedWorkspaceSlotV1::PairedPrefillTarget,
            M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
            M1InitializedWorkspaceSlotV1::SpeculativeTarget,
        ]
        .into_iter()
        .enumerate()
        {
            let role = m1_step_workspace_content_role_v1(slot).unwrap();
            assert_eq!(
                role.identity(),
                M1_INITIALIZED_STEP_WORKSPACE_CONTENT_ROLE_IDENTITY_V1
            );
            assert_eq!(role.ordinal(), u32::try_from(ordinal).unwrap());
        }
    }

    #[test]
    fn descriptor_binds_slot_extent_and_complete_bytes_without_kfd() {
        let bytes = b"complete-workspace-image";
        let first = m1_step_workspace_content_descriptor_v1(
            M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
            bytes,
        )
        .unwrap();
        let second = m1_step_workspace_content_descriptor_v1(
            M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
            bytes,
        )
        .unwrap();
        let other_slot = m1_step_workspace_content_descriptor_v1(
            M1InitializedWorkspaceSlotV1::PairedPrefillTarget,
            bytes,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.byte_len(), u64::try_from(bytes.len()).unwrap());
        assert_eq!(first.sha256(), Sha256::digest(bytes).as_slice());
        assert_ne!(first.identity(), other_slot.identity());
    }

    #[test]
    fn matching_target_only_plan_and_image_preflight_without_kfd() {
        let plan = exact_plan(
            selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            1,
        );
        let image = exact_image(&plan, 0x5a);
        let descriptors = preflight_full_step_workspaces(
            &M1FullStepWorkspacePlans::target_only(plan),
            &M1FullStepWorkspaceImagesV1::target_only(image),
        )
        .unwrap();
        assert!(matches!(
            descriptors,
            WorkspaceDescriptors::TargetOnly { .. }
        ));
    }

    #[test]
    fn image_shape_drift_recovers_exact_plans_and_bytes_without_kfd() {
        let plan = exact_plan(
            selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            2,
        );
        let workspace_id = plan.workspace_id();
        let plans = M1FullStepWorkspacePlans::target_only(plan);
        let images = M1FullStepWorkspaceImagesV1::paired_prefill(
            vec![1, 2].into_boxed_slice(),
            vec![3, 4, 5].into_boxed_slice(),
        );
        let error = preflight_full_step_workspaces(&plans, &images).unwrap_err();
        let failure = preflight_failure(error, plans, images);
        let (error, plans, images) = failure.into_preflight_parts().unwrap();
        assert!(matches!(
            error,
            InitializedM1FullStepWorkspacePreflightErrorV1::InputKind {
                expected: M1FullStepWorkspaceInputKind::TargetOnly,
                actual: M1FullStepWorkspaceInputKind::PairedPrefill,
            }
        ));
        assert_eq!(plans.target().workspace_id(), workspace_id);
        let M1FullStepWorkspaceImagesV1::PairedPrefill { draft, target } = images else {
            panic!("exact rejected image shape changed")
        };
        assert_eq!(&*draft, &[1, 2]);
        assert_eq!(&*target, &[3, 4, 5]);
    }

    #[test]
    fn wrong_complete_image_length_fails_before_kfd() {
        let plan = exact_plan(
            selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
            ),
            3,
        );
        let expected = plan.allocation().byte_len();
        let error = preflight_full_step_workspaces(
            &M1FullStepWorkspacePlans::target_only(plan),
            &M1FullStepWorkspaceImagesV1::target_only(vec![0; 7].into_boxed_slice()),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            InitializedM1FullStepWorkspacePreflightErrorV1::ImageLength {
                slot: M1InitializedWorkspaceSlotV1::TargetOnlyTarget,
                expected: actual_expected,
                actual: 7,
            } if actual_expected == expected
        ));
    }

    #[test]
    fn paired_selection_drift_fails_before_large_images_or_kfd() {
        let draft = exact_plan(
            selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
            ),
            4,
        );
        let target = exact_plan(
            selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            ),
            5,
        );
        let error = preflight_full_step_workspaces(
            &M1FullStepWorkspacePlans::paired_prefill(draft, target),
            &M1FullStepWorkspaceImagesV1::paired_prefill(Box::new([]), Box::new([])),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            InitializedM1FullStepWorkspacePreflightErrorV1::Selection {
                slot: M1InitializedWorkspaceSlotV1::PairedPrefillDraft,
                ..
            }
        ));
    }

    #[test]
    fn paired_allocation_alias_fails_before_large_images_or_kfd() {
        let draft = exact_plan(
            selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            ),
            6,
        );
        let target = exact_plan(
            selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
            ),
            6,
        );
        let error = preflight_full_step_workspaces(
            &M1FullStepWorkspacePlans::paired_prefill(draft, target),
            &M1FullStepWorkspaceImagesV1::paired_prefill(Box::new([]), Box::new([])),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            InitializedM1FullStepWorkspacePreflightErrorV1::AllocationAlias {
                allocation_id,
            } if allocation_id == Identity::new([6; 32])
        ));
    }

    #[test]
    fn speculative_draft_bucket_must_match_target_sequence_shape_without_kfd() {
        let draft = exact_plan(
            selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
            ),
            7,
        );
        let target = exact_plan(
            selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
            ),
            8,
        );
        let error = preflight_full_step_workspaces(
            &M1FullStepWorkspacePlans::speculative_round(draft, target),
            &M1FullStepWorkspaceImagesV1::speculative_round(Box::new([]), Box::new([])),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            InitializedM1FullStepWorkspacePreflightErrorV1::Selection {
                slot: M1InitializedWorkspaceSlotV1::SpeculativeDraftDecode,
                ..
            }
        ));
    }
}
