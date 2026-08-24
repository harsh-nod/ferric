//! Diagnostic-only host readback for the live S1/K4 speculative choices.
//!
//! This opt-in owner substitutes exactly the four draft choice scalars and the
//! five target verification choices with coherent host-download allocations.
//! Each substituted range is sealed with an invalid-token sentinel before its
//! inspected producer/consumer sequence, so a missing device write fails closed
//! during dispatch or observation. It is deliberately restricted to
//! `SpeculativeS1K4C8192` and carries no completion, benchmark, performance, or
//! qualification authority.

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

const TOKEN_BYTES: u64 = 4;
// Every device consumer bounds-checks this value before using it as a token index.
const M1_SPECULATIVE_DIAGNOSTIC_UNWRITTEN_CHOICE_V1: u32 = u32::MAX;

/// Fixed diagnostic choice geometry for one S1/K4 round.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1SpeculativeDiagnosticChoicesShapeV1 {
    selection: Qwen3PlanSelection,
    draft_extent_bytes: u64,
    target_extent_bytes: u64,
}

impl M1SpeculativeDiagnosticChoicesShapeV1 {
    /// Exact target speculative selection.
    #[must_use]
    pub const fn selection(self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Four little-endian `u32` draft choices.
    #[must_use]
    pub const fn draft_extent_bytes(self) -> u64 {
        self.draft_extent_bytes
    }

    /// Five little-endian `u32` target choices.
    #[must_use]
    pub const fn target_extent_bytes(self) -> u64 {
        self.target_extent_bytes
    }
}

/// Allocation, binding, copy, or token-shape rejection.
#[derive(Debug)]
pub enum M1SpeculativeDiagnosticChoicesErrorV1 {
    /// The selection is not exact target S1/K4 speculation.
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
    /// Retained draft/target byte geometry is not exact `[4, 1]`/`[5, 1]`.
    ChoiceExtents {
        draft_expected: u64,
        draft_actual: u64,
        target_expected: u64,
        target_actual: u64,
    },
    /// A copied range came from another dispatch generation.
    DispatchGeneration { expected: u64, actual: u64 },
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
    target_key: ChoiceAllocationKeyV1,
    target_range: ServiceHostDispatchRangeV1,
}

impl BoundM1SpeculativeDiagnosticChoicesV1 {
    /// Exact S1/K4 geometry bound by this owner.
    #[must_use]
    pub const fn shape(&self) -> M1SpeculativeDiagnosticChoicesShapeV1 {
        self.shape
    }

    /// Initially owner-checked full draft range.
    #[must_use]
    pub const fn retained_draft_range(&self) -> ServiceHostDispatchRangeV1 {
        self.draft_range
    }

    /// Initially owner-checked full target range.
    #[must_use]
    pub const fn retained_target_range(&self) -> ServiceHostDispatchRangeV1 {
        self.target_range
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
    dispatch_generation: u64,
    draft: ServiceCompletedReadbackV1,
    draft_choices: [TokenId; M1_SPECULATIVE_DIAGNOSTIC_DRAFT_CHOICES_V1 as usize],
    draft_sha256: [u8; 32],
    target: ServiceCompletedReadbackV1,
    target_choices: [TokenId; M1_SPECULATIVE_DIAGNOSTIC_TARGET_CHOICES_V1 as usize],
    target_sha256: [u8; 32],
}

impl M1ObservedSpeculativeDiagnosticChoicesV1 {
    /// Queue generation authorizing both completed copies.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }

    /// Exact four draft proposals in iteration order.
    #[must_use]
    pub const fn draft_choices(&self) -> &[TokenId; 4] {
        &self.draft_choices
    }

    /// Exact five target choices in verification-row order.
    #[must_use]
    pub const fn target_choices(&self) -> &[TokenId; 5] {
        &self.target_choices
    }

