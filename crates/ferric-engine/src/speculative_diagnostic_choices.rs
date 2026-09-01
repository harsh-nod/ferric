//! Diagnostic-only host readback for the live M1 speculative choices.
//!
//! This opt-in owner substitutes the exact target `[K,S]` draft-choice matrix
//! and `[S,K+1]` verification-choice matrix with coherent host-download
//! allocations.
//! Each substituted range is sealed with an invalid-token sentinel before its
//! inspected producer/consumer sequence, so a missing live-lane device write
//! fails closed during dispatch or observation. It is deliberately restricted to
//! the four finite M1 speculative buckets and carries no completion, benchmark,
//! performance, or qualification authority.

use core::fmt;

use fe2o3_service_host::{
    HostDownloadRoleV1, HostVisibleAllocationV1, ServiceAllocationErrorV1, ServiceAllocationKeyV1,
    ServiceAllocationSessionV1, ServiceCompletedReadbackV1, ServiceHostDispatchRangeV1,
};
use ferric_spec::{
    Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, Qwen3PlanSelection, TokenId,
    QWEN3_VOCABULARY_SIZE,
};
use sha2::{Digest, Sha256};

use crate::BoundM1CompletionOutputV1;

type ChoiceAllocationKeyV1 = ServiceAllocationKeyV1<HostDownloadRoleV1, HostVisibleAllocationV1>;

/// Exact byte alignment of a device-written token choice.
pub const M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1: u64 = 4;
/// Exact number of draft proposals produced by S1/K4.
pub const M1_SPECULATIVE_DIAGNOSTIC_DRAFT_CHOICES_V1: u32 = 4;
/// Exact number of target verification choices, including the bonus row.
pub const M1_SPECULATIVE_DIAGNOSTIC_TARGET_CHOICES_V1: u32 = 5;
/// Maximum speculative width admitted by the finite M1 catalog.
pub const M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1: usize = 16;

const TOKEN_BYTES: u64 = 4;
const TOKEN_BYTES_USIZE: usize = 4;
// Every device consumer bounds-checks this value before using it as a token index.
const M1_SPECULATIVE_DIAGNOSTIC_UNWRITTEN_CHOICE_V1: u32 = u32::MAX;

/// Exact diagnostic choice geometry for one finite M1 speculative round.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeDiagnosticChoicesShapeV1 {
    selection: Qwen3PlanSelection,
    sequences: u32,
    draft_tokens: u8,
    draft_extent_bytes: u64,
    target_extent_bytes: u64,
}

impl M1SpeculativeDiagnosticChoicesShapeV1 {
    /// Exact target speculative selection.
    #[must_use]
    pub const fn selection(self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Fixed sequence width `S`.
    #[must_use]
    pub const fn sequences(self) -> u32 {
        self.sequences
    }

    /// Fixed speculative width `K`.
    #[must_use]
    pub const fn draft_tokens(self) -> u8 {
        self.draft_tokens
    }

    /// Complete little-endian `u32 [K,S]` draft-choice extent.
    #[must_use]
    pub const fn draft_extent_bytes(self) -> u64 {
        self.draft_extent_bytes
    }

    /// Complete little-endian `u32 [S,K+1]` target-choice extent.
    #[must_use]
    pub const fn target_extent_bytes(self) -> u64 {
        self.target_extent_bytes
    }

    /// Byte extent of one iteration-major `[S]` draft row.
    #[must_use]
    pub const fn draft_iteration_extent_bytes(self) -> u64 {
        self.sequences as u64 * TOKEN_BYTES
    }

    /// Returns one iteration-major draft row's relative byte offset.
    #[must_use]
    pub const fn draft_iteration_relative_offset(self, iteration: u8) -> Option<u64> {
        if iteration < self.draft_tokens {
            Some(iteration as u64 * self.draft_iteration_extent_bytes())
        } else {
            None
        }
    }
}

/// Allocation, binding, copy, or token-shape rejection.
#[derive(Debug)]
pub enum M1SpeculativeDiagnosticChoicesErrorV1 {
    /// The selection is not one of the four exact target M1 speculative buckets.
    InvalidSelection { selection: Qwen3PlanSelection },
    /// This compact output already owns a diagnostic choice capture.
    AlreadyEnabled,
    /// Checked extent arithmetic overflowed.
    Overflow,
    /// A complete initialized sentinel image could not be reserved on the host.
    HostInitialization { requested_bytes: usize },
    /// A retained key no longer has its exact extent.
    AllocationExtent { expected: u64, actual: u64 },
    /// A retained key cannot satisfy `u32` alignment.
    AllocationAlignment { required: u64, actual: u64 },
    /// Owner revalidation returned another host dispatch range.
    DispatchRangeDrift,
    /// Retained draft/target byte geometry does not match exact `[K,S]`/`[S,K+1]`.
    ChoiceExtents {
        draft_expected: u64,
        draft_actual: u64,
        target_expected: u64,
        target_actual: u64,
    },
    /// The number of live scheduler lanes is outside `1..=S`.
    LiveSequenceCount { capacity: u32, actual: u32 },
    /// The number of completed draft-row copies does not equal `K`.
    DraftReadbackCount { expected: usize, actual: usize },
    /// A retained bounded draft-row slot was unexpectedly absent.
    DraftReadRangeMissing { iteration: usize },
    /// A copied range came from another dispatch generation.
    DispatchGeneration { expected: u64, actual: u64 },
    /// A copied range came from another fixed-dispatch data ordinal.
    ReadbackDataIndex { expected: usize, actual: usize },
    /// A copied range began at another allocation offset.
    ReadbackOffset { expected: u64, actual: u64 },
    /// A copied byte extent drifted.
    ReadbackExtent { expected: u64, actual: u64 },
    /// A device-written choice was outside the Qwen vocabulary.
    ChoiceOutOfVocabulary { ordinal: usize, actual: u32 },
    /// Generic allocation, mapping, range, or completed-copy rejection.
    Allocation(ServiceAllocationErrorV1),
}

impl fmt::Display for M1SpeculativeDiagnosticChoicesErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "M1 speculative diagnostic choices rejected: {self:?}"
        )
    }
}

