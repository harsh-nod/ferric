//! Qualification-only host-visible capture of final target logits rows.
//!
//! Production workspaces remain device-local. Qualification explicitly opts in
//! to a second coherent output whose complete range substitutes only the target
//! `Logits` workspace binding. The substituted range is sealed with a BF16
//! quiet-NaN sentinel before dispatch so its inspected producer/consumer access
//! remains initialized and any missing final write fails closed. Completed
//! observation narrows that allocation to one final live BF16 row per scheduled
//! lane.
//!
//! The separate engineering S1/K4 diagnostic can additionally opt into two
//! appended final-RMS workspaces. That mode retains one complete owned readback
//! and exposes only active row 3; it grants no qualification authority.

use core::fmt;

use fe2o3_service_host::{
    HostDownloadRoleV1, HostVisibleAllocationV1, ServiceAllocationErrorV1, ServiceAllocationKeyV1,
    ServiceAllocationSessionV1, ServiceCompletedReadbackV1, ServiceHostDispatchRangeV1,
};
use ferric_build::{m1_step_workspace_requirements, M1StepWorkspaceRangeRole};
use ferric_spec::{
    finite_bf16_order_key, Qwen3ModelRole, Qwen3PlanSelection, TokenId, QWEN3_VOCABULARY_SIZE,
};
use sha2::{Digest, Sha256};

use crate::BoundM1CompletionOutputV1;

type QualificationLogitsAllocationKeyV1 =
    ServiceAllocationKeyV1<HostDownloadRoleV1, HostVisibleAllocationV1>;

/// BF16 byte width used by the admitted target logits workspace.
pub const M1_QUALIFICATION_LOGITS_ELEMENT_BYTES_V1: u64 = 2;
/// Exact alignment required by the existing BF16 logits kernel arguments.
pub const M1_QUALIFICATION_LOGITS_ALIGNMENT_V1: u64 = 2;

const M1_QUALIFICATION_UNWRITTEN_BF16_V1: u16 = 0x7fc0;
const ENGINEERING_S1_K4_LOGITS_BYTES: usize = 5 * QWEN3_VOCABULARY_SIZE as usize * 2;
const ENGINEERING_FINAL_RMS_ROW_BYTES: u64 = 4096 * 2;
const ENGINEERING_FINAL_RMS_WORKSPACE_BYTES: u64 = 5 * ENGINEERING_FINAL_RMS_ROW_BYTES;
const ENGINEERING_FINAL_RMS_ACTIVE_INDEX: u64 = 3;

/// Exact target workspace shape retained by qualification logits capture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1QualificationLogitsShapeV1 {
    selection: Qwen3PlanSelection,
    sequences: u32,
    active_tokens: u32,
    vocabulary: u32,
    row_bytes: u64,
    extent_bytes: u64,
}

impl M1QualificationLogitsShapeV1 {
    /// Exact target selection whose existing logits workspace is captured.
    #[must_use]
    pub const fn selection(self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Fixed sequence capacity of the selected graph.
    #[must_use]
    pub const fn sequences(self) -> u32 {
        self.sequences
    }

    /// Fixed active-token width of the selected graph.
    #[must_use]
    pub const fn active_tokens(self) -> u32 {
        self.active_tokens
    }

    /// Exact Qwen vocabulary width of every logits row.
    #[must_use]
    pub const fn vocabulary(self) -> u32 {
        self.vocabulary
    }

    /// Exact byte extent of one `[151936]` BF16 row.
    #[must_use]
    pub const fn row_bytes(self) -> u64 {
        self.row_bytes
    }

    /// Exact byte extent of the existing `[S,A,151936]` logits workspace.
    #[must_use]
    pub const fn extent_bytes(self) -> u64 {
        self.extent_bytes
    }

    /// Returns the relative byte offset of one lane's final live row.
    ///
    /// # Errors
    ///
    /// Rejects an out-of-capacity lane or an active length outside `1..=A`.
    pub fn final_row_relative_offset(
        self,
        lane: usize,
        active_length: u32,
    ) -> Result<u64, M1QualificationLogitsErrorV1> {
        let lane = u64::try_from(lane).map_err(|_| M1QualificationLogitsErrorV1::Overflow)?;
        if lane >= u64::from(self.sequences) {
            return Err(M1QualificationLogitsErrorV1::LaneOutOfRange {
                sequences: self.sequences,
                lane: usize::try_from(lane).unwrap_or(usize::MAX),
            });
        }
        if active_length == 0 || active_length > self.active_tokens {
            return Err(M1QualificationLogitsErrorV1::ActiveLength {
                lane: usize::try_from(lane).unwrap_or(usize::MAX),
                capacity: self.active_tokens,
                actual: active_length,
            });
        }
        let row = lane
            .checked_mul(u64::from(self.active_tokens))
            .and_then(|base| base.checked_add(u64::from(active_length - 1)))
            .ok_or(M1QualificationLogitsErrorV1::Overflow)?;
        row.checked_mul(self.row_bytes)
            .ok_or(M1QualificationLogitsErrorV1::Overflow)
    }
}

/// Qualification logits allocation, shape, or observation diagnostic.
#[derive(Debug)]
pub enum M1QualificationLogitsErrorV1 {
    /// The selection is not an admitted target graph with a logits workspace.
    InvalidTargetSelection { selection: Qwen3PlanSelection },
    /// The compact output already owns a qualification logits allocation.
    AlreadyEnabled,
    /// Checked shape arithmetic overflowed.
    Overflow,
    /// The complete initialized sentinel image could not be reserved on the host.
    HostInitialization { requested_bytes: usize },
    /// The generated logits range differs from the canonical `[S,A,V]` extent.
    WorkspaceExtent { expected: u64, actual: u64 },
    /// A later target selection differs from the retained capture shape.
    SelectionDrift {
        expected: Qwen3PlanSelection,
        actual: Qwen3PlanSelection,
    },
    /// A retained allocation key differs from the exact capture extent.
    AllocationExtent { expected: u64, actual: u64 },
    /// A retained allocation key cannot satisfy BF16 alignment.
    AllocationAlignment { required: u64, actual: u64 },
    /// Owner revalidation derived another host-visible dispatch range.
    DispatchRangeDrift,
    /// The scheduler live prefix exceeds the selected sequence capacity.
    LiveLaneCount { capacity: usize, actual: usize },
    /// One requested lane lies outside fixed sequence capacity.
    LaneOutOfRange { sequences: u32, lane: usize },
    /// One live lane has no valid final active row.
    ActiveLength {
        lane: usize,
        capacity: u32,
        actual: u32,
    },
    /// The copied row count differs from the live scheduler prefix.
    RowCount { expected: usize, actual: usize },
    /// One row was copied under another dispatch generation.
    DispatchGeneration {
        lane: usize,
        expected: u64,
        actual: u64,
    },
    /// One completed row returned another allocation offset.
    RowOffset {
        lane: usize,
        expected: u64,
        actual: u64,
    },
    /// One completed row returned another byte extent.
    RowExtent {
        lane: usize,
        expected: u64,
        actual: u64,
    },
    /// Generic allocation, mapping, range, or completed-copy rejection.
    Allocation(ServiceAllocationErrorV1),
}

impl fmt::Display for M1QualificationLogitsErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 qualification logits rejected: {self:?}")
    }
}

impl std::error::Error for M1QualificationLogitsErrorV1 {}

impl From<ServiceAllocationErrorV1> for M1QualificationLogitsErrorV1 {
    fn from(error: ServiceAllocationErrorV1) -> Self {
        Self::Allocation(error)
    }
}

