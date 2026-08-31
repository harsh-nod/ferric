//! Host-visible target argmax evidence for direct M1 completions.
//!
//! K6 writes the target `Choices [S,A]` workspace and K7 consumes that same
//! workspace to publish compact completion records. This opt-in owner replaces
//! only that target workspace with initialized host-download storage. After the
//! queue completes, Ferric copies each live lane's final active-row scalar and
//! joins those independently copied K6 choices to the compact K7 output.
//! It grants no queue, completion, or inference authority by itself.

use core::fmt;

use fe2o3_service_host::{
    HostDownloadRoleV1, HostVisibleAllocationV1, ServiceAllocationErrorV1, ServiceAllocationKeyV1,
    ServiceAllocationSessionV1, ServiceCompletedReadbackV1, ServiceHostDispatchRangeV1,
};
use ferric_spec::{
    Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanSelection, TokenId, QWEN3_VOCABULARY_SIZE,
};
use sha2::{Digest, Sha256};

use crate::BoundM1CompletionOutputV1;

type ChoiceAllocationKeyV1 = ServiceAllocationKeyV1<HostDownloadRoleV1, HostVisibleAllocationV1>;

/// Exact byte alignment of one device-written direct target choice.
pub const M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1: u64 = 4;

const TOKEN_BYTES: u64 = 4;
const TOKEN_BYTES_USIZE: usize = 4;
// Every device consumer bounds-checks this value before using it as a token index.
const M1_DIRECT_DIAGNOSTIC_UNWRITTEN_CHOICE_V1: u32 = u32::MAX;

/// Exact target `Choices [S,A]` geometry for one direct graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1DirectDiagnosticChoicesShapeV1 {
    selection: Qwen3PlanSelection,
    sequences: u32,
    active_tokens: u32,
    extent_bytes: u64,
}

impl M1DirectDiagnosticChoicesShapeV1 {
    /// Exact target prefill or decode selection.
    #[must_use]
    pub const fn selection(self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Fixed sequence capacity.
    #[must_use]
    pub const fn sequences(self) -> u32 {
        self.sequences
    }

    /// Fixed active-token width.
    #[must_use]
    pub const fn active_tokens(self) -> u32 {
        self.active_tokens
    }

    /// Complete `u32 [S,A]` byte extent.
    #[must_use]
    pub const fn extent_bytes(self) -> u64 {
        self.extent_bytes
    }

    /// Relative byte offset of one lane's final live choice.
    ///
    /// # Errors
    ///
    /// Rejects an out-of-capacity lane or active length outside `1..=A`.
    pub fn final_choice_relative_offset(
        self,
        lane: usize,
        active_length: u32,
    ) -> Result<u64, M1DirectDiagnosticChoicesErrorV1> {
        let lane = u64::try_from(lane).map_err(|_| M1DirectDiagnosticChoicesErrorV1::Overflow)?;
        if lane >= u64::from(self.sequences) {
            return Err(M1DirectDiagnosticChoicesErrorV1::LaneOutOfRange {
                sequences: self.sequences,
                lane: usize::try_from(lane).unwrap_or(usize::MAX),
            });
        }
        if active_length == 0 || active_length > self.active_tokens {
            return Err(M1DirectDiagnosticChoicesErrorV1::ActiveLength {
                lane: usize::try_from(lane).unwrap_or(usize::MAX),
                capacity: self.active_tokens,
                actual: active_length,
            });
        }
        lane.checked_mul(u64::from(self.active_tokens))
            .and_then(|base| base.checked_add(u64::from(active_length - 1)))
            .and_then(|element| element.checked_mul(TOKEN_BYTES))
            .ok_or(M1DirectDiagnosticChoicesErrorV1::Overflow)
    }
}

/// Allocation, binding, copy, or token-shape rejection.
#[derive(Debug)]
pub enum M1DirectDiagnosticChoicesErrorV1 {
    /// The selection is not a target prefill or decode graph.
    InvalidSelection { selection: Qwen3PlanSelection },
    /// This compact output already owns direct choice capture.
    AlreadyEnabled,
    /// Checked extent arithmetic overflowed.
    Overflow,
    /// A complete sentinel image could not be reserved on the host.
    HostInitialization { requested_bytes: usize },
    /// A retained key no longer has its exact extent.
    AllocationExtent { expected: u64, actual: u64 },
    /// A retained key cannot satisfy `u32` alignment.
    AllocationAlignment { required: u64, actual: u64 },
    /// Owner revalidation returned another host dispatch range.
    DispatchRangeDrift,
    /// A live lane is outside fixed sequence capacity.
    LaneOutOfRange { sequences: u32, lane: usize },
    /// A live lane has no valid final active row.
    ActiveLength {
        lane: usize,
        capacity: u32,
        actual: u32,
    },
    /// Active lengths and completed copies have different cardinality.
    ReadbackCount { expected: usize, actual: usize },
    /// A copied range came from another dispatch generation.
    DispatchGeneration {
        lane: usize,
        expected: u64,
        actual: u64,
    },
    /// A copied range began at another allocation offset.
    ReadbackOffset {
        lane: usize,
        expected: u64,
        actual: u64,
    },
    /// A copied scalar has another byte extent.
    ReadbackExtent {
        lane: usize,
        expected: u64,
        actual: u64,
    },
    /// A device-written choice was outside the Qwen vocabulary.
    ChoiceOutOfVocabulary { lane: usize, actual: u32 },
    /// Generic allocation, mapping, range, or completed-copy rejection.
    Allocation(ServiceAllocationErrorV1),
}

impl fmt::Display for M1DirectDiagnosticChoicesErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 direct diagnostic choices rejected: {self:?}")
    }
}