impl std::error::Error for M1SpeculativeDiagnosticChoicesErrorV1 {}

impl From<ServiceAllocationErrorV1> for M1SpeculativeDiagnosticChoicesErrorV1 {
    fn from(error: ServiceAllocationErrorV1) -> Self {
        Self::Allocation(error)
    }
}

/// Move-only ownership of both exact diagnostic choice allocations.
#[must_use = "diagnostic choice allocation custody must remain retained"]
#[derive(Debug)]
pub struct BoundM1SpeculativeDiagnosticChoicesV1 {
    shape: M1SpeculativeDiagnosticChoicesShapeV1,
    draft_key: ChoiceAllocationKeyV1,
    draft_range: ServiceHostDispatchRangeV1,
    draft_data_index: usize,
    target_key: ChoiceAllocationKeyV1,
    target_range: ServiceHostDispatchRangeV1,
    target_data_index: usize,
}

impl BoundM1SpeculativeDiagnosticChoicesV1 {
    /// Exact finite speculative geometry bound by this owner.
    #[must_use]
    pub const fn shape(&self) -> M1SpeculativeDiagnosticChoicesShapeV1 {
        self.shape
    }

    /// Initially owner-checked full draft range.
    #[must_use]
    pub const fn retained_draft_range(&self) -> ServiceHostDispatchRangeV1 {
        self.draft_range
    }

    pub(crate) fn replacement_draft_image(
        &self,
    ) -> Result<Box<[u8]>, M1SpeculativeDiagnosticChoicesErrorV1> {
        replacement_image(self.shape.draft_extent_bytes)
    }

    pub(crate) fn replace_retained_draft_range(
        &mut self,
        range: ServiceHostDispatchRangeV1,
    ) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
        validate_replacement_range(range, self.shape.draft_extent_bytes)?;
        self.draft_range = range;
        Ok(())
    }

    pub(crate) fn retained_draft_read_ranges(
        &self,
    ) -> Result<
        [Option<ServiceHostDispatchRangeV1>; M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1],
        M1SpeculativeDiagnosticChoicesErrorV1,
    > {
        let mut ranges = [None; M1_SPECULATIVE_DIAGNOSTIC_MAX_DRAFT_TOKENS_V1];
        let extent = self.shape.draft_iteration_extent_bytes();
        for iteration in 0..self.shape.draft_tokens {
            let relative = self
                .shape
                .draft_iteration_relative_offset(iteration)
                .ok_or(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
            ranges[usize::from(iteration)] = Some(
                self.draft_range
                    .checked_subrange(
                        relative,
                        extent,
                        M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
                    )
                    .map_err(M1SpeculativeDiagnosticChoicesErrorV1::from)?,
            );
        }
        Ok(ranges)
    }

    /// Initially owner-checked full target range.
    #[must_use]
    pub const fn retained_target_range(&self) -> ServiceHostDispatchRangeV1 {
        self.target_range
    }

    pub(crate) fn replacement_target_image(
        &self,
    ) -> Result<Box<[u8]>, M1SpeculativeDiagnosticChoicesErrorV1> {
        replacement_image(self.shape.target_extent_bytes)
    }

    pub(crate) fn replace_retained_target_range(
        &mut self,
        range: ServiceHostDispatchRangeV1,
    ) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
        validate_replacement_range(range, self.shape.target_extent_bytes)?;
        self.target_range = range;
        Ok(())
    }

    pub(crate) fn host_dispatch_ranges(
        &self,
        allocations: &ServiceAllocationSessionV1,
        selection: Qwen3PlanSelection,
    ) -> Result<
        (ServiceHostDispatchRangeV1, ServiceHostDispatchRangeV1),
        M1SpeculativeDiagnosticChoicesErrorV1,
    > {
        let actual = m1_speculative_diagnostic_choices_shape_v1(selection)?;
        if actual != self.shape {
            return Err(M1SpeculativeDiagnosticChoicesErrorV1::InvalidSelection { selection });
        }
        let draft = revalidate_range(
            allocations,
            self.draft_key,
            self.shape.draft_extent_bytes,
            self.draft_range,
        )?;
        let target = revalidate_range(
            allocations,
            self.target_key,
            self.shape.target_extent_bytes,
            self.target_range,
        )?;
        Ok((draft, target))
    }
}

/// Allocation failure retaining the unchanged compact output.
#[must_use = "compact-output custody remains retained by this failure"]
#[derive(Debug)]
pub struct M1SpeculativeDiagnosticChoicesAllocationFailureV1 {
    error: M1SpeculativeDiagnosticChoicesErrorV1,
    completion: BoundM1CompletionOutputV1,
}