/// Numerical rejection while deriving terminal choices from copied BF16 rows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1QualificationFinalLogitsErrorV1 {
    /// A retained row appeared outside exact scheduler lane order.
    LaneOrder { expected: usize, actual: usize },
    /// A retained row did not cover exactly one full vocabulary.
    RowExtent {
        lane: usize,
        expected: usize,
        actual: usize,
    },
    /// Qualification requires every terminal logit to be finite.
    NonFinite { lane: usize, token: TokenId },
    /// Bounded host storage for the derived lane choices was unavailable.
    HostAllocation,
}

impl fmt::Display for M1QualificationFinalLogitsErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 qualification final logits rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1QualificationFinalLogitsErrorV1 {}

/// Inert terminal choices derived from exact copied BF16 rows.
///
/// This value carries no completion or inference authority. It is kept private
/// to the engine crate so only the qualification-observed queue transition can
/// use it while joining compact output to quiescence.
#[derive(Debug)]
pub(crate) struct M1QualificationFinalRowChoicesV1 {
    choices: Box<[M1QualificationFinalRowChoiceV1]>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct M1QualificationFinalRowChoiceV1 {
    token: TokenId,
}

impl M1QualificationFinalRowChoiceV1 {
    pub(crate) const fn token(self) -> TokenId {
        self.token
    }
}

impl M1QualificationFinalRowChoicesV1 {
    pub(crate) fn len(&self) -> usize {
        self.choices.len()
    }

    pub(crate) fn choice(&self, lane: usize) -> Option<M1QualificationFinalRowChoiceV1> {
        self.choices.get(lane).copied()
    }

    fn from_raw_rows<'a>(
        rows: impl ExactSizeIterator<Item = &'a [u8]>,
    ) -> Result<Self, M1QualificationFinalLogitsErrorV1> {
        let mut choices = Vec::new();
        choices
            .try_reserve_exact(rows.len())
            .map_err(|_| M1QualificationFinalLogitsErrorV1::HostAllocation)?;
        for (lane, row) in rows.enumerate() {
            choices.push(M1QualificationFinalRowChoiceV1 {
                token: lowest_id_finite_bf16_argmax(row, lane)?,
            });
        }
        Ok(Self {
            choices: choices.into_boxed_slice(),
        })
    }
}

/// Move-only binding for one qualification-only coherent logits allocation.
///
/// ```compile_fail
/// use ferric_engine::BoundM1QualificationLogitsV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<BoundM1QualificationLogitsV1>();
/// ```
#[must_use = "qualification logits allocation custody must remain retained"]
#[derive(Debug)]
pub struct BoundM1QualificationLogitsV1 {
    shape: M1QualificationLogitsShapeV1,
    key: QualificationLogitsAllocationKeyV1,
    dispatch_range: ServiceHostDispatchRangeV1,
    capture_range: ServiceHostDispatchRangeV1,
    final_rms: bool,
}

impl BoundM1QualificationLogitsV1 {
    /// Exact selected logits workspace geometry.
    #[must_use]
    pub const fn shape(&self) -> M1QualificationLogitsShapeV1 {
        self.shape
    }

    /// Initially owner-checked complete `[S,A,V]` host-visible range.
    #[must_use]
    pub const fn retained_host_dispatch_range(&self) -> ServiceHostDispatchRangeV1 {
        self.dispatch_range
    }

    pub(crate) const fn retained_capture_range(&self) -> ServiceHostDispatchRangeV1 {
        self.capture_range
    }

    pub(crate) const fn captures_final_rms(&self) -> bool {
        self.final_rms
    }

    pub(crate) fn final_rms_workspace_ranges(
        &self,
    ) -> Result<Option<[ServiceHostDispatchRangeV1; 2]>, M1QualificationLogitsErrorV1> {
        if !self.final_rms {
            return Ok(None);
        }
        let offsets = engineering_final_rms_workspace_offsets(self.shape)?;
        Ok(Some([
            self.capture_range.checked_subrange(
                offsets[0],
                ENGINEERING_FINAL_RMS_WORKSPACE_BYTES,
                2,
            )?,
            self.capture_range.checked_subrange(
                offsets[1],
                ENGINEERING_FINAL_RMS_WORKSPACE_BYTES,
                2,
            )?,
        ]))
    }

    pub(crate) fn host_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        selection: Qwen3PlanSelection,
    ) -> Result<ServiceHostDispatchRangeV1, M1QualificationLogitsErrorV1> {
        let actual = m1_qualification_logits_shape_v1(selection)?;
        if actual != self.shape {
            return Err(M1QualificationLogitsErrorV1::SelectionDrift {
                expected: self.shape.selection,
                actual: selection,
            });
        }
        let extent = engineering_capture_extent(self.shape, self.final_rms)?;
        validate_key_geometry(self.key, extent)?;
        let typed = allocations.range(self.key, 0, extent, M1_QUALIFICATION_LOGITS_ALIGNMENT_V1)?;
        let capture = allocations.host_dispatch_range(typed)?;
        if capture != self.capture_range {
            return Err(M1QualificationLogitsErrorV1::DispatchRangeDrift);
        }
        let range = capture.checked_subrange(0, self.shape.extent_bytes, 2)?;
        if range != self.dispatch_range {
            return Err(M1QualificationLogitsErrorV1::DispatchRangeDrift);
        }
        Ok(range)
    }
}

/// Allocation failure retaining the already-bound compact K7 output.
///
/// ```compile_fail
/// use ferric_engine::M1QualificationLogitsAllocationFailureV1;
/// fn recover_twice(failure: M1QualificationLogitsAllocationFailureV1) {
///     let _first = failure.into_parts();
///     let _second = failure.into_parts();
/// }
/// ```
#[must_use = "the compact output and qualification allocation failure remain retained"]
#[derive(Debug)]
pub struct M1QualificationLogitsAllocationFailureV1 {
    error: M1QualificationLogitsErrorV1,
    completion: BoundM1CompletionOutputV1,
}

impl M1QualificationLogitsAllocationFailureV1 {
    /// Exact allocation or shape rejection.
    #[must_use]
    pub const fn error(&self) -> &M1QualificationLogitsErrorV1 {
        &self.error
    }

    /// Recovers the diagnostic and unchanged compact-output binding once.
    #[must_use = "the compact output binding remains retained"]
    pub fn into_parts(self) -> (M1QualificationLogitsErrorV1, BoundM1CompletionOutputV1) {
        (self.error, self.completion)
    }
}

/// Derives the exact existing target logits workspace shape.
///
/// # Errors
///
/// Rejects a non-target selection, missing or mismatched logits workspace, or
/// shape arithmetic that cannot be represented by the host allocation API.
pub fn m1_qualification_logits_shape_v1(
    selection: Qwen3PlanSelection,
) -> Result<M1QualificationLogitsShapeV1, M1QualificationLogitsErrorV1> {
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .filter(|_| selection.role == Qwen3ModelRole::Target8B)
        .ok_or(M1QualificationLogitsErrorV1::InvalidTargetSelection { selection })?;
    let row_bytes = u64::from(QWEN3_VOCABULARY_SIZE)
        .checked_mul(M1_QUALIFICATION_LOGITS_ELEMENT_BYTES_V1)
        .ok_or(M1QualificationLogitsErrorV1::Overflow)?;
    let extent_bytes = u64::from(dimensions.sequences)
        .checked_mul(u64::from(dimensions.active_tokens))
        .and_then(|elements| elements.checked_mul(row_bytes))
        .ok_or(M1QualificationLogitsErrorV1::Overflow)?;
    usize::try_from(extent_bytes).map_err(|_| M1QualificationLogitsErrorV1::Overflow)?;
    let requirements = m1_step_workspace_requirements(selection)
        .map_err(|_| M1QualificationLogitsErrorV1::InvalidTargetSelection { selection })?;
    let actual = requirements
        .range(M1StepWorkspaceRangeRole::Logits)
        .ok_or(M1QualificationLogitsErrorV1::InvalidTargetSelection { selection })?
        .byte_len();
    if actual != extent_bytes {
        return Err(M1QualificationLogitsErrorV1::WorkspaceExtent {
            expected: extent_bytes,
            actual,
        });
    }
    Ok(M1QualificationLogitsShapeV1 {
        selection,
        sequences: dimensions.sequences,
        active_tokens: dimensions.active_tokens,
        vocabulary: QWEN3_VOCABULARY_SIZE,
        row_bytes,
        extent_bytes,
    })
}