impl std::error::Error for M1DirectDiagnosticChoicesErrorV1 {}

impl From<ServiceAllocationErrorV1> for M1DirectDiagnosticChoicesErrorV1 {
    fn from(error: ServiceAllocationErrorV1) -> Self {
        Self::Allocation(error)
    }
}

/// Move-only ownership of the exact target `Choices [S,A]` allocation.
#[must_use = "direct diagnostic choice allocation custody must remain retained"]
#[derive(Debug)]
pub struct BoundM1DirectDiagnosticChoicesV1 {
    shape: M1DirectDiagnosticChoicesShapeV1,
    key: ChoiceAllocationKeyV1,
    range: ServiceHostDispatchRangeV1,
}

impl BoundM1DirectDiagnosticChoicesV1 {
    /// Exact direct target choice geometry.
    #[must_use]
    pub const fn shape(&self) -> M1DirectDiagnosticChoicesShapeV1 {
        self.shape
    }

    /// Initially owner-checked complete host dispatch range.
    #[must_use]
    pub const fn retained_range(&self) -> ServiceHostDispatchRangeV1 {
        self.range
    }

    pub(crate) fn replacement_image(&self) -> Result<Box<[u8]>, M1DirectDiagnosticChoicesErrorV1> {
        let requested = usize::try_from(self.shape.extent_bytes)
            .map_err(|_| M1DirectDiagnosticChoicesErrorV1::Overflow)?;
        initial_image(requested)
    }

    pub(crate) fn replace_retained_range(
        &mut self,
        range: ServiceHostDispatchRangeV1,
    ) -> Result<(), M1DirectDiagnosticChoicesErrorV1> {
        if range.extent_bytes() != self.shape.extent_bytes {
            return Err(M1DirectDiagnosticChoicesErrorV1::AllocationExtent {
                expected: self.shape.extent_bytes,
                actual: range.extent_bytes(),
            });
        }
        let checked = range.checked_subrange(
            0,
            self.shape.extent_bytes,
            M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
        )?;
        if checked != range {
            return Err(M1DirectDiagnosticChoicesErrorV1::DispatchRangeDrift);
        }
        self.range = range;
        Ok(())
    }

    pub(crate) fn retained_final_choice_range(
        &self,
        lane: usize,
        active_length: u32,
    ) -> Result<ServiceHostDispatchRangeV1, M1DirectDiagnosticChoicesErrorV1> {
        let relative = self
            .shape
            .final_choice_relative_offset(lane, active_length)?;
        self.range
            .checked_subrange(
                relative,
                TOKEN_BYTES,
                M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
            )
            .map_err(M1DirectDiagnosticChoicesErrorV1::from)
    }