impl M1SpeculativeDiagnosticChoicesAllocationFailureV1 {
    /// Exact allocation rejection.
    #[must_use]
    pub const fn error(&self) -> &M1SpeculativeDiagnosticChoicesErrorV1 {
        &self.error
    }

    /// Recovers the diagnostic and unchanged compact output once.
    #[must_use = "the compact output remains retained"]
    pub fn into_parts(
        self,
    ) -> (
        M1SpeculativeDiagnosticChoicesErrorV1,
        BoundM1CompletionOutputV1,
    ) {
        (self.error, self.completion)
    }
}

/// Exact copied choice arrays from one completed dispatch generation.
#[must_use = "diagnostic choice bytes must be reported or retained"]
#[derive(Debug)]
pub struct M1ObservedSpeculativeDiagnosticChoicesV1 {
    shape: M1SpeculativeDiagnosticChoicesShapeV1,
    live_sequences: u32,
    dispatch_generation: u64,
    _draft: Box<[ServiceCompletedReadbackV1]>,
    draft_bytes: Box<[u8]>,
    draft_choice_matrix: Box<[TokenId]>,
    lane_major_draft_choices: Box<[TokenId]>,
    legacy_k4_draft_choices: [TokenId; M1_SPECULATIVE_DIAGNOSTIC_DRAFT_CHOICES_V1 as usize],
    draft_sha256: [u8; 32],
    target: ServiceCompletedReadbackV1,
    target_choice_matrix: Box<[TokenId]>,
    legacy_k4_target_choices: [TokenId; M1_SPECULATIVE_DIAGNOSTIC_TARGET_CHOICES_V1 as usize],
    target_sha256: [u8; 32],
}

impl M1ObservedSpeculativeDiagnosticChoicesV1 {
    /// Exact finite speculative geometry carried by these copied choices.
    #[must_use]
    pub const fn shape(&self) -> M1SpeculativeDiagnosticChoicesShapeV1 {
        self.shape
    }

    /// Number of leading scheduler lanes that were live for this round.
    #[must_use]
    pub const fn live_sequences(&self) -> u32 {
        self.live_sequences
    }

    /// Queue generation authorizing all `K+1` completed range copies.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }

    /// Exact four lane-zero draft proposals for the legacy S1/K4 API.
    #[must_use]
    pub const fn draft_choices(&self) -> &[TokenId; 4] {
        &self.legacy_k4_draft_choices
    }

    /// Exact five lane-zero target choices for the legacy S1/K4 API.
    #[must_use]
    pub const fn target_choices(&self) -> &[TokenId; 5] {
        &self.legacy_k4_target_choices
    }

    /// Exact physical draft words in raw iteration-major `[K,S]` order.
    ///
    /// Inactive capacity lanes are retained as non-authoritative padding.
    #[must_use]
    pub fn draft_choice_matrix(&self) -> &[TokenId] {
        &self.draft_choice_matrix
    }

    /// Exact physical target words in sequence-major `[S,K+1]` order.
    ///
    /// Inactive capacity lanes are retained as non-authoritative padding.
    #[must_use]
    pub fn target_choice_matrix(&self) -> &[TokenId] {
        &self.target_choice_matrix
    }

    /// Exact `K` draft words for one capacity lane.
    #[must_use]
    pub fn draft_choices_for_lane(&self, lane: usize) -> Option<&[TokenId]> {
        if lane >= self.shape.sequences as usize {
            return None;
        }
        let width = usize::from(self.shape.draft_tokens);
        let start = lane.checked_mul(width)?;
        self.lane_major_draft_choices
            .get(start..start.checked_add(width)?)
    }

    /// Exact `K+1` target verification words for one capacity lane.
    #[must_use]
    pub fn target_choices_for_lane(&self, lane: usize) -> Option<&[TokenId]> {
        if lane >= self.shape.sequences as usize {
            return None;
        }
        let width = usize::from(self.shape.draft_tokens) + 1;
        let start = lane.checked_mul(width)?;
        self.target_choice_matrix
            .get(start..start.checked_add(width)?)
    }

    /// Exact copied draft bytes.
    #[must_use]
    pub fn draft_bytes(&self) -> &[u8] {
        &self.draft_bytes
    }

    /// SHA-256 of the copied draft bytes.
    #[must_use]
    pub const fn draft_sha256(&self) -> &[u8; 32] {
        &self.draft_sha256
    }

    /// Exact copied target bytes.
    #[must_use]
    pub fn target_bytes(&self) -> &[u8] {
        self.target.bytes()
    }

    /// SHA-256 of the copied target bytes.
    #[must_use]
    pub const fn target_sha256(&self) -> &[u8; 32] {
        &self.target_sha256
    }
}

