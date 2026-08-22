//! Qualification-only host-visible capture of final target logits rows.
//!
//! Production workspaces remain device-local. Qualification explicitly opts in
//! to a second coherent output whose complete range substitutes only the target
//! `Logits` workspace binding. Completed observation narrows that allocation to
//! one final live BF16 row per scheduled lane.

use core::fmt;

use fe2o3_service_host::{
    HostDownloadRoleV1, HostVisibleAllocationV1, ServiceAllocationErrorV1, ServiceAllocationKeyV1,
    ServiceAllocationSessionV1, ServiceCompletedReadbackV1, ServiceHostDispatchRangeV1,
};
use ferric_build::{m1_step_workspace_requirements, M1StepWorkspaceRangeRole};
use ferric_spec::{Qwen3ModelRole, Qwen3PlanSelection, QWEN3_VOCABULARY_SIZE};
use sha2::{Digest, Sha256};

use crate::BoundM1CompletionOutputV1;

type QualificationLogitsAllocationKeyV1 =
    ServiceAllocationKeyV1<HostDownloadRoleV1, HostVisibleAllocationV1>;

/// BF16 byte width used by the admitted target logits workspace.
pub const M1_QUALIFICATION_LOGITS_ELEMENT_BYTES_V1: u64 = 2;
/// Exact alignment required by the existing BF16 logits kernel arguments.
pub const M1_QUALIFICATION_LOGITS_ALIGNMENT_V1: u64 = 2;

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
        validate_key_geometry(self.key, self.shape)?;
        let typed = allocations.range(
            self.key,
            0,
            self.shape.extent_bytes,
            M1_QUALIFICATION_LOGITS_ALIGNMENT_V1,
        )?;
        let range = allocations.host_dispatch_range(typed)?;
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
    let shape = m1_qualification_logits_shape_v1(selection)?;
    let requested =
        usize::try_from(shape.extent_bytes).map_err(|_| M1QualificationLogitsErrorV1::Overflow)?;
    let key = allocations.allocate_host_visible::<HostDownloadRoleV1>(requested)?;
    validate_key_geometry(key, shape)?;
    let _mapped = allocations.map_host_visible(key)?;
    let typed = allocations.range(
        key,
        0,
        shape.extent_bytes,
        M1_QUALIFICATION_LOGITS_ALIGNMENT_V1,
    )?;
    let dispatch_range = allocations.host_dispatch_range(typed)?;
    Ok(BoundM1QualificationLogitsV1 {
        shape,
        key,
        dispatch_range,
    })
}

pub(crate) fn attach_m1_qualification_logits_v1(
    allocations: &mut ServiceAllocationSessionV1,
    completion: BoundM1CompletionOutputV1,
) -> Result<BoundM1CompletionOutputV1, Box<M1QualificationLogitsAllocationFailureV1>> {
    if completion.qualification_logits().is_some() {
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

fn validate_key_geometry(
    key: QualificationLogitsAllocationKeyV1,
    shape: M1QualificationLogitsShapeV1,
) -> Result<(), M1QualificationLogitsErrorV1> {
    if key.extent_bytes() != shape.extent_bytes {
        return Err(M1QualificationLogitsErrorV1::AllocationExtent {
            expected: shape.extent_bytes,
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
mod tests {
    use ferric_spec::{Qwen3ExecutionMode, Qwen3PlanBucket};

    use super::*;

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