    pub(crate) fn host_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        selection: Qwen3PlanSelection,
    ) -> Result<ServiceHostDispatchRangeV1, M1DirectDiagnosticChoicesErrorV1> {
        let actual = m1_direct_diagnostic_choices_shape_v1(selection)?;
        if actual != self.shape {
            return Err(M1DirectDiagnosticChoicesErrorV1::InvalidSelection { selection });
        }
        validate_key(self.key, self.shape.extent_bytes)?;
        let typed = allocations.range(
            self.key,
            0,
            self.shape.extent_bytes,
            M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
        )?;
        let actual = allocations.host_dispatch_range(typed)?;
        if actual != self.range {
            return Err(M1DirectDiagnosticChoicesErrorV1::DispatchRangeDrift);
        }
        Ok(actual)
    }
}

/// Allocation failure retaining the unchanged compact output.
#[must_use = "compact-output custody remains retained by this failure"]
#[derive(Debug)]
pub struct M1DirectDiagnosticChoicesAllocationFailureV1 {
    error: M1DirectDiagnosticChoicesErrorV1,
    completion: BoundM1CompletionOutputV1,
}

impl M1DirectDiagnosticChoicesAllocationFailureV1 {
    /// Exact allocation rejection.
    #[must_use]
    pub const fn error(&self) -> &M1DirectDiagnosticChoicesErrorV1 {
        &self.error
    }

    /// Recovers the diagnostic and unchanged compact output once.
    #[must_use = "the compact output remains retained"]
    pub fn into_parts(self) -> (M1DirectDiagnosticChoicesErrorV1, BoundM1CompletionOutputV1) {
        (self.error, self.completion)
    }
}

/// Exact final active-row target choices copied from one queue generation.
#[must_use = "direct diagnostic choice evidence must be reported or retained"]
#[derive(Debug)]
pub struct M1ObservedDirectDiagnosticChoicesV1 {
    dispatch_generation: u64,
    active_lengths: Box<[u32]>,
    _readbacks: Box<[ServiceCompletedReadbackV1]>,
    choices: Box<[TokenId]>,
    raw_sha256: Box<[[u8; 32]]>,
}

impl M1ObservedDirectDiagnosticChoicesV1 {
    /// Queue generation authorizing every completed scalar copy.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }

    /// Exact live-lane active lengths used to select final rows.
    #[must_use]
    pub fn active_lengths(&self) -> &[u32] {
        &self.active_lengths
    }

    /// Exact target choices in scheduler lane order.
    #[must_use]
    pub fn choices(&self) -> &[TokenId] {
        &self.choices
    }

    /// SHA-256 of each copied little-endian choice scalar.
    #[must_use]
    pub fn raw_sha256(&self) -> &[[u8; 32]] {
        &self.raw_sha256
    }
}

#[cfg(test)]
impl M1ObservedDirectDiagnosticChoicesV1 {
    pub(crate) fn for_serving_history_test(choices: Box<[TokenId]>) -> Self {
        let live_sequences = choices.len();
        Self {
            dispatch_generation: 0,
            active_lengths: vec![1; live_sequences].into_boxed_slice(),
            _readbacks: Box::new([]),
            choices,
            raw_sha256: vec![[0; 32]; live_sequences].into_boxed_slice(),
        }
    }
}