/// Derives one of the four admitted diagnostic shapes.
///
/// # Errors
///
/// Rejects every selection outside the finite target speculative catalog.
pub fn m1_speculative_diagnostic_choices_shape_v1(
    selection: Qwen3PlanSelection,
) -> Result<M1SpeculativeDiagnosticChoicesShapeV1, M1SpeculativeDiagnosticChoicesErrorV1> {
    if selection.role != Qwen3ModelRole::Target8B
        || selection.mode != Qwen3ExecutionMode::Speculative
    {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::InvalidSelection { selection });
    }
    let (sequences, draft_tokens) = match selection.bucket {
        Qwen3PlanBucket::SpeculativeS1K4C8192 => (1, 4),
        Qwen3PlanBucket::SpeculativeS8K4C8192 => (8, 4),
        Qwen3PlanBucket::SpeculativeS1K8C8192 => (1, 8),
        Qwen3PlanBucket::SpeculativeS1K16C8192 => (1, 16),
        _ => return Err(M1SpeculativeDiagnosticChoicesErrorV1::InvalidSelection { selection }),
    };
    let draft_extent_bytes = u64::from(sequences)
        .checked_mul(u64::from(draft_tokens))
        .and_then(|elements| elements.checked_mul(TOKEN_BYTES))
        .ok_or(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    let target_extent_bytes = u64::from(sequences)
        .checked_mul(u64::from(draft_tokens) + 1)
        .and_then(|elements| elements.checked_mul(TOKEN_BYTES))
        .ok_or(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    let shape = M1SpeculativeDiagnosticChoicesShapeV1 {
        selection,
        sequences,
        draft_tokens,
        draft_extent_bytes,
        target_extent_bytes,
    };
    validate_choice_extents(shape, shape.draft_extent_bytes, shape.target_extent_bytes)?;
    Ok(shape)
}

pub(crate) const fn m1_speculative_diagnostic_is_s1_k4_selection_v1(
    selection: Qwen3PlanSelection,
) -> bool {
    matches!(selection.role, Qwen3ModelRole::Target8B)
        && matches!(selection.mode, Qwen3ExecutionMode::Speculative)
        && matches!(selection.bucket, Qwen3PlanBucket::SpeculativeS1K4C8192)
}

fn validate_choice_extents(
    shape: M1SpeculativeDiagnosticChoicesShapeV1,
    draft_actual: u64,
    target_actual: u64,
) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
    let draft_expected = shape.draft_extent_bytes;
    let target_expected = shape.target_extent_bytes;
    if (draft_actual, target_actual) != (draft_expected, target_expected) {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::ChoiceExtents {
            draft_expected,
            draft_actual,
            target_expected,
            target_actual,
        });
    }
    Ok(())
}

pub(crate) fn attach_m1_speculative_diagnostic_choices_v1(
    allocations: &mut ServiceAllocationSessionV1,
    completion: BoundM1CompletionOutputV1,
) -> Result<BoundM1CompletionOutputV1, Box<M1SpeculativeDiagnosticChoicesAllocationFailureV1>> {
    if completion.speculative_diagnostic_choices().is_some() {
        return Err(Box::new(
            M1SpeculativeDiagnosticChoicesAllocationFailureV1 {
                error: M1SpeculativeDiagnosticChoicesErrorV1::AlreadyEnabled,
                completion,
            },
        ));
    }
    let shape = match m1_speculative_diagnostic_choices_shape_v1(completion.shape().selection()) {
        Ok(shape) => shape,
        Err(error) => {
            return Err(Box::new(
                M1SpeculativeDiagnosticChoicesAllocationFailureV1 { error, completion },
            ))
        }
    };
    match allocate(allocations, shape) {
        Ok(choices) => Ok(completion.attach_speculative_diagnostic_choices(choices)),
        Err(error) => Err(Box::new(
            M1SpeculativeDiagnosticChoicesAllocationFailureV1 { error, completion },
        )),
    }
}

pub(crate) fn attach_m1_speculative_k4_diagnostic_choices_v1(
    allocations: &mut ServiceAllocationSessionV1,
    completion: BoundM1CompletionOutputV1,
) -> Result<BoundM1CompletionOutputV1, Box<M1SpeculativeDiagnosticChoicesAllocationFailureV1>> {
    let selection = completion.shape().selection();
    if !m1_speculative_diagnostic_is_s1_k4_selection_v1(selection) {
        return Err(Box::new(
            M1SpeculativeDiagnosticChoicesAllocationFailureV1 {
                error: M1SpeculativeDiagnosticChoicesErrorV1::InvalidSelection { selection },
                completion,
            },
        ));
    }
    attach_m1_speculative_diagnostic_choices_v1(allocations, completion)
}

fn allocate(
    allocations: &mut ServiceAllocationSessionV1,
    shape: M1SpeculativeDiagnosticChoicesShapeV1,
) -> Result<BoundM1SpeculativeDiagnosticChoicesV1, M1SpeculativeDiagnosticChoicesErrorV1> {
    let (draft_key, draft_range, draft_data_index) =
        allocate_range(allocations, shape.draft_extent_bytes)?;
    let (target_key, target_range, target_data_index) =
        allocate_range(allocations, shape.target_extent_bytes)?;
    Ok(BoundM1SpeculativeDiagnosticChoicesV1 {
        shape,
        draft_key,
        draft_range,
        draft_data_index,
        target_key,
        target_range,
        target_data_index,
    })
}

fn allocate_range(
    allocations: &mut ServiceAllocationSessionV1,
    extent: u64,
) -> Result<
    (ChoiceAllocationKeyV1, ServiceHostDispatchRangeV1, usize),
    M1SpeculativeDiagnosticChoicesErrorV1,