pub(crate) fn allocate_m1_qualification_logits_v1(
    allocations: &mut ServiceAllocationSessionV1,
    selection: Qwen3PlanSelection,
) -> Result<BoundM1QualificationLogitsV1, M1QualificationLogitsErrorV1> {
    allocate_logits_capture(allocations, selection, false)
}

fn allocate_logits_capture(
    allocations: &mut ServiceAllocationSessionV1,
    selection: Qwen3PlanSelection,
    final_rms: bool,
) -> Result<BoundM1QualificationLogitsV1, M1QualificationLogitsErrorV1> {
    let shape = m1_qualification_logits_shape_v1(selection)?;
    let extent = engineering_capture_extent(shape, final_rms)?;
    let requested = usize::try_from(extent).map_err(|_| M1QualificationLogitsErrorV1::Overflow)?;
    let initialized = qualification_logits_initial_image(requested)?;
    let key = allocations.allocate_initialized_host_visible::<HostDownloadRoleV1>(initialized)?;
    validate_key_geometry(key, extent)?;
    let typed = allocations.range(key, 0, extent, M1_QUALIFICATION_LOGITS_ALIGNMENT_V1)?;
    let capture_range = allocations.host_dispatch_range(typed)?;
    let dispatch_range = capture_range.checked_subrange(0, shape.extent_bytes, 2)?;
    Ok(BoundM1QualificationLogitsV1 {
        shape,
        key,
        dispatch_range,
        capture_range,
        final_rms,
    })
}

fn qualification_logits_initial_image(
    requested_bytes: usize,
) -> Result<Box<[u8]>, M1QualificationLogitsErrorV1> {
    if requested_bytes == 0 || !requested_bytes.is_multiple_of(2) {
        return Err(M1QualificationLogitsErrorV1::Overflow);
    }
    let sentinel = M1_QUALIFICATION_UNWRITTEN_BF16_V1.to_le_bytes();
    let mut image = Vec::new();
    image
        .try_reserve_exact(requested_bytes)
        .map_err(|_| M1QualificationLogitsErrorV1::HostInitialization { requested_bytes })?;
    image.extend_from_slice(&sentinel);
    while image.len() < requested_bytes {
        let copy_len = image.len().min(requested_bytes - image.len());
        image.extend_from_within(..copy_len);
    }
    Ok(image.into_boxed_slice())
}

pub(crate) fn attach_m1_qualification_logits_v1(
    allocations: &mut ServiceAllocationSessionV1,
    completion: BoundM1CompletionOutputV1,
) -> Result<BoundM1CompletionOutputV1, Box<M1QualificationLogitsAllocationFailureV1>> {
    if completion.qualification_logits().is_some()
        || completion.engineering_s1_k4_logits().is_some()
    {
        return Err(Box::new(M1QualificationLogitsAllocationFailureV1 {
            error: M1QualificationLogitsErrorV1::AlreadyEnabled,
            completion,
        }));
    }
    match allocate_m1_qualification_logits_v1(allocations, completion.shape().selection()) {
        Ok(logits) => Ok(completion.attach_qualification_logits(logits)),
        Err(error) => Err(Box::new(M1QualificationLogitsAllocationFailureV1 {
            error,
            completion,
        })),
    }
}

/// Exact engineering-only S1/K4 target geometry. This does not admit a
/// speculative graph to the protected target-only qualification path.
///
/// # Errors
/// Rejects any role, mode, or bucket other than Target8B/Speculative/S1K4.
pub fn m1_engineering_s1_k4_logits_shape_v1(
    selection: Qwen3PlanSelection,
) -> Result<M1QualificationLogitsShapeV1, M1QualificationLogitsErrorV1> {
    if !crate::speculative_diagnostic_choices::m1_speculative_diagnostic_is_s1_k4_selection_v1(
        selection,
    ) {
        return Err(M1QualificationLogitsErrorV1::InvalidTargetSelection { selection });
    }
    m1_qualification_logits_shape_v1(selection)
}

pub(crate) fn attach_m1_engineering_s1_k4_logits_v1(
    allocations: &mut ServiceAllocationSessionV1,
    completion: BoundM1CompletionOutputV1,
    final_rms: bool,
) -> Result<BoundM1CompletionOutputV1, Box<M1QualificationLogitsAllocationFailureV1>> {
    let result =
        m1_engineering_s1_k4_logits_shape_v1(completion.shape().selection()).and_then(|_| {
            if completion.qualification_logits().is_some()
                || completion.engineering_s1_k4_logits().is_some()
                || completion.completion_canary().is_some()
                || completion.direct_diagnostic_choices().is_some()
                || completion.speculative_diagnostic_choices().is_none()
                || completion.is_draft_catchup()
            {
                return Err(M1QualificationLogitsErrorV1::AlreadyEnabled);
            }
            allocate_logits_capture(allocations, completion.shape().selection(), final_rms)
        });
    match result {
        Ok(logits) => Ok(completion.attach_engineering_s1_k4_logits(logits)),
        Err(error) => Err(Box::new(M1QualificationLogitsAllocationFailureV1 {
            error,
            completion,
        })),
    }
}

/// Inert, move-only engineering evidence for all five target positions in one
/// actual S1/K4 generation. It grants no qualification or completion authority.
///
/// ```compile_fail
/// use ferric_engine::M1ObservedEngineeringS1K4LogitsV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1ObservedEngineeringS1K4LogitsV1>();
/// ```
#[must_use = "all five copied target rows must be reported or retained"]
#[derive(Debug)]
pub struct M1ObservedEngineeringS1K4LogitsV1 {
    shape: M1QualificationLogitsShapeV1,
    readback: ServiceCompletedReadbackV1,
    raw_sha256: [u8; 32],
    choices: [TokenId; 5],
    final_rms: bool,
    capture_sha256: [u8; 32],
}

impl M1ObservedEngineeringS1K4LogitsV1 {
    pub(crate) fn into_readback(self) -> ServiceCompletedReadbackV1 {
        self.readback
    }