    /// Exact copied draft bytes.
    #[must_use]
    pub fn draft_bytes(&self) -> &[u8] {
        self.draft.bytes()
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

/// Derives the only admitted diagnostic shape.
///
/// # Errors
///
/// Rejects every selection other than exact target S1/K4 speculation.
pub fn m1_speculative_diagnostic_choices_shape_v1(
    selection: Qwen3PlanSelection,
) -> Result<M1SpeculativeDiagnosticChoicesShapeV1, M1SpeculativeDiagnosticChoicesErrorV1> {
    if selection.role != Qwen3ModelRole::Target8B
        || selection.mode != Qwen3ExecutionMode::Speculative
        || selection.bucket != Qwen3PlanBucket::SpeculativeS1K4C8192
    {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::InvalidSelection { selection });
    }
    let shape = M1SpeculativeDiagnosticChoicesShapeV1 {
        selection,
        draft_extent_bytes: u64::from(M1_SPECULATIVE_DIAGNOSTIC_DRAFT_CHOICES_V1) * TOKEN_BYTES,
        target_extent_bytes: u64::from(M1_SPECULATIVE_DIAGNOSTIC_TARGET_CHOICES_V1) * TOKEN_BYTES,
    };
    validate_choice_extents(shape.draft_extent_bytes, shape.target_extent_bytes)?;
    Ok(shape)
}

fn validate_choice_extents(
    draft_actual: u64,
    target_actual: u64,
) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
    let draft_expected = u64::from(M1_SPECULATIVE_DIAGNOSTIC_DRAFT_CHOICES_V1) * TOKEN_BYTES;
    let target_expected = u64::from(M1_SPECULATIVE_DIAGNOSTIC_TARGET_CHOICES_V1) * TOKEN_BYTES;
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

fn allocate(
    allocations: &mut ServiceAllocationSessionV1,
    shape: M1SpeculativeDiagnosticChoicesShapeV1,
) -> Result<BoundM1SpeculativeDiagnosticChoicesV1, M1SpeculativeDiagnosticChoicesErrorV1> {
    let (draft_key, draft_range) = allocate_range(allocations, shape.draft_extent_bytes)?;
    let (target_key, target_range) = allocate_range(allocations, shape.target_extent_bytes)?;
    Ok(BoundM1SpeculativeDiagnosticChoicesV1 {
        shape,
        draft_key,
        draft_range,
        target_key,
        target_range,
    })
}

fn allocate_range(
    allocations: &mut ServiceAllocationSessionV1,
    extent: u64,
) -> Result<
    (ChoiceAllocationKeyV1, ServiceHostDispatchRangeV1),
    M1SpeculativeDiagnosticChoicesErrorV1,
> {
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
    Ok((key, allocations.host_dispatch_range(typed)?))
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

pub(crate) fn observe_m1_speculative_diagnostic_choices_v1(
    owner: &BoundM1SpeculativeDiagnosticChoicesV1,
    dispatch_generation: u64,
    draft: ServiceCompletedReadbackV1,
    target: ServiceCompletedReadbackV1,
) -> Result<
    M1ObservedSpeculativeDiagnosticChoicesV1,
    Box<(
        M1SpeculativeDiagnosticChoicesErrorV1,
        ServiceCompletedReadbackV1,
        ServiceCompletedReadbackV1,
    )>,
> {
    if let Err(error) = validate_choice_extents(
        owner.shape.draft_extent_bytes,
        owner.shape.target_extent_bytes,
    ) {
        return Err(Box::new((error, draft, target)));
    }
    if let Err(error) = validate_readback(
        &draft,
        owner.draft_range,
        owner.shape.draft_extent_bytes,
        dispatch_generation,
    ) {
        return Err(Box::new((error, draft, target)));
    }
    if let Err(error) = validate_readback(
        &target,
        owner.target_range,
        owner.shape.target_extent_bytes,
        dispatch_generation,
    ) {
        return Err(Box::new((error, draft, target)));
    }
    let draft_choices = match decode_choices(draft.bytes()) {
        Ok(choices) => choices,
        Err(error) => return Err(Box::new((error, draft, target))),
    };
    let target_choices = match decode_choices(target.bytes()) {
        Ok(choices) => choices,
        Err(error) => return Err(Box::new((error, draft, target))),
    };
    Ok(M1ObservedSpeculativeDiagnosticChoicesV1 {
        dispatch_generation,
        draft_sha256: Sha256::digest(draft.bytes()).into(),
        draft,
        draft_choices,
        target_sha256: Sha256::digest(target.bytes()).into(),
        target,
        target_choices,
    })
}

fn validate_readback(
    readback: &ServiceCompletedReadbackV1,
    range: ServiceHostDispatchRangeV1,
    extent: u64,
    generation: u64,
) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
    validate_readback_coordinates(
        generation,
        range.offset_bytes(),
        extent,
        readback.dispatch_generation(),
        readback.offset_bytes(),
        u64::try_from(readback.bytes().len()).unwrap_or(u64::MAX),
    )
}

fn validate_readback_coordinates(
    expected_generation: u64,
    expected_offset: u64,
    expected_extent: u64,
    actual_generation: u64,
    actual_offset: u64,
    actual_extent: u64,
) -> Result<(), M1SpeculativeDiagnosticChoicesErrorV1> {
    if actual_generation != expected_generation {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::DispatchGeneration {
            expected: expected_generation,
            actual: actual_generation,
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

fn decode_choices<const N: usize>(
    bytes: &[u8],
) -> Result<[TokenId; N], M1SpeculativeDiagnosticChoicesErrorV1> {
    let expected = N
        .checked_mul(usize::try_from(TOKEN_BYTES).expect("token width fits usize"))
        .ok_or(M1SpeculativeDiagnosticChoicesErrorV1::Overflow)?;
    if bytes.len() != expected {
        return Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackExtent {
            expected: u64::try_from(expected).unwrap_or(u64::MAX),
            actual: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }
    let mut choices = [0; N];
    for (ordinal, encoded) in bytes.chunks_exact(4).enumerate() {
        let token = u32::from_le_bytes(encoded.try_into().expect("exact u32 chunk"));
        if token >= QWEN3_VOCABULARY_SIZE {
            return Err(
                M1SpeculativeDiagnosticChoicesErrorV1::ChoiceOutOfVocabulary {
                    ordinal,
                    actual: token,
                },
            );
        }
        choices[ordinal] = token;
    }
    Ok(choices)
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
    fn shape_is_exactly_s1_k4() {
        let shape = m1_speculative_diagnostic_choices_shape_v1(selection(
            Qwen3PlanBucket::SpeculativeS1K4C8192,
        ))
        .unwrap();
        assert_eq!(shape.draft_extent_bytes(), 16);
        assert_eq!(shape.target_extent_bytes(), 20);
        assert!(m1_speculative_diagnostic_choices_shape_v1(selection(
            Qwen3PlanBucket::SpeculativeS1K8C8192,
        ))
        .is_err());
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
    fn exact_choice_extents_reject_16_or_20_byte_substitution() {
        validate_choice_extents(16, 20).unwrap();
        assert!(matches!(
            validate_choice_extents(15, 20),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ChoiceExtents { .. })
        ));
        assert!(matches!(
            validate_choice_extents(16, 21),
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
        for requested_bytes in [16, 20] {
            let image = speculative_choice_initial_image(requested_bytes).unwrap();
            assert!(image
                .chunks_exact(4)
                .all(|encoded| encoded == u32::MAX.to_le_bytes()));
        }

        let image = speculative_choice_initial_image(16).unwrap();
        assert!(matches!(
            decode_choices::<4>(&image),
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
        validate_readback_coordinates(31, 64, 16, 31, 64, 16).unwrap();
        assert!(matches!(
            validate_readback_coordinates(31, 64, 16, 32, 64, 16),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::DispatchGeneration { .. })
        ));
        assert!(matches!(
            validate_readback_coordinates(31, 64, 16, 31, 68, 16),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackOffset { .. })
        ));
        assert!(matches!(
            validate_readback_coordinates(31, 64, 16, 31, 64, 20),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ReadbackExtent { .. })
        ));
    }

    #[test]
    fn choice_decoder_rejects_extent_and_vocabulary_substitution() {
        assert_eq!(decode_choices::<4>(&[0; 15]).unwrap_err().to_string(),
            "M1 speculative diagnostic choices rejected: ReadbackExtent { expected: 16, actual: 15 }");
        let mut bytes = [0_u8; 16];
        bytes[4..8].copy_from_slice(&QWEN3_VOCABULARY_SIZE.to_le_bytes());
        assert!(matches!(
            decode_choices::<4>(&bytes),
            Err(M1SpeculativeDiagnosticChoicesErrorV1::ChoiceOutOfVocabulary { ordinal: 1, .. })
        ));
    }
}