> {
    // Ferric appends each initialized diagnostic allocation after the complete
    // mapped allocation roster. fe2o3 assigns fixed-dispatch ordinals densely
    // in that same device-then-host roster, so the pre-allocation count is an
    // independent expected ordinal retained for completed-readback checking.
    let data_index = allocations.allocation_count();
    let requested =
        usize::try_from(extent).map_err(|_| M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    let initialized = speculative_choice_initial_image(requested)?;
    let key = allocations.allocate_initialized_host_visible::<HostDownloadRoleV1>(initialized)?;
    validate_key(key, extent)?;
    let typed = allocations.range(
        key,
        0,
        extent,
        M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
    )?;
    Ok((key, allocations.host_dispatch_range(typed)?, data_index))
}

fn speculative_choice_initial_image(
    requested_bytes: usize,
) -> Result<Box<[u8]>, M1SpeculativeDiagnosticChoicesErrorV1> {
    let token_bytes = usize::try_from(TOKEN_BYTES).expect("token width fits usize");
    if requested_bytes == 0 || !requested_bytes.is_multiple_of(token_bytes) {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::Overflow);
    }
    let sentinel = M1_SPECULATIVE_DIAGNOSTIC_UNWRITTEN_CHOICE_V1.to_le_bytes();
    let mut image = Vec::new();
    image.try_reserve_exact(requested_bytes).map_err(|_| {
        M1SpeculativeDiagnosticChoicesErrorV1::HostInitialization { requested_bytes }
    })?;
    image.extend_from_slice(&sentinel);
    while image.len() < requested_bytes {
        let copy_len = image.len().min(requested_bytes - image.len());
        image.extend_from_within(..copy_len);
    }
    Ok(image.into_boxed_slice())
}

fn replacement_image(extent: u64) -> Result<Box<[u8]>, M1SpeculativeDiagnosticChoicesErrorV1> {
    let requested =
        usize::try_from(extent).map_err(|_| M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    speculative_choice_initial_image(requested)
}

fn validate_replacement_range(
    range: ServiceHostDispatchRangeV1,
    expected_extent: u64,
) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
    if range.extent_bytes() != expected_extent {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::AllocationExtent {
            expected: expected_extent,
            actual: range.extent_bytes(),
        });
    }
    let checked = range.checked_subrange(
        0,
        expected_extent,
        M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
    )?;
    if checked != range {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::DispatchRangeDrift);
    }
    Ok(())
}

fn revalidate_range(
    allocations: &ServiceAllocationSessionV1,
    key: ChoiceAllocationKeyV1,
    extent: u64,
    retained: ServiceHostDispatchRangeV1,
) -> Result<ServiceHostDispatchRangeV1, M1SpeculativeDiagnosticChoicesErrorV1> {
    validate_key(key, extent)?;
    let typed = allocations.range(
        key,
        0,
        extent,
        M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
    )?;
    let actual = allocations.host_dispatch_range(typed)?;
    if actual != retained {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::DispatchRangeDrift);
    }
    Ok(actual)
}

fn validate_key(
    key: ChoiceAllocationKeyV1,
    extent: u64,
) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
    if key.extent_bytes() != extent {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::AllocationExtent {
            expected: extent,
            actual: key.extent_bytes(),
        });
    }
    if key.alignment() < M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1
        || !key
            .alignment()
            .is_multiple_of(M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1)
    {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::AllocationAlignment {
            required: M1_SPECULATIVE_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
            actual: key.alignment(),
        });
    }
    Ok(())
}

type M1SpeculativeDiagnosticChoicesObservationResultV1 = Result<
    M1ObservedSpeculativeDiagnosticChoicesV1,
    Box<(
        M1SpeculativeDiagnosticChoicesErrorV1,
        Box<[ServiceCompletedReadbackV1]>,
        ServiceCompletedReadbackV1,
    )>,
>;