    #[must_use]
    pub const fn shape(&self) -> M1QualificationLogitsShapeV1 {
        self.shape
    }

    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.readback.dispatch_generation()
    }

    #[must_use]
    pub const fn offset_bytes(&self) -> u64 {
        self.readback.offset_bytes()
    }

    #[must_use]
    pub fn raw_bytes(&self) -> &[u8] {
        &self.readback.bytes()[..ENGINEERING_S1_K4_LOGITS_BYTES]
    }

    /// SHA-256 of the actual complete owned copy, including unpublished RMS rows.
    #[must_use]
    pub const fn completed_readback_sha256(&self) -> &[u8; 32] {
        &self.capture_sha256
    }

    /// Exact complete copied extent, not merely the published logits prefix.
    #[must_use]
    pub fn completed_readback_extent_bytes(&self) -> usize {
        self.readback.bytes().len()
    }

    /// Final RMS input and output for target active row 3 only, when opted in.
    #[must_use]
    pub fn final_rms_row_bytes(&self) -> Option<[&[u8]; 2]> {
        if !self.final_rms {
            return None;
        }
        let offsets = engineering_final_rms_row_offsets(self.shape).ok()?;
        let row = |offset: u64| {
            let start = usize::try_from(offset).ok()?;
            let bytes = usize::try_from(ENGINEERING_FINAL_RMS_ROW_BYTES).ok()?;
            self.readback.bytes().get(start..start.checked_add(bytes)?)
        };
        Some([row(offsets[0])?, row(offsets[1])?])
    }

    /// Allocation-relative completed-copy coordinates of the two published rows.
    #[must_use]
    pub fn final_rms_row_offsets(&self) -> Option<[u64; 2]> {
        if !self.final_rms {
            return None;
        }
        let offsets = engineering_final_rms_row_offsets(self.shape).ok()?;
        Some([
            self.offset_bytes().checked_add(offsets[0])?,
            self.offset_bytes().checked_add(offsets[1])?,
        ])
    }

    #[must_use]
    pub const fn raw_sha256(&self) -> &[u8; 32] {
        &self.raw_sha256
    }

    /// A bounded row in sequence-major `[1,5,151936]` order.
    #[must_use]
    pub fn row_bytes(&self, active_index: usize) -> Option<&[u8]> {
        engineering_s1_k4_row_bounds(self.shape, active_index)
            .and_then(|(start, end)| self.readback.bytes().get(start..end))
    }

    #[must_use]
    pub const fn choices(&self) -> &[TokenId; 5] {
        &self.choices
    }
}

#[derive(Debug)]
pub enum M1EngineeringS1K4LogitsErrorV1 {
    Geometry(M1QualificationLogitsErrorV1),
    Values(M1QualificationFinalLogitsErrorV1),
    /// One final-RMS workspace element is nonfinite or retains its unwritten sentinel.
    FinalRmsNonFinite {
        workspace: usize,
        element: usize,
    },
}

fn engineering_final_rms_workspace_offsets(
    shape: M1QualificationLogitsShapeV1,
) -> Result<[u64; 2], M1QualificationLogitsErrorV1> {
    if m1_engineering_s1_k4_logits_shape_v1(shape.selection)? != shape {
        return Err(M1QualificationLogitsErrorV1::InvalidTargetSelection {
            selection: shape.selection,
        });
    }
    let requirements = m1_step_workspace_requirements(shape.selection).map_err(|_| {
        M1QualificationLogitsErrorV1::InvalidTargetSelection {
            selection: shape.selection,
        }
    })?;
    for role in [
        M1StepWorkspaceRangeRole::ResidualHidden,
        M1StepWorkspaceRangeRole::FinalNormalized,
    ] {
        let actual = requirements
            .range(role)
            .ok_or(M1QualificationLogitsErrorV1::InvalidTargetSelection {
                selection: shape.selection,
            })?
            .byte_len();
        if actual != ENGINEERING_FINAL_RMS_WORKSPACE_BYTES {
            return Err(M1QualificationLogitsErrorV1::WorkspaceExtent {
                expected: ENGINEERING_FINAL_RMS_WORKSPACE_BYTES,
                actual,
            });
        }
    }
    let second = shape
        .extent_bytes
        .checked_add(ENGINEERING_FINAL_RMS_WORKSPACE_BYTES)
        .ok_or(M1QualificationLogitsErrorV1::Overflow)?;
    Ok([shape.extent_bytes, second])
}

fn engineering_final_rms_row_offsets(
    shape: M1QualificationLogitsShapeV1,
) -> Result<[u64; 2], M1QualificationLogitsErrorV1> {
    let [input, output] = engineering_final_rms_workspace_offsets(shape)?;
    let row = ENGINEERING_FINAL_RMS_ACTIVE_INDEX * ENGINEERING_FINAL_RMS_ROW_BYTES;
    Ok([
        input
            .checked_add(row)
            .ok_or(M1QualificationLogitsErrorV1::Overflow)?,
        output
            .checked_add(row)
            .ok_or(M1QualificationLogitsErrorV1::Overflow)?,
    ])
}

fn engineering_capture_extent(
    shape: M1QualificationLogitsShapeV1,
    final_rms: bool,
) -> Result<u64, M1QualificationLogitsErrorV1> {
    if !final_rms {
        return Ok(shape.extent_bytes);
    }
    let offsets = engineering_final_rms_workspace_offsets(shape)?;
    offsets[1]
        .checked_add(ENGINEERING_FINAL_RMS_WORKSPACE_BYTES)
        .ok_or(M1QualificationLogitsErrorV1::Overflow)
}

fn engineering_s1_k4_row_bounds(
    shape: M1QualificationLogitsShapeV1,
    active_index: usize,
) -> Option<(usize, usize)> {
    if m1_engineering_s1_k4_logits_shape_v1(shape.selection).ok() != Some(shape)
        || active_index >= 5
    {
        return None;
    }
    let width = usize::try_from(shape.row_bytes).ok()?;
    let start = active_index.checked_mul(width)?;
    let end = start.checked_add(width)?;
    (u64::try_from(end).ok()? <= shape.extent_bytes).then_some((start, end))
}

fn validate_engineering_s1_k4_logits(
    shape: M1QualificationLogitsShapeV1,
    offset: u64,
    generation: u64,
    coordinates: M1QualificationLogitsRowCoordinatesV1,
    bytes: &[u8],
) -> Result<[TokenId; 5], M1EngineeringS1K4LogitsErrorV1> {
    validate_engineering_s1_k4_capture(shape, offset, generation, coordinates, bytes, false)
}

fn validate_engineering_s1_k4_capture(
    shape: M1QualificationLogitsShapeV1,
    offset: u64,
    generation: u64,
    coordinates: M1QualificationLogitsRowCoordinatesV1,
    bytes: &[u8],
    final_rms: bool,
) -> Result<[TokenId; 5], M1EngineeringS1K4LogitsErrorV1> {
    let geometry = || -> Result<(), M1QualificationLogitsErrorV1> {
        if m1_engineering_s1_k4_logits_shape_v1(shape.selection)? != shape {
            return Err(M1QualificationLogitsErrorV1::InvalidTargetSelection {
                selection: shape.selection,
            });
        }
        if coordinates.dispatch_generation != generation {
            return Err(M1QualificationLogitsErrorV1::DispatchGeneration {
                lane: 0,
                expected: generation,
                actual: coordinates.dispatch_generation,
            });
        }
        if coordinates.offset_bytes != offset {
            return Err(M1QualificationLogitsErrorV1::RowOffset {
                lane: 0,
                expected: offset,
                actual: coordinates.offset_bytes,
            });
        }
        let extent = engineering_capture_extent(shape, final_rms)?;
        if coordinates.extent_bytes != extent || u64::try_from(bytes.len()).ok() != Some(extent) {
            return Err(M1QualificationLogitsErrorV1::RowExtent {
                lane: 0,
                expected: extent,
                actual: coordinates.extent_bytes,
            });
        }
        Ok(())
    };
    geometry().map_err(M1EngineeringS1K4LogitsErrorV1::Geometry)?;
    let mut choices = [0; 5];
    for (index, choice) in choices.iter_mut().enumerate() {
        let (start, end) =
            engineering_s1_k4_row_bounds(shape, index).expect("validated exact S1/K4 geometry");
        *choice = lowest_id_finite_bf16_argmax(&bytes[start..end], index)
            .map_err(M1EngineeringS1K4LogitsErrorV1::Values)?;
    }
    if final_rms {
        let offsets = engineering_final_rms_workspace_offsets(shape)
            .map_err(M1EngineeringS1K4LogitsErrorV1::Geometry)?;
        for (workspace, offset) in offsets.into_iter().enumerate() {
            let start = usize::try_from(offset).map_err(|_| {
                M1EngineeringS1K4LogitsErrorV1::Geometry(M1QualificationLogitsErrorV1::Overflow)
            })?;
            let end = usize::try_from(ENGINEERING_FINAL_RMS_WORKSPACE_BYTES)
                .ok()
                .and_then(|extent| start.checked_add(extent))
                .ok_or(M1EngineeringS1K4LogitsErrorV1::Geometry(
                    M1QualificationLogitsErrorV1::Overflow,
                ))?;
            for (element, value) in bytes[start..end].chunks_exact(2).enumerate() {
                if u16::from_le_bytes([value[0], value[1]]) & 0x7f80 == 0x7f80 {
                    return Err(M1EngineeringS1K4LogitsErrorV1::FinalRmsNonFinite {
                        workspace,
                        element,
                    });
                }
            }
        }
    }
    Ok(choices)
}