/// Derives exact target `Choices [S,A]` geometry for a direct graph.
///
/// # Errors
///
/// Rejects non-target, speculative, or otherwise unsupported selections.
pub fn m1_direct_diagnostic_choices_shape_v1(
    selection: Qwen3PlanSelection,
) -> Result<M1DirectDiagnosticChoicesShapeV1, M1DirectDiagnosticChoicesErrorV1> {
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .filter(|_| {
            selection.role == Qwen3ModelRole::Target8B
                && matches!(
                    selection.mode,
                    Qwen3ExecutionMode::Prefill | Qwen3ExecutionMode::Decode
                )
        })
        .ok_or(M1DirectDiagnosticChoicesErrorV1::InvalidSelection { selection })?;
    let extent_bytes = u64::from(dimensions.sequences)
        .checked_mul(u64::from(dimensions.active_tokens))
        .and_then(|elements| elements.checked_mul(TOKEN_BYTES))
        .ok_or(M1DirectDiagnosticChoicesErrorV1::Overflow)?;
    usize::try_from(extent_bytes).map_err(|_| M1DirectDiagnosticChoicesErrorV1::Overflow)?;
    Ok(M1DirectDiagnosticChoicesShapeV1 {
        selection,
        sequences: dimensions.sequences,
        active_tokens: dimensions.active_tokens,
        extent_bytes,
    })
}

pub(crate) fn attach_m1_direct_diagnostic_choices_v1(
    allocations: &mut ServiceAllocationSessionV1,
    completion: BoundM1CompletionOutputV1,
) -> Result<BoundM1CompletionOutputV1, Box<M1DirectDiagnosticChoicesAllocationFailureV1>> {
    if completion.direct_diagnostic_choices().is_some() {
        return Err(Box::new(M1DirectDiagnosticChoicesAllocationFailureV1 {
            error: M1DirectDiagnosticChoicesErrorV1::AlreadyEnabled,
            completion,
        }));
    }
    let shape = match m1_direct_diagnostic_choices_shape_v1(completion.shape().selection()) {
        Ok(shape) => shape,
        Err(error) => {
            return Err(Box::new(M1DirectDiagnosticChoicesAllocationFailureV1 {
                error,
                completion,
            }))
        }
    };
    match allocate(allocations, shape) {
        Ok(choices) => Ok(completion.attach_direct_diagnostic_choices(choices)),
        Err(error) => Err(Box::new(M1DirectDiagnosticChoicesAllocationFailureV1 {
            error,
            completion,
        })),
    }
}

fn allocate(
    allocations: &mut ServiceAllocationSessionV1,
    shape: M1DirectDiagnosticChoicesShapeV1,
) -> Result<BoundM1DirectDiagnosticChoicesV1, M1DirectDiagnosticChoicesErrorV1> {
    let requested = usize::try_from(shape.extent_bytes)
        .map_err(|_| M1DirectDiagnosticChoicesErrorV1::Overflow)?;
    let initialized = initial_image(requested)?;
    let key = allocations.allocate_initialized_host_visible::<HostDownloadRoleV1>(initialized)?;
    validate_key(key, shape.extent_bytes)?;
    let typed = allocations.range(
        key,
        0,
        shape.extent_bytes,
        M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
    )?;
    let range = allocations.host_dispatch_range(typed)?;
    Ok(BoundM1DirectDiagnosticChoicesV1 { shape, key, range })
}

fn initial_image(requested_bytes: usize) -> Result<Box<[u8]>, M1DirectDiagnosticChoicesErrorV1> {
    if requested_bytes == 0 || !requested_bytes.is_multiple_of(TOKEN_BYTES_USIZE) {
        return Err(M1DirectDiagnosticChoicesErrorV1::Overflow);
    }
    let sentinel = M1_DIRECT_DIAGNOSTIC_UNWRITTEN_CHOICE_V1.to_le_bytes();
    let mut image = Vec::new();
    image
        .try_reserve_exact(requested_bytes)
        .map_err(|_| M1DirectDiagnosticChoicesErrorV1::HostInitialization { requested_bytes })?;
    image.extend_from_slice(&sentinel);
    while image.len() < requested_bytes {
        let copy_len = image.len().min(requested_bytes - image.len());
        image.extend_from_within(..copy_len);
    }
    Ok(image.into_boxed_slice())
}

fn validate_key(
    key: ChoiceAllocationKeyV1,
    extent: u64,
) -> Result<(), M1DirectDiagnosticChoicesErrorV1> {
    if key.extent_bytes() != extent {
        return Err(M1DirectDiagnosticChoicesErrorV1::AllocationExtent {
            expected: extent,
            actual: key.extent_bytes(),
        });
    }
    if key.alignment() < M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1
        || !key
            .alignment()
            .is_multiple_of(M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1)
    {
        return Err(M1DirectDiagnosticChoicesErrorV1::AllocationAlignment {
            required: M1_DIRECT_DIAGNOSTIC_CHOICE_ALIGNMENT_V1,
            actual: key.alignment(),
        });
    }
    Ok(())
}