pub(crate) fn observe_m1_speculative_diagnostic_choices_v1(
    owner: &BoundM1SpeculativeDiagnosticChoicesV1,
    dispatch_generation: u64,
    live_sequences: u32,
    draft: Box<[ServiceCompletedReadbackV1]>,
    target: ServiceCompletedReadbackV1,
) -> M1SpeculativeDiagnosticChoicesObservationResultV1 {
    if let Err(error) = validate_choice_extents(
        owner.shape,
        owner.shape.draft_extent_bytes,
        owner.shape.target_extent_bytes,
    ) {
        return Err(Box::new((error, draft, target)));
    }
    if live_sequences == 0 || live_sequences > owner.shape.sequences {
        return Err(Box::new((
            M1SpeculativeDiagnosticChoicesErrorV1::LiveSequenceCount {
                capacity: owner.shape.sequences,
                actual: live_sequences,
            },
            draft,
            target,
        )));
    }
    let expected_draft_readbacks = usize::from(owner.shape.draft_tokens);
    if draft.len() != expected_draft_readbacks {
        return Err(Box::new((
            M1SpeculativeDiagnosticChoicesErrorV1::DraftReadbackCount {
                expected: expected_draft_readbacks,
                actual: draft.len(),
            },
            draft,
            target,
        )));
    }
    let draft_ranges = match owner.retained_draft_read_ranges() {
        Ok(ranges) => ranges,
        Err(error) => return Err(Box::new((error, draft, target))),
    };
    let draft_extent = match usize::try_from(owner.shape.draft_extent_bytes) {
        Ok(extent) => extent,
        Err(_) => {
            return Err(Box::new((
                M1SpeculativeDiagnosticChoicesErrorV1::Overflow,
                draft,
                target,
            )))
        }
    };
    let mut draft_bytes = Vec::new();
    if draft_bytes.try_reserve_exact(draft_extent).is_err() {
        return Err(Box::new((
            M1SpeculativeDiagnosticChoicesErrorV1::HostInitialization {
                requested_bytes: draft_extent,
            },
            draft,
            target,
        )));
    }
    let row_extent = owner.shape.draft_iteration_extent_bytes();
    for (index, readback) in draft.iter().enumerate() {
        let Some(range) = draft_ranges[index] else {
            return Err(Box::new((
                M1SpeculativeDiagnosticChoicesErrorV1::DraftReadRangeMissing { iteration: index },
                draft,
                target,
            )));
        };
        if let Err(error) = validate_readback(
            readback,
            range,
            owner.draft_data_index,
            row_extent,
            dispatch_generation,
        ) {
            return Err(Box::new((error, draft, target)));
        }
        draft_bytes.extend_from_slice(readback.bytes());
    }
    if let Err(error) = validate_readback(
        &target,
        owner.target_range,
        owner.target_data_index,
        owner.shape.target_extent_bytes,
        dispatch_generation,
    ) {
        return Err(Box::new((error, draft, target)));
    }
    let draft_choice_matrix = match decode_choice_matrix(
        &draft_bytes,
        owner.shape.sequences,
        owner.shape.draft_tokens,
        live_sequences,
        ChoiceMatrixLayout::IterationMajor,
    ) {
        Ok(choices) => choices,
        Err(error) => return Err(Box::new((error, draft, target))),
    };
    let lane_major_draft_choices = match transpose_draft_choices(
        &draft_choice_matrix,
        owner.shape.sequences,
        owner.shape.draft_tokens,
    ) {
        Ok(choices) => choices,
        Err(error) => return Err(Box::new((error, draft, target))),
    };
    let target_choice_matrix = match decode_choice_matrix(
        target.bytes(),
        owner.shape.sequences,
        owner.shape.draft_tokens + 1,
        live_sequences,
        ChoiceMatrixLayout::SequenceMajor,
    ) {
        Ok(choices) => choices,
        Err(error) => return Err(Box::new((error, draft, target))),
    };
    let legacy_k4_draft_choices = lane_major_draft_choices[..4]
        .try_into()
        .expect("every finite speculative shape has at least four lane-zero draft choices");
    let legacy_k4_target_choices = target_choice_matrix[..5]
        .try_into()
        .expect("every finite speculative shape has at least five lane-zero target choices");
    Ok(M1ObservedSpeculativeDiagnosticChoicesV1 {
        shape: owner.shape,
        live_sequences,
        dispatch_generation,
        draft_sha256: Sha256::digest(&draft_bytes).into(),
        _draft: draft,
        draft_bytes: draft_bytes.into_boxed_slice(),
        draft_choice_matrix,
        lane_major_draft_choices,
        legacy_k4_draft_choices,
        target_sha256: Sha256::digest(target.bytes()).into(),
        target,
        target_choice_matrix,
        legacy_k4_target_choices,
    })
}

fn validate_readback(
    readback: &ServiceCompletedReadbackV1,
    range: ServiceHostDispatchRangeV1,
    data_index: usize,
    extent: u64,
    generation: u64,
) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
    validate_readback_coordinates(
        (generation, data_index, range.offset_bytes(), extent),
        (
            readback.dispatch_generation(),
            readback.data_index(),
            readback.offset_bytes(),
            u64::try_from(readback.bytes().len()).unwrap_or(u64::MAX),
        ),
    )
}

fn validate_readback_coordinates(
    expected: (u64, usize, u64, u64),
    actual: (u64, usize, u64, u64),
) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
    let (expected_generation, expected_data_index, expected_offset, expected_extent) = expected;
    let (actual_generation, actual_data_index, actual_offset, actual_extent) = actual;
    if actual_generation != expected_generation {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::DispatchGeneration {
            expected: expected_generation,
            actual: actual_generation,
        });
    }
    if actual_data_index != expected_data_index {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackDataIndex {
            expected: expected_data_index,
            actual: actual_data_index,
        });
    }
    if actual_offset != expected_offset {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackOffset {
            expected: expected_offset,
            actual: actual_offset,
        });
    }
    if actual_extent != expected_extent {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackExtent {
            expected: expected_extent,
            actual: actual_extent,
        });
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum ChoiceMatrixLayout {
    IterationMajor,
    SequenceMajor,
}