pub(crate) fn observe_m1_engineering_s1_k4_logits_v1(
    shape: M1QualificationLogitsShapeV1,
    range: ServiceHostDispatchRangeV1,
    generation: u64,
    readback: ServiceCompletedReadbackV1,
    final_rms: bool,
) -> Result<
    M1ObservedEngineeringS1K4LogitsV1,
    (M1EngineeringS1K4LogitsErrorV1, ServiceCompletedReadbackV1),
> {
    let coordinates = M1QualificationLogitsRowCoordinatesV1 {
        dispatch_generation: readback.dispatch_generation(),
        offset_bytes: readback.offset_bytes(),
        extent_bytes: u64::try_from(readback.bytes().len()).unwrap_or(u64::MAX),
    };
    let validation = if final_rms {
        validate_engineering_s1_k4_capture(
            shape,
            range.offset_bytes(),
            generation,
            coordinates,
            readback.bytes(),
            true,
        )
    } else {
        validate_engineering_s1_k4_logits(
            shape,
            range.offset_bytes(),
            generation,
            coordinates,
            readback.bytes(),
        )
    };
    match validation {
        Ok(choices) => Ok(M1ObservedEngineeringS1K4LogitsV1 {
            shape,
            raw_sha256: Sha256::digest(&readback.bytes()[..ENGINEERING_S1_K4_LOGITS_BYTES]).into(),
            capture_sha256: Sha256::digest(readback.bytes()).into(),
            readback,
            choices,
            final_rms,
        }),
        Err(error) => Err((error, readback)),
    }
}

fn validate_key_geometry(
    key: QualificationLogitsAllocationKeyV1,
    extent: u64,
) -> Result<(), M1QualificationLogitsErrorV1> {
    if key.extent_bytes() != extent {
        return Err(M1QualificationLogitsErrorV1::AllocationExtent {
            expected: extent,
            actual: key.extent_bytes(),
        });
    }
    if key.alignment() < M1_QUALIFICATION_LOGITS_ALIGNMENT_V1
        || !key
            .alignment()
            .is_multiple_of(M1_QUALIFICATION_LOGITS_ALIGNMENT_V1)
    {
        return Err(M1QualificationLogitsErrorV1::AllocationAlignment {
            required: M1_QUALIFICATION_LOGITS_ALIGNMENT_V1,
            actual: key.alignment(),
        });
    }
    Ok(())
}

/// One exact final live `[151936]` BF16 row copied after queue completion.
#[must_use = "qualification logits bytes must be reported or retained"]
#[derive(Debug)]
pub struct M1ObservedQualificationLogitsRowV1 {
    lane: usize,
    active_index: u32,
    raw_sha256: [u8; 32],
    readback: ServiceCompletedReadbackV1,
}

impl M1ObservedQualificationLogitsRowV1 {
    /// Scheduler lane owning this final row.
    #[must_use]
    pub const fn lane(&self) -> usize {
        self.lane
    }

    /// Zero-based active-token index of the copied final row.
    #[must_use]
    pub const fn active_index(&self) -> u32 {
        self.active_index
    }

    /// Generic dispatch generation that authorized this copy.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.readback.dispatch_generation()
    }

    /// Exact offset of this row inside the complete logits allocation.
    #[must_use]
    pub const fn offset_bytes(&self) -> u64 {
        self.readback.offset_bytes()
    }

    /// Exact raw little-endian BF16 row bytes.
    #[must_use]
    pub fn raw_bytes(&self) -> &[u8] {
        self.readback.bytes()
    }

    /// SHA-256 of the exact copied row bytes.
    #[must_use]
    pub const fn raw_sha256(&self) -> &[u8; 32] {
        &self.raw_sha256
    }
}

/// Move-only inert final-row observation for one target-only qualification step.
///
/// ```compile_fail
/// use ferric_engine::M1ObservedQualificationLogitsV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<M1ObservedQualificationLogitsV1>();
/// ```
#[must_use = "qualification logits observation must be reported or retained"]
#[derive(Debug)]
pub struct M1ObservedQualificationLogitsV1 {
    shape: M1QualificationLogitsShapeV1,
    rows: Box<[M1ObservedQualificationLogitsRowV1]>,
}

impl M1ObservedQualificationLogitsV1 {
    /// Exact target selection and complete backing geometry.
    #[must_use]
    pub const fn shape(&self) -> M1QualificationLogitsShapeV1 {
        self.shape
    }

    /// Final live rows in exact scheduler lane order.
    #[must_use = "the final live logits rows remain retained by this observation"]
    pub fn rows(&self) -> &[M1ObservedQualificationLogitsRowV1] {
        &self.rows
    }

    pub(crate) fn final_row_choices(
        &self,
    ) -> Result<M1QualificationFinalRowChoicesV1, M1QualificationFinalLogitsErrorV1> {
        for (lane, row) in self.rows.iter().enumerate() {
            if row.lane != lane {
                return Err(M1QualificationFinalLogitsErrorV1::LaneOrder {
                    expected: lane,
                    actual: row.lane,
                });
            }
        }
        M1QualificationFinalRowChoicesV1::from_raw_rows(
            self.rows
                .iter()
                .map(M1ObservedQualificationLogitsRowV1::raw_bytes),
        )
    }
}

fn lowest_id_finite_bf16_argmax(
    bytes: &[u8],
    lane: usize,
) -> Result<TokenId, M1QualificationFinalLogitsErrorV1> {
    let expected = usize::try_from(
        u64::from(QWEN3_VOCABULARY_SIZE) * M1_QUALIFICATION_LOGITS_ELEMENT_BYTES_V1,
    )
    .expect("the fixed M1 BF16 vocabulary row fits usize");
    if bytes.len() != expected {
        return Err(M1QualificationFinalLogitsErrorV1::RowExtent {
            lane,
            expected,
            actual: bytes.len(),
        });
    }
    let mut best_token = 0;
    let mut best_value = i64::MIN;
    for (token, encoded) in bytes.chunks_exact(2).enumerate() {
        let bits = u16::from_le_bytes([encoded[0], encoded[1]]);
        let token = TokenId::try_from(token).expect("the fixed M1 vocabulary fits TokenId");
        let Some(value) = finite_bf16_order_key(bits) else {
            return Err(M1QualificationFinalLogitsErrorV1::NonFinite { lane, token });
        };
        if value > best_value {
            best_value = value;
            best_token = token;
        }
    }
    Ok(best_token)
}

pub(crate) fn observe_m1_qualification_logits_v1(
    shape: M1QualificationLogitsShapeV1,
    full_range: ServiceHostDispatchRangeV1,
    dispatch_generation: u64,
    active_lengths: &[u32],
    readbacks: Vec<ServiceCompletedReadbackV1>,
) -> Result<
    M1ObservedQualificationLogitsV1,
    (
        M1QualificationLogitsErrorV1,
        Vec<ServiceCompletedReadbackV1>,
    ),