pub(crate) fn observe_m1_direct_diagnostic_choices_v1(
    owner: &BoundM1DirectDiagnosticChoicesV1,
    dispatch_generation: u64,
    active_lengths: &[u32],
    readbacks: Vec<ServiceCompletedReadbackV1>,
) -> Result<
    M1ObservedDirectDiagnosticChoicesV1,
    (
        M1DirectDiagnosticChoicesErrorV1,
        Vec<ServiceCompletedReadbackV1>,
    ),
> {
    if readbacks.len() != active_lengths.len() {
        return Err((
            M1DirectDiagnosticChoicesErrorV1::ReadbackCount {
                expected: active_lengths.len(),
                actual: readbacks.len(),
            },
            readbacks,
        ));
    }
    let mut choices = Vec::new();
    let mut hashes = Vec::new();
    if choices.try_reserve_exact(readbacks.len()).is_err()
        || hashes.try_reserve_exact(readbacks.len()).is_err()
    {
        return Err((M1DirectDiagnosticChoicesErrorV1::Overflow, readbacks));
    }
    for (lane, (active_length, readback)) in active_lengths
        .iter()
        .copied()
        .zip(readbacks.iter())
        .enumerate()
    {
        let expected = match owner.retained_final_choice_range(lane, active_length) {
            Ok(expected) => expected,
            Err(error) => return Err((error, readbacks)),
        };
        if let Err(error) = validate_readback_coordinates(
            lane,
            dispatch_generation,
            expected.offset_bytes(),
            readback.dispatch_generation(),
            readback.offset_bytes(),
            u64::try_from(readback.bytes().len()).unwrap_or(u64::MAX),
        ) {
            return Err((error, readbacks));
        }
        let bytes: [u8; TOKEN_BYTES_USIZE] = match readback.bytes().try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                return Err((
                    M1DirectDiagnosticChoicesErrorV1::ReadbackExtent {
                        lane,
                        expected: TOKEN_BYTES,
                        actual: u64::try_from(readback.bytes().len()).unwrap_or(u64::MAX),
                    },
                    readbacks,
                ))
            }
        };
        let choice = u32::from_le_bytes(bytes);
        if choice >= QWEN3_VOCABULARY_SIZE {
            return Err((
                M1DirectDiagnosticChoicesErrorV1::ChoiceOutOfVocabulary {
                    lane,
                    actual: choice,
                },
                readbacks,
            ));
        }
        choices.push(choice);
        hashes.push(Sha256::digest(bytes).into());
    }
    Ok(M1ObservedDirectDiagnosticChoicesV1 {
        dispatch_generation,
        active_lengths: active_lengths.into(),
        _readbacks: readbacks.into_boxed_slice(),
        choices: choices.into_boxed_slice(),
        raw_sha256: hashes.into_boxed_slice(),
    })
}