fn decode_choice_matrix(
    bytes: &[u8],
    sequences: u32,
    width: u8,
    live_sequences: u32,
    layout: ChoiceMatrixLayout,
) -> Result<Box<[TokenId]>, M1SpeculativeDiagnosticChoicesErrorV1> {
    let elements = usize::try_from(sequences)
        .ok()
        .and_then(|sequences| sequences.checked_mul(usize::from(width)))
        .ok_or(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    let expected = elements
        .checked_mul(TOKEN_BYTES_USIZE)
        .ok_or(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    if bytes.len() != expected {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackExtent {
            expected: u64::try_from(expected).unwrap_or(u64::MAX),
            actual: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }
    let mut choices = Vec::new();
    choices.try_reserve_exact(elements).map_err(|_| {
        M1SpeculativeDiagnosticChoicesErrorV1::HostInitialization {
            requested_bytes: expected,
        }
    })?;
    for (ordinal, encoded) in bytes.chunks_exact(4).enumerate() {
        let token = u32::from_le_bytes(encoded.try_into().expect("exact u32 chunk"));
        let lane = match layout {
            ChoiceMatrixLayout::IterationMajor => ordinal % sequences as usize,
            ChoiceMatrixLayout::SequenceMajor => ordinal / usize::from(width),
        };
        if lane < live_sequences as usize && token >= QWEN3_VOCABULARY_SIZE {
            return Err(
                M1SpeculativeDiagnosticChoicesErrorV1::ChoiceOutOfVocabulary {
                    ordinal,
                    actual: token,
                },
            );
        }
        choices.push(token);
    }
    Ok(choices.into_boxed_slice())
}

fn transpose_draft_choices(
    iteration_major: &[TokenId],
    sequences: u32,
    draft_tokens: u8,
) -> Result<Box<[TokenId]>, M1SpeculativeDiagnosticChoicesErrorV1> {
    let sequences =
        usize::try_from(sequences).map_err(|_| M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    let draft_tokens = usize::from(draft_tokens);
    let elements = sequences
        .checked_mul(draft_tokens)
        .ok_or(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    if iteration_major.len() != elements {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackExtent {
            expected: u64::try_from(elements.saturating_mul(TOKEN_BYTES_USIZE)).unwrap_or(u64::MAX),
            actual: u64::try_from(iteration_major.len().saturating_mul(TOKEN_BYTES_USIZE))
                .unwrap_or(u64::MAX),
        });
    }
    let mut lane_major = Vec::new();
    lane_major.try_reserve_exact(elements).map_err(|_| {
        M1SpeculativeDiagnosticChoicesErrorV1::HostInitialization {
            requested_bytes: elements.saturating_mul(TOKEN_BYTES_USIZE),
        }
    })?;
    for lane in 0..sequences {
        for iteration in 0..draft_tokens {
            lane_major.push(iteration_major[iteration * sequences + lane]);
        }
    }
    Ok(lane_major.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Speculative,
            bucket,
        }
    }

    #[test]
    fn shapes_are_exactly_the_four_finite_m1_speculative_buckets() {
        for (bucket, sequences, draft_tokens, draft_bytes, target_bytes) in [
            (Qwen3PlanBucket::SpeculativeS1K4C8192, 1, 4, 16, 20),
            (Qwen3PlanBucket::SpeculativeS8K4C8192, 8, 4, 128, 160),
            (Qwen3PlanBucket::SpeculativeS1K8C8192, 1, 8, 32, 36),
            (Qwen3PlanBucket::SpeculativeS1K16C8192, 1, 16, 64, 68),
        ] {
            let shape = m1_speculative_diagnostic_choices_shape_v1(selection(bucket)).unwrap();
            assert_eq!(shape.sequences(), sequences);
            assert_eq!(shape.draft_tokens(), draft_tokens);
            assert_eq!(shape.draft_extent_bytes(), draft_bytes);
            assert_eq!(shape.target_extent_bytes(), target_bytes);
        }
        assert!(
            m1_speculative_diagnostic_choices_shape_v1(Qwen3PlanSelection {
                role: Qwen3ModelRole::Target8B,
                mode: Qwen3ExecutionMode::Decode,
                bucket: Qwen3PlanBucket::DecodeS1C8192,
            })
            .is_err()
        );
    }

    #[test]
    fn legacy_k4_gate_admits_only_exact_s1_k4() {
        assert!(m1_speculative_diagnostic_is_s1_k4_selection_v1(selection(
            Qwen3PlanBucket::SpeculativeS1K4C8192
        )));
        for bucket in [
            Qwen3PlanBucket::SpeculativeS8K4C8192,
            Qwen3PlanBucket::SpeculativeS1K8C8192,
            Qwen3PlanBucket::SpeculativeS1K16C8192,
        ] {
            assert!(!m1_speculative_diagnostic_is_s1_k4_selection_v1(selection(
                bucket
            )));
        }
    }

    #[test]
    fn exact_choice_extents_reject_16_or_20_byte_substitution() {
        let shape = m1_speculative_diagnostic_choices_shape_v1(selection(
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ))
        .unwrap();
        validate_choice_extents(shape, 16, 20).unwrap();
        assert!(matches!(
            validate_choice_extents(shape, 15, 20),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ChoiceExtents { .. })
        ));
        assert!(matches!(
            validate_choice_extents(shape, 16, 21),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ChoiceExtents { .. })
        ));
    }

    #[test]
    fn initialized_choice_images_use_invalid_token_sentinels() {
        assert!(matches!(
            speculative_choice_initial_image(0),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)
        ));
        assert!(matches!(
            speculative_choice_initial_image(15),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)
        ));
        for requested_bytes in [16, 20, 32, 36, 64, 68, 128, 160] {
            let image = speculative_choice_initial_image(requested_bytes).unwrap();
            assert!(image
                .chunks_exact(4)
                .all(|encoded| encoded == u32::MAX.to_le_bytes()));
        }

        let image = speculative_choice_initial_image(16).unwrap();
        assert!(matches!(
            decode_choice_matrix(&image, 1, 4, 1, ChoiceMatrixLayout::IterationMajor),
            Err(
                M1SpeculativeDiagnosticChoicesErrorV1::ChoiceOutOfVocabulary {
                    ordinal: 0,
                    actual: u32::MAX,
                }
            )
        ));
    }

    #[test]
    fn generation_offset_and_extent_substitution_reject() {
        validate_readback_coordinates((31, 7, 64, 16), (31, 7, 64, 16)).unwrap();
        assert!(matches!(
            validate_readback_coordinates((31, 7, 64, 16), (32, 7, 64, 16)),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::DispatchGeneration { .. })
        ));
        assert!(matches!(
            validate_readback_coordinates((31, 7, 64, 16), (31, 8, 64, 16)),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackDataIndex { .. })
        ));
        assert!(matches!(
            validate_readback_coordinates((31, 7, 64, 16), (31, 7, 68, 16)),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackOffset { .. })
        ));
        assert!(matches!(
            validate_readback_coordinates((31, 7, 64, 16), (31, 7, 64, 20)),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackExtent { .. })
        ));
    }

    #[test]
    fn choice_decoder_rejects_extent_and_vocabulary_substitution() {
        assert_eq!(decode_choice_matrix(&[0; 15], 1, 4, 1, ChoiceMatrixLayout::IterationMajor).unwrap_err().to_string(),
            "M1 speculative diagnostic choices rejected: ReadbackExtent { expected: 16, actual: 15 }");
        let mut bytes = [0_u8; 16];
        bytes[4..8].copy_from_slice(&QWEN3_VOCABULARY_SIZE.to_le_bytes());
        assert!(matches!(
            decode_choice_matrix(&bytes, 1, 4, 1, ChoiceMatrixLayout::IterationMajor),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ChoiceOutOfVocabulary { ordinal: 1, .. })
        ));
    }

    #[test]
    fn s8_decode_preserves_layout_and_ignores_inactive_capacity_rows() {
        let mut draft = speculative_choice_initial_image(128).unwrap();
        for iteration in 0..4_usize {
            for lane in 0..3_usize {
                let ordinal = iteration * 8 + lane;
                draft[ordinal * 4..ordinal * 4 + 4].copy_from_slice(
                    &(100 + u32::try_from(ordinal).expect("small test matrix ordinal"))
                        .to_le_bytes(),
                );
            }
        }
        let decoded =
            decode_choice_matrix(&draft, 8, 4, 3, ChoiceMatrixLayout::IterationMajor).unwrap();
        let transposed = transpose_draft_choices(&decoded, 8, 4).unwrap();
        assert_eq!(&transposed[..4], &[100, 108, 116, 124]);
        assert_eq!(&transposed[8..12], &[102, 110, 118, 126]);
        assert_eq!(&transposed[12..16], &[u32::MAX; 4]);

        draft[3 * 4..3 * 4 + 4].copy_from_slice(&7_u32.to_le_bytes());
        let decoded =
            decode_choice_matrix(&draft, 8, 4, 3, ChoiceMatrixLayout::IterationMajor).unwrap();
        assert_eq!(decoded[3], 7);
    }

    #[test]
    fn s8_target_matrix_authenticates_only_live_sequence_major_rows() {
        let mut target = speculative_choice_initial_image(160).unwrap();
        for lane in 0..3_usize {
            for choice in 0..5_usize {
                let ordinal = lane * 5 + choice;
                target[ordinal * 4..ordinal * 4 + 4].copy_from_slice(
                    &(200 + u32::try_from(ordinal).expect("small target matrix ordinal"))
                        .to_le_bytes(),
                );
            }
        }
        let decoded =
            decode_choice_matrix(&target, 8, 5, 3, ChoiceMatrixLayout::SequenceMajor).unwrap();
        assert_eq!(&decoded[..5], &[200, 201, 202, 203, 204]);
        assert_eq!(&decoded[10..15], &[210, 211, 212, 213, 214]);
        assert_eq!(&decoded[15..20], &[u32::MAX; 5]);

        target[15 * 4..15 * 4 + 4].copy_from_slice(&QWEN3_VOCABULARY_SIZE.to_le_bytes());
        assert!(decode_choice_matrix(&target, 8, 5, 3, ChoiceMatrixLayout::SequenceMajor).is_ok());
        target[7 * 4..7 * 4 + 4].copy_from_slice(&QWEN3_VOCABULARY_SIZE.to_le_bytes());
        assert!(matches!(
            decode_choice_matrix(&target, 8, 5, 3, ChoiceMatrixLayout::SequenceMajor),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ChoiceOutOfVocabulary { ordinal: 7, .. })
        ));
    }

    #[test]
    fn legacy_choice_accessors_retain_typed_array_signatures() {
        let _: for<'a> fn(&'a M1ObservedSpeculativeDiagnosticChoicesV1) -> &'a [TokenId; 4] =
            M1ObservedSpeculativeDiagnosticChoicesV1::draft_choices;
        let _: for<'a> fn(&'a M1ObservedSpeculativeDiagnosticChoicesV1) -> &'a [TokenId; 5] =
            M1ObservedSpeculativeDiagnosticChoicesV1::target_choices;
    }
}