> {
    if let Err(error) = validate_rows(
        shape,
        full_range.offset_bytes(),
        dispatch_generation,
        active_lengths,
        &readbacks,
        |readback| M1QualificationLogitsRowCoordinatesV1 {
            dispatch_generation: readback.dispatch_generation(),
            offset_bytes: readback.offset_bytes(),
            extent_bytes: u64::try_from(readback.bytes().len()).unwrap_or(u64::MAX),
        },
    ) {
        return Err((error, readbacks));
    }
    let rows = readbacks
        .into_iter()
        .enumerate()
        .map(|(lane, readback)| M1ObservedQualificationLogitsRowV1 {
            lane,
            active_index: active_lengths[lane] - 1,
            raw_sha256: Sha256::digest(readback.bytes()).into(),
            readback,
        })
        .collect();
    Ok(M1ObservedQualificationLogitsV1 { shape, rows })
}

#[derive(Clone, Copy, Debug)]
struct M1QualificationLogitsRowCoordinatesV1 {
    dispatch_generation: u64,
    offset_bytes: u64,
    extent_bytes: u64,
}

fn validate_rows<T>(
    shape: M1QualificationLogitsShapeV1,
    allocation_offset: u64,
    dispatch_generation: u64,
    active_lengths: &[u32],
    backings: &[T],
    coordinates: impl Fn(&T) -> M1QualificationLogitsRowCoordinatesV1,
) -> Result<(), M1QualificationLogitsErrorV1> {
    if active_lengths.len() > shape.sequences as usize {
        return Err(M1QualificationLogitsErrorV1::LiveLaneCount {
            capacity: shape.sequences as usize,
            actual: active_lengths.len(),
        });
    }
    if backings.len() != active_lengths.len() {
        return Err(M1QualificationLogitsErrorV1::RowCount {
            expected: active_lengths.len(),
            actual: backings.len(),
        });
    }
    for (lane, backing) in backings.iter().enumerate() {
        let coordinates = coordinates(backing);
        let relative = shape.final_row_relative_offset(lane, active_lengths[lane])?;
        let expected_offset = allocation_offset
            .checked_add(relative)
            .ok_or(M1QualificationLogitsErrorV1::Overflow)?;
        if coordinates.dispatch_generation != dispatch_generation {
            return Err(M1QualificationLogitsErrorV1::DispatchGeneration {
                lane,
                expected: dispatch_generation,
                actual: coordinates.dispatch_generation,
            });
        }
        if coordinates.offset_bytes != expected_offset {
            return Err(M1QualificationLogitsErrorV1::RowOffset {
                lane,
                expected: expected_offset,
                actual: coordinates.offset_bytes,
            });
        }
        if coordinates.extent_bytes != shape.row_bytes {
            return Err(M1QualificationLogitsErrorV1::RowExtent {
                lane,
                expected: shape.row_bytes,
                actual: coordinates.extent_bytes,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use ferric_spec::{Qwen3ExecutionMode, Qwen3PlanBucket};

    use super::*;

    fn bf16_row(fill: u16) -> Vec<u8> {
        let mut row = vec![0; usize::try_from(u64::from(QWEN3_VOCABULARY_SIZE) * 2).unwrap()];
        for encoded in row.chunks_exact_mut(2) {
            encoded.copy_from_slice(&fill.to_le_bytes());
        }
        row
    }

    fn set_bf16(row: &mut [u8], token: usize, bits: u16) {
        let offset = token * 2;
        row[offset..offset + 2].copy_from_slice(&bits.to_le_bytes());
    }

    pub(crate) fn final_row_choices_for_join_test(row: &[u8]) -> M1QualificationFinalRowChoicesV1 {
        M1QualificationFinalRowChoicesV1::from_raw_rows([row].into_iter()).unwrap()
    }

    const TARGET_ONLY_BUCKETS: [Qwen3PlanBucket; 7] = [
        Qwen3PlanBucket::PrefillS1T128,
        Qwen3PlanBucket::PrefillS8T128,
        Qwen3PlanBucket::PrefillS1T512,
        Qwen3PlanBucket::PrefillS1T2048,
        Qwen3PlanBucket::DecodeS1C8192,
        Qwen3PlanBucket::DecodeS8C8192,
        Qwen3PlanBucket::DecodeS32C8192,
    ];

    fn selection(bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: if matches!(
                bucket,
                Qwen3PlanBucket::PrefillS1T128
                    | Qwen3PlanBucket::PrefillS8T128
                    | Qwen3PlanBucket::PrefillS1T512
                    | Qwen3PlanBucket::PrefillS1T2048
            ) {
                Qwen3ExecutionMode::Prefill
            } else {
                Qwen3ExecutionMode::Decode
            },
            bucket,
        }
    }

    fn test_rows(
        shape: M1QualificationLogitsShapeV1,
        base: u64,
        generation: u64,
        active: &[u32],
    ) -> Vec<M1QualificationLogitsRowCoordinatesV1> {
        active
            .iter()
            .enumerate()
            .map(|(lane, length)| M1QualificationLogitsRowCoordinatesV1 {
                dispatch_generation: generation,
                offset_bytes: base + shape.final_row_relative_offset(lane, *length).unwrap(),
                extent_bytes: shape.row_bytes,
            })
            .collect()
    }

    #[test]
    fn engineering_s1_k4_shape_rejects_target_only_and_other_speculative_buckets() {
        let exact = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        };
        let shape = m1_engineering_s1_k4_logits_shape_v1(exact).unwrap();
        assert_eq!((shape.sequences(), shape.active_tokens()), (1, 5));
        assert_eq!(shape.extent_bytes(), 1_519_360);
        for bucket in TARGET_ONLY_BUCKETS {
            assert!(m1_engineering_s1_k4_logits_shape_v1(selection(bucket)).is_err());
        }
        for bucket in [
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ] {
            assert!(
                m1_engineering_s1_k4_logits_shape_v1(Qwen3PlanSelection { bucket, ..exact })
                    .is_err()
            );
        }
        assert!(m1_engineering_s1_k4_logits_shape_v1(Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            ..exact
        })
        .is_err());
    }

    #[test]
    fn engineering_s1_k4_all_rows_bind_generation_extent_offsets_and_finite_values() {
        let shape = m1_engineering_s1_k4_logits_shape_v1(Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        })
        .unwrap();
        let width = usize::try_from(shape.row_bytes()).unwrap();
        let mut bytes = vec![0; usize::try_from(shape.extent_bytes()).unwrap()];
        for index in 0..5 {
            let (start, end) = engineering_s1_k4_row_bounds(shape, index).unwrap();
            assert_eq!((start, end), (index * width, (index + 1) * width));
            set_bf16(&mut bytes[start..end], index + 11, 0x3f80);
        }
        assert_eq!(engineering_s1_k4_row_bounds(shape, 5), None);
        assert_eq!(engineering_s1_k4_row_bounds(shape, usize::MAX), None);
        let coordinates = M1QualificationLogitsRowCoordinatesV1 {
            dispatch_generation: 9,
            offset_bytes: 64,
            extent_bytes: shape.extent_bytes(),
        };
        assert_eq!(
            validate_engineering_s1_k4_logits(shape, 64, 9, coordinates, &bytes).unwrap(),
            [11, 12, 13, 14, 15]
        );
        for wrong in [
            M1QualificationLogitsRowCoordinatesV1 {
                dispatch_generation: 8,
                ..coordinates
            },
            M1QualificationLogitsRowCoordinatesV1 {
                offset_bytes: 66,
                ..coordinates
            },
            M1QualificationLogitsRowCoordinatesV1 {
                extent_bytes: shape.extent_bytes() - 2,
                ..coordinates
            },
        ] {
            assert!(matches!(
                validate_engineering_s1_k4_logits(shape, 64, 9, wrong, &bytes),
                Err(M1EngineeringS1K4LogitsErrorV1::Geometry(_))
            ));
        }
        assert!(validate_engineering_s1_k4_logits(
            shape,
            64,
            9,
            coordinates,
            &bytes[..bytes.len() - 2]
        )
        .is_err());
        for index in 0..5 {
            let mut missing = bytes.clone();
            let (start, end) = engineering_s1_k4_row_bounds(shape, index).unwrap();
            set_bf16(
                &mut missing[start..end],
                0,
                M1_QUALIFICATION_UNWRITTEN_BF16_V1,
            );
            assert!(
                matches!(validate_engineering_s1_k4_logits(shape, 64, 9, coordinates, &missing), Err(M1EngineeringS1K4LogitsErrorV1::Values(M1QualificationFinalLogitsErrorV1::NonFinite { lane, token: 0 })) if lane == index)
            );
        }
    }

    #[test]
    fn engineering_final_rms_layout_is_disjoint_aligned_and_opt_in_only() {
        let shape = m1_engineering_s1_k4_logits_shape_v1(Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        })
        .unwrap();
        assert_eq!(engineering_capture_extent(shape, false).unwrap(), 1_519_360);
        assert_eq!(engineering_capture_extent(shape, true).unwrap(), 1_601_280);
        assert_eq!(
            engineering_final_rms_workspace_offsets(shape).unwrap(),
            [1_519_360, 1_560_320]
        );
        assert_eq!(
            engineering_final_rms_row_offsets(shape).unwrap(),
            [1_543_936, 1_584_896]
        );
        let offsets = engineering_final_rms_workspace_offsets(shape).unwrap();
        assert_eq!(offsets[0], shape.extent_bytes());
        assert_eq!(
            offsets[0] + ENGINEERING_FINAL_RMS_WORKSPACE_BYTES,
            offsets[1]
        );
        assert_eq!(
            offsets[1] + ENGINEERING_FINAL_RMS_WORKSPACE_BYTES,
            engineering_capture_extent(shape, true).unwrap()
        );
        assert!(offsets.iter().all(|offset| offset.is_multiple_of(2)));
        for bucket in TARGET_ONLY_BUCKETS {
            let ordinary = m1_qualification_logits_shape_v1(selection(bucket)).unwrap();
            assert_eq!(
                engineering_capture_extent(ordinary, false).unwrap(),
                ordinary.extent_bytes()
            );
            assert!(engineering_capture_extent(ordinary, true).is_err());
        }
    }

    #[test]
    fn engineering_final_rms_copy_rejects_extent_generation_and_missing_writes() {
        let shape = m1_engineering_s1_k4_logits_shape_v1(Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket: Qwen3PlanBucket::SpeculativeS1K4C8192,
        })
        .unwrap();
        let extent = engineering_capture_extent(shape, true).unwrap();
        let bytes = vec![0; usize::try_from(extent).unwrap()];
        let coordinates = M1QualificationLogitsRowCoordinatesV1 {
            dispatch_generation: 9,
            offset_bytes: 64,
            extent_bytes: extent,
        };
        assert_eq!(
            validate_engineering_s1_k4_capture(shape, 64, 9, coordinates, &bytes, true).unwrap(),
            [0; 5]
        );
        assert!(validate_engineering_s1_k4_logits(shape, 64, 9, coordinates, &bytes).is_err());
        for wrong in [
            M1QualificationLogitsRowCoordinatesV1 {
                dispatch_generation: 8,
                ..coordinates
            },
            M1QualificationLogitsRowCoordinatesV1 {
                offset_bytes: 66,
                ..coordinates
            },
            M1QualificationLogitsRowCoordinatesV1 {
                extent_bytes: shape.extent_bytes(),
                ..coordinates
            },
        ] {
            assert!(validate_engineering_s1_k4_capture(shape, 64, 9, wrong, &bytes, true).is_err());
        }
        assert!(validate_engineering_s1_k4_capture(
            shape,
            64,
            9,
            coordinates,
            &bytes[..bytes.len() - 2],
            true
        )
        .is_err());
        for (workspace, offset) in engineering_final_rms_workspace_offsets(shape)
            .unwrap()
            .into_iter()
            .enumerate()
        {
            for element in [
                0,
                usize::try_from(ENGINEERING_FINAL_RMS_WORKSPACE_BYTES / 2 - 1).unwrap(),
            ] {
                for sentinel in [0x7fc0_u16, 0x7f80, 0xff80] {
                    let mut missing = bytes.clone();
                    set_bf16(
                        &mut missing[usize::try_from(offset).unwrap()..],
                        element,
                        sentinel,
                    );
                    assert!(
                        matches!(validate_engineering_s1_k4_capture(shape, 64, 9, coordinates, &missing, true),
                        Err(M1EngineeringS1K4LogitsErrorV1::FinalRmsNonFinite { workspace: actual_workspace, element: actual_element })
                        if actual_workspace == workspace && actual_element == element)
                    );
                }
            }
        }
    }

    #[test]
    fn all_target_only_shapes_match_existing_logits_workspace() {
        for bucket in TARGET_ONLY_BUCKETS {
            let shape = m1_qualification_logits_shape_v1(selection(bucket)).unwrap();
            assert_eq!(shape.vocabulary(), QWEN3_VOCABULARY_SIZE);
            assert_eq!(shape.row_bytes(), u64::from(QWEN3_VOCABULARY_SIZE) * 2);
            assert_eq!(
                shape.extent_bytes(),
                u64::from(shape.sequences()) * u64::from(shape.active_tokens()) * shape.row_bytes()
            );
        }
    }

    #[test]
    fn initialized_capture_image_uses_fail_closed_bf16_nan_sentinels() {
        let s1_t128 =
            m1_qualification_logits_shape_v1(selection(Qwen3PlanBucket::PrefillS1T128)).unwrap();
        assert_eq!(s1_t128.extent_bytes(), 38_895_616);
        assert!(matches!(
            qualification_logits_initial_image(0),
            Err(M1QualificationLogitsErrorV1::Overflow)
        ));
        assert!(matches!(
            qualification_logits_initial_image(3),
            Err(M1QualificationLogitsErrorV1::Overflow)
        ));

        let image = qualification_logits_initial_image(8).unwrap();
        assert_eq!(
            image.as_ref(),
            [0xc0, 0x7f, 0xc0, 0x7f, 0xc0, 0x7f, 0xc0, 0x7f]
        );

        let full_row =
            qualification_logits_initial_image(usize::try_from(QWEN3_VOCABULARY_SIZE).unwrap() * 2)
                .unwrap();
        assert_eq!(
            lowest_id_finite_bf16_argmax(&full_row, 5),
            Err(M1QualificationFinalLogitsErrorV1::NonFinite {
                lane: 5,
                token: TokenId::try_from(0usize).unwrap(),
            })
        );
    }

    #[test]
    fn terminal_bf16_argmax_is_finite_and_uses_lowest_id_for_ties() {
        let mut tied = bf16_row(0);
        set_bf16(&mut tied, 4, 0x3f80);
        set_bf16(&mut tied, 7, 0x3f80);
        assert_eq!(lowest_id_finite_bf16_argmax(&tied, 3).unwrap(), 4);

        let mut all_negative = bf16_row(0xc000);
        let last = usize::try_from(QWEN3_VOCABULARY_SIZE - 1).unwrap();
        set_bf16(&mut all_negative, last, 0xbf80);
        assert_eq!(
            lowest_id_finite_bf16_argmax(&all_negative, 3).unwrap(),
            TokenId::try_from(last).unwrap()
        );

        let mut signed_zero = bf16_row(0xbf80);
        set_bf16(&mut signed_zero, 2, 0x8000);
        set_bf16(&mut signed_zero, 5, 0x0000);
        assert_eq!(lowest_id_finite_bf16_argmax(&signed_zero, 3).unwrap(), 2);
        let mut reversed_signed_zero = bf16_row(0xbf80);
        set_bf16(&mut reversed_signed_zero, 2, 0x0000);
        set_bf16(&mut reversed_signed_zero, 5, 0x8000);
        assert_eq!(
            lowest_id_finite_bf16_argmax(&reversed_signed_zero, 3).unwrap(),
            2
        );
    }

    #[test]
    fn terminal_bf16_argmax_rejects_nan_infinity_and_extent_drift() {
        for bits in [0x7f80, 0xff80, 0x7fc0, 0xffc0, 0x7f81, 0xff81] {
            let mut row = bf16_row(0);
            set_bf16(&mut row, 11, bits);
            assert_eq!(
                lowest_id_finite_bf16_argmax(&row, 7),
                Err(M1QualificationFinalLogitsErrorV1::NonFinite { lane: 7, token: 11 })
            );
        }
        let mut first_nonfinite = bf16_row(0);
        set_bf16(&mut first_nonfinite, 0, 0x7f80);
        assert_eq!(
            lowest_id_finite_bf16_argmax(&first_nonfinite, 9),
            Err(M1QualificationFinalLogitsErrorV1::NonFinite { lane: 9, token: 0 })
        );

        let exact = bf16_row(0);
        assert!(matches!(
            lowest_id_finite_bf16_argmax(&exact[..exact.len() - 2], 1),
            Err(M1QualificationFinalLogitsErrorV1::RowExtent { lane: 1, .. })
        ));
        let mut trailing = exact;
        trailing.extend_from_slice(&[0, 0]);
        assert!(matches!(
            lowest_id_finite_bf16_argmax(&trailing, 2),
            Err(M1QualificationFinalLogitsErrorV1::RowExtent { lane: 2, .. })
        ));
    }

    #[test]
    fn terminal_bf16_argmax_matches_integer_sampler_for_finite_edge_rows() {
        for (fill, entries, expected) in [
            (0xbf80, [(2, 0x8000), (5, 0x0000), (7, 0x8001)], 2),
            (0xbf80, [(2, 0x0000), (5, 0x8000), (7, 0x8001)], 2),
            (0x0000, [(2, 0x8001), (5, 0x0001), (7, 0x0001)], 5),
            (0xc000, [(2, 0xbf81), (5, 0xbf80), (7, 0xbf80)], 5),
            (0x0000, [(2, 0x7f7e), (5, 0x7f7f), (7, 0x7f7f)], 5),
            (0xbf80, [(16, 0x4196), (220, 0x4196), (576, 0x4196)], 16),
            (0xbf80, [(16, 0x4195), (220, 0x4195), (576, 0x4196)], 576),
        ] {
            let mut row = bf16_row(fill);
            for (token, bits) in entries {
                set_bf16(&mut row, token, bits);
            }
            let scores: Vec<_> = row
                .chunks_exact(2)
                .map(|encoded| {
                    finite_bf16_order_key(u16::from_le_bytes([encoded[0], encoded[1]])).unwrap()
                })
                .collect();
            assert_eq!(ferric_spec::select_lowest_argmax(&scores), Ok(expected));
            assert_eq!(lowest_id_finite_bf16_argmax(&row, 3), Ok(expected));
        }
    }

    #[test]
    fn terminal_bf16_argmax_rejects_late_nonfinite_after_finite_maximum() {
        let last = usize::try_from(QWEN3_VOCABULARY_SIZE - 1).unwrap();
        for bits in [0x7f80, 0xff80, 0x7fc0, 0xffc0, 0x7f81, 0xff81] {
            let mut row = bf16_row(0);
            set_bf16(&mut row, 0, 0x7f7f);
            set_bf16(&mut row, last, bits);
            assert_eq!(
                lowest_id_finite_bf16_argmax(&row, 9),
                Err(M1QualificationFinalLogitsErrorV1::NonFinite {
                    lane: 9,
                    token: TokenId::try_from(last).unwrap(),
                })
            );
        }
    }

    #[test]
    fn every_target_only_shape_has_exact_first_and_last_final_row_boundaries() {
        for bucket in TARGET_ONLY_BUCKETS {
            let shape = m1_qualification_logits_shape_v1(selection(bucket)).unwrap();
            let last_lane = usize::try_from(shape.sequences() - 1).unwrap();
            let full_active = shape.active_tokens();
            let first_full = u64::from(full_active - 1) * shape.row_bytes();
            let last_first =
                u64::from(shape.sequences() - 1) * u64::from(full_active) * shape.row_bytes();

            assert_eq!(shape.final_row_relative_offset(0, 1).unwrap(), 0);
            assert_eq!(
                shape.final_row_relative_offset(0, full_active).unwrap(),
                first_full
            );
            assert_eq!(
                shape.final_row_relative_offset(last_lane, 1).unwrap(),
                last_first
            );
            assert_eq!(
                shape
                    .final_row_relative_offset(last_lane, full_active)
                    .unwrap(),
                shape.extent_bytes() - shape.row_bytes()
            );
            assert!(last_first
                .checked_add(shape.row_bytes())
                .is_some_and(|end| end <= shape.extent_bytes()));
        }
    }

    #[test]
    fn final_live_rows_preserve_lane_index_coordinates_and_extents() {
        let shape =
            m1_qualification_logits_shape_v1(selection(Qwen3PlanBucket::PrefillS8T128)).unwrap();
        let active = [1, 2, 3, 64, 127, 128];
        let rows = test_rows(shape, 4096, 17, &active);
        validate_rows(shape, 4096, 17, &active, &rows, |row| *row).unwrap();
        assert_eq!(rows.len(), active.len());
        for (lane, row) in rows.iter().enumerate() {
            assert_eq!(row.dispatch_generation, 17);
            assert_eq!(row.extent_bytes, shape.row_bytes());
            assert_eq!(
                row.offset_bytes,
                4096 + shape.final_row_relative_offset(lane, active[lane]).unwrap()
            );
        }
    }

    #[test]
    fn hostile_active_length_generation_offset_extent_and_count_fail_closed() {
        let shape =
            m1_qualification_logits_shape_v1(selection(Qwen3PlanBucket::DecodeS8C8192)).unwrap();
        let active = [1, 1];

        let rows = test_rows(shape, 256, 9, &active);
        assert!(matches!(
            validate_rows(shape, 256, 9, &[0, 1], &rows, |row| *row),
            Err(M1QualificationLogitsErrorV1::ActiveLength { lane: 0, .. })
        ));

        let mut rows = test_rows(shape, 256, 9, &active);
        rows[0].dispatch_generation = 10;
        assert!(matches!(
            validate_rows(shape, 256, 9, &active, &rows, |row| *row),
            Err(M1QualificationLogitsErrorV1::DispatchGeneration { lane: 0, .. })
        ));

        let mut rows = test_rows(shape, 256, 9, &active);
        rows[1].offset_bytes += 2;
        assert!(matches!(
            validate_rows(shape, 256, 9, &active, &rows, |row| *row),
            Err(M1QualificationLogitsErrorV1::RowOffset { lane: 1, .. })
        ));

        let mut rows = test_rows(shape, 256, 9, &active);
        rows[0].extent_bytes -= 1;
        assert!(matches!(
            validate_rows(shape, 256, 9, &active, &rows, |row| *row),
            Err(M1QualificationLogitsErrorV1::RowExtent { lane: 0, .. })
        ));

        assert!(matches!(
            validate_rows(
                shape,
                256,
                9,
                &active,
                &[] as &[M1QualificationLogitsRowCoordinatesV1],
                |row| *row,
            ),
            Err(M1QualificationLogitsErrorV1::RowCount { .. })
        ));
    }
}