fn validate_readback_coordinates(
    lane: usize,
    expected_generation: u64,
    expected_offset: u64,
    actual_generation: u64,
    actual_offset: u64,
    actual_extent: u64,
) -> Result<(), M1DirectDiagnosticChoicesErrorV1> {
    if actual_generation != expected_generation {
        return Err(M1DirectDiagnosticChoicesErrorV1::DispatchGeneration {
            lane,
            expected: expected_generation,
            actual: actual_generation,
        });
    }
    if actual_offset != expected_offset {
        return Err(M1DirectDiagnosticChoicesErrorV1::ReadbackOffset {
            lane,
            expected: expected_offset,
            actual: actual_offset,
        });
    }
    if actual_extent != TOKEN_BYTES {
        return Err(M1DirectDiagnosticChoicesErrorV1::ReadbackExtent {
            lane,
            expected: TOKEN_BYTES,
            actual: actual_extent,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferric_spec::{Qwen3PlanBucket, Qwen3PlanSelection};

    const fn selection(
        role: Qwen3ModelRole,
        mode: Qwen3ExecutionMode,
        bucket: Qwen3PlanBucket,
    ) -> Qwen3PlanSelection {
        Qwen3PlanSelection { role, mode, bucket }
    }

    #[test]
    fn every_direct_target_bucket_has_exact_choices_geometry() {
        for (mode, bucket, sequences, active_tokens) in [
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
                1,
                128,
            ),
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
                8,
                128,
            ),
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
                1,
                512,
            ),
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
                1,
                2048,
            ),
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
                1,
                1,
            ),
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
                8,
                1,
            ),
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS32C8192,
                32,
                1,
            ),
        ] {
            let shape = m1_direct_diagnostic_choices_shape_v1(selection(
                Qwen3ModelRole::Target8B,
                mode,
                bucket,
            ))
            .unwrap();
            assert_eq!(shape.sequences(), sequences);
            assert_eq!(shape.active_tokens(), active_tokens);
            assert_eq!(
                shape.extent_bytes(),
                u64::from(sequences) * u64::from(active_tokens) * TOKEN_BYTES
            );
            assert_eq!(
                shape
                    .final_choice_relative_offset(
                        usize::try_from(sequences - 1).unwrap(),
                        active_tokens,
                    )
                    .unwrap(),
                shape.extent_bytes() - TOKEN_BYTES
            );
        }
    }

    #[test]
    fn non_target_speculative_and_invalid_final_rows_fail_closed() {
        let target = selection(
            Qwen3ModelRole::Target8B,
            Qwen3ExecutionMode::Prefill,
            Qwen3PlanBucket::PrefillS8T128,
        );
        let shape = m1_direct_diagnostic_choices_shape_v1(target).unwrap();
        assert!(matches!(
            m1_direct_diagnostic_choices_shape_v1(selection(
                Qwen3ModelRole::Draft06B,
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
            )),
            Err(M1DirectDiagnosticChoicesErrorV1::InvalidSelection { .. })
        ));
        assert!(matches!(
            m1_direct_diagnostic_choices_shape_v1(selection(
                Qwen3ModelRole::Target8B,
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
            )),
            Err(M1DirectDiagnosticChoicesErrorV1::InvalidSelection { .. })
        ));
        assert!(matches!(
            shape.final_choice_relative_offset(0, 0),
            Err(M1DirectDiagnosticChoicesErrorV1::ActiveLength { .. })
        ));
        assert!(matches!(
            shape.final_choice_relative_offset(8, 1),
            Err(M1DirectDiagnosticChoicesErrorV1::LaneOutOfRange { .. })
        ));
    }

    #[test]
    fn initialized_choice_images_use_invalid_token_sentinels() {
        assert!(matches!(
            initial_image(0),
            Err(M1DirectDiagnosticChoicesErrorV1::Overflow)
        ));
        assert!(matches!(
            initial_image(6),
            Err(M1DirectDiagnosticChoicesErrorV1::Overflow)
        ));
        for requested_bytes in [4, 32, 8192] {
            let image = initial_image(requested_bytes).unwrap();
            assert_eq!(image.len(), requested_bytes);
            assert!(image
                .chunks_exact(TOKEN_BYTES_USIZE)
                .all(|encoded| encoded == u32::MAX.to_le_bytes()));
        }
    }

    #[test]
    fn readback_generation_offset_and_extent_are_exact() {
        assert!(validate_readback_coordinates(0, 7, 128, 7, 128, 4).is_ok());
        assert!(matches!(
            validate_readback_coordinates(0, 7, 128, 8, 128, 4),
            Err(M1DirectDiagnosticChoicesErrorV1::DispatchGeneration { .. })
        ));
        assert!(matches!(
            validate_readback_coordinates(0, 7, 128, 7, 132, 4),
            Err(M1DirectDiagnosticChoicesErrorV1::ReadbackOffset { .. })
        ));
        assert!(matches!(
            validate_readback_coordinates(0, 7, 128, 7, 128, 8),
            Err(M1DirectDiagnosticChoicesErrorV1::ReadbackExtent { .. })
        ));
    }
}
