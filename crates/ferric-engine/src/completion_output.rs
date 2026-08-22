//! Typed host-visible output custody for one M1 compact-completion batch.
//!
//! K7 writes one canonical record per target sequence. This module binds that
//! Ferric shape to generic `fe2o3` host-download storage while retaining an
//! addressless range that can be used both by fixed dispatch and by completed
//! readback. It constructs no packet, publishes no queue, launches no work,
//! and grants no completion, content, inference, or refinement authority.

use core::fmt;

use fe2o3_service_host::{
    HostDownloadRoleV1, HostVisibleAllocationV1, ServiceAllocationErrorV1, ServiceAllocationKeyV1,
    ServiceAllocationSessionV1, ServiceHostDispatchRangeV1,
};
use ferric_qwen_kernels::logits::Qwen3LogitsCompactRecordLayoutV1;
use ferric_spec::{Qwen3ModelRole, Qwen3PlanSelection};

type CompletionOutputAllocationKeyV1 =
    ServiceAllocationKeyV1<HostDownloadRoleV1, HostVisibleAllocationV1>;

/// Exact alignment required by K7's compact-record output pointer.
pub const M1_COMPLETION_OUTPUT_ALIGNMENT_V1: u64 = 4;

/// Checked target selection and exact host-download byte shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1CompletionOutputShapeV1 {
    selection: Qwen3PlanSelection,
    sequences: u32,
    extent_bytes: u64,
}

impl M1CompletionOutputShapeV1 {
    /// Returns the exact target selection bound to this output.
    #[must_use]
    pub const fn selection(self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Returns the exact number of fixed sequence records.
    #[must_use]
    pub const fn sequences(self) -> u32 {
        self.sequences
    }

    /// Returns `sequences * 120` checked output bytes.
    #[must_use]
    pub const fn extent_bytes(self) -> u64 {
        self.extent_bytes
    }

    /// Revalidates that a later step still names the exact same target shape.
    ///
    /// # Errors
    ///
    /// Returns [`M1CompletionOutputErrorV1::SelectionDrift`] when the selection
    /// differs, including changes that preserve the same sequence count.
    pub fn revalidate_selection(
        self,
        selection: Qwen3PlanSelection,
    ) -> Result<(), M1CompletionOutputErrorV1> {
        let actual = m1_completion_output_shape_v1(selection)?;
        if actual != self {
            return Err(M1CompletionOutputErrorV1::SelectionDrift {
                expected: self.selection,
                actual: selection,
            });
        }
        Ok(())
    }

    /// Selects one canonical 120-byte record from an exact completed byte copy.
    ///
    /// This is only a byte-layout operation. The caller must obtain `bytes`
    /// from generic generation-checked `read_completed` custody before using
    /// the result as a completed device record.
    ///
    /// # Errors
    ///
    /// Returns [`M1CompletionOutputErrorV1::ReadbackExtentDrift`] unless the
    /// complete byte slice has this shape's exact extent, or
    /// [`M1CompletionOutputErrorV1::SequenceOutOfRange`] for an invalid lane.
    pub fn record_bytes(
        self,
        bytes: &[u8],
        sequence: u32,
    ) -> Result<&[u8], M1CompletionOutputErrorV1> {
        let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if actual != self.extent_bytes {
            return Err(M1CompletionOutputErrorV1::ReadbackExtentDrift {
                expected: self.extent_bytes,
                actual,
            });
        }
        if sequence >= self.sequences {
            return Err(M1CompletionOutputErrorV1::SequenceOutOfRange {
                sequences: self.sequences,
                actual: sequence,
            });
        }
        let record_bytes = Qwen3LogitsCompactRecordLayoutV1::RECORD_BYTES_USIZE;
        let start = usize::try_from(sequence)
            .ok()
            .and_then(|sequence| sequence.checked_mul(record_bytes))
            .ok_or(M1CompletionOutputErrorV1::ExtentOverflow)?;
        let end = start
            .checked_add(record_bytes)
            .ok_or(M1CompletionOutputErrorV1::ExtentOverflow)?;
        bytes
            .get(start..end)
            .ok_or(M1CompletionOutputErrorV1::ExtentOverflow)
    }
}

/// Fail-closed M1 completion-output allocation or shape error.
#[derive(Debug)]
pub enum M1CompletionOutputErrorV1 {
    /// The selection does not name one admitted target plan.
    InvalidTargetSelection {
        /// Rejected role/mode/bucket tuple.
        selection: Qwen3PlanSelection,
    },
    /// The exact record extent overflowed its host representation.
    ExtentOverflow,
    /// A later step selection differs from the retained allocation shape.
    SelectionDrift {
        /// Exact retained target selection.
        expected: Qwen3PlanSelection,
        /// Rejected later selection.
        actual: Qwen3PlanSelection,
    },
    /// A retained generic allocation key no longer has the exact byte extent.
    AllocationExtentDrift {
        /// Exact required bytes.
        expected: u64,
        /// Rejected key extent.
        actual: u64,
    },
    /// A retained generic allocation key cannot satisfy K7 record alignment.
    AllocationAlignmentDrift {
        /// Minimum required alignment.
        required: u64,
        /// Rejected key alignment.
        actual: u64,
    },
    /// Owner revalidation derived a different host dispatch range.
    DispatchRangeDrift,
    /// A completed byte copy differs from the exact full output extent.
    ReadbackExtentDrift {
        /// Exact required bytes.
        expected: u64,
        /// Rejected byte count.
        actual: u64,
    },
    /// A requested record lane lies outside the fixed output shape.
    SequenceOutOfRange {
        /// Exact sequence count.
        sequences: u32,
        /// Rejected zero-based lane.
        actual: u32,
    },
    /// The generic allocation owner rejected allocation, mapping, or range use.
    Allocation(ServiceAllocationErrorV1),
}

impl fmt::Display for M1CompletionOutputErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 completion output rejected: {self:?}")
    }
}

impl std::error::Error for M1CompletionOutputErrorV1 {}

impl From<ServiceAllocationErrorV1> for M1CompletionOutputErrorV1 {
    fn from(error: ServiceAllocationErrorV1) -> Self {
        Self::Allocation(error)
    }
}

/// Move-only custody of one exact M1 host-download allocation binding.
///
/// Native allocation ownership remains in the generic allocation session and
/// later moves into the queue ledger. The retained range is inert and remains
/// generation checked by both fixed-batch binding and completed readback.
///
/// ```compile_fail
/// use ferric_engine::BoundM1CompletionOutputV1;
/// fn require_clone<T: Clone>() {}
/// require_clone::<BoundM1CompletionOutputV1>();
/// ```
#[must_use = "the exact host-download allocation binding must remain retained"]
#[derive(Debug)]
pub struct BoundM1CompletionOutputV1 {
    shape: M1CompletionOutputShapeV1,
    key: CompletionOutputAllocationKeyV1,
    dispatch_range: ServiceHostDispatchRangeV1,
}

impl BoundM1CompletionOutputV1 {
    /// Returns the exact target selection and output geometry.
    #[must_use]
    pub const fn shape(&self) -> M1CompletionOutputShapeV1 {
        self.shape
    }

    /// Returns the initially owner-checked range retained for post-recycle
    /// `completed_read_request` and `read_completed`.
    #[must_use]
    pub const fn retained_host_dispatch_range(&self) -> ServiceHostDispatchRangeV1 {
        self.dispatch_range
    }

    /// Revalidates the exact selection, key geometry, owner generation, and
    /// mapped host range before fixed-dispatch construction.
    ///
    /// # Errors
    ///
    /// Returns [`M1CompletionOutputErrorV1`] for target-selection drift, key
    /// geometry drift, or generic owner/range rejection.
    pub(crate) fn host_dispatch_range(
        &self,
        allocations: &ServiceAllocationSessionV1,
        selection: Qwen3PlanSelection,
    ) -> Result<ServiceHostDispatchRangeV1, M1CompletionOutputErrorV1> {
        self.shape.revalidate_selection(selection)?;
        validate_key_geometry(self.key, self.shape)?;
        let typed = allocations.range(
            self.key,
            0,
            self.shape.extent_bytes,
            M1_COMPLETION_OUTPUT_ALIGNMENT_V1,
        )?;
        let range = allocations.host_dispatch_range(typed)?;
        if range != self.dispatch_range {
            return Err(M1CompletionOutputErrorV1::DispatchRangeDrift);
        }
        Ok(range)
    }
}

/// Derives the exact M1 compact-output geometry for one target selection.
///
/// # Errors
///
/// Returns [`M1CompletionOutputErrorV1::InvalidTargetSelection`] unless the
/// selection names an admitted target role/mode/bucket combination.
pub fn m1_completion_output_shape_v1(
    selection: Qwen3PlanSelection,
) -> Result<M1CompletionOutputShapeV1, M1CompletionOutputErrorV1> {
    let dimensions = selection
        .bucket
        .dimensions(selection.role, selection.mode)
        .filter(|_| selection.role == Qwen3ModelRole::Target8B)
        .ok_or(M1CompletionOutputErrorV1::InvalidTargetSelection { selection })?;
    let extent_bytes = u64::from(dimensions.sequences)
        .checked_mul(Qwen3LogitsCompactRecordLayoutV1::RECORD_BYTES)
        .ok_or(M1CompletionOutputErrorV1::ExtentOverflow)?;
    usize::try_from(extent_bytes).map_err(|_| M1CompletionOutputErrorV1::ExtentOverflow)?;
    Ok(M1CompletionOutputShapeV1 {
        selection,
        sequences: dimensions.sequences,
        extent_bytes,
    })
}

/// Allocates and GPU-maps one exact coherent host-download output.
///
/// Allocation and mapping remain owned by `allocations`; successful return
/// retains only the typed key, exact Ferric shape, and owner-checked addressless
/// dispatch range. A generic allocation failure leaves all native custody in
/// the allocation session according to its fail-closed phase.
///
/// # Errors
///
/// Returns [`M1CompletionOutputErrorV1`] for an invalid target selection,
/// extent conversion, generic allocation/mapping rejection, or unexpected key
/// geometry drift.
pub fn allocate_m1_completion_output_v1(
    allocations: &mut ServiceAllocationSessionV1,
    selection: Qwen3PlanSelection,
) -> Result<BoundM1CompletionOutputV1, M1CompletionOutputErrorV1> {
    let shape = m1_completion_output_shape_v1(selection)?;
    let requested_bytes = usize::try_from(shape.extent_bytes)
        .map_err(|_| M1CompletionOutputErrorV1::ExtentOverflow)?;
    let key = allocations.allocate_host_visible::<HostDownloadRoleV1>(requested_bytes)?;
    validate_key_geometry(key, shape)?;
    let _mapped = allocations.map_host_visible(key)?;
    let typed = allocations.range(
        key,
        0,
        shape.extent_bytes,
        M1_COMPLETION_OUTPUT_ALIGNMENT_V1,
    )?;
    let dispatch_range = allocations.host_dispatch_range(typed)?;
    Ok(BoundM1CompletionOutputV1 {
        shape,
        key,
        dispatch_range,
    })
}

fn validate_key_geometry(
    key: CompletionOutputAllocationKeyV1,
    shape: M1CompletionOutputShapeV1,
) -> Result<(), M1CompletionOutputErrorV1> {
    if key.extent_bytes() != shape.extent_bytes {
        return Err(M1CompletionOutputErrorV1::AllocationExtentDrift {
            expected: shape.extent_bytes,
            actual: key.extent_bytes(),
        });
    }
    if key.alignment() < M1_COMPLETION_OUTPUT_ALIGNMENT_V1
        || !key
            .alignment()
            .is_multiple_of(M1_COMPLETION_OUTPUT_ALIGNMENT_V1)
    {
        return Err(M1CompletionOutputErrorV1::AllocationAlignmentDrift {
            required: M1_COMPLETION_OUTPUT_ALIGNMENT_V1,
            actual: key.alignment(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        check_inert_completion_record, CompletionWireExpectation, CompletionWireSemanticExpectation,
    };
    use ferric_spec::{
        completion::CompletionEpoch, Identity, Qwen3ExecutionMode, Qwen3PlanBucket, RequestId,
        StepPlan, TokenId,
    };

    const fn target(mode: Qwen3ExecutionMode, bucket: Qwen3PlanBucket) -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode,
            bucket,
        }
    }

    #[test]
    fn every_target_bucket_has_exact_sequence_major_record_extent() {
        let cases = [
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T128,
                1,
            ),
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS8T128,
                8,
            ),
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T512,
                1,
            ),
            (
                Qwen3ExecutionMode::Prefill,
                Qwen3PlanBucket::PrefillS1T2048,
                1,
            ),
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS1C8192,
                1,
            ),
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS8C8192,
                8,
            ),
            (
                Qwen3ExecutionMode::Decode,
                Qwen3PlanBucket::DecodeS32C8192,
                32,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K4C8192,
                1,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS8K4C8192,
                8,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K8C8192,
                1,
            ),
            (
                Qwen3ExecutionMode::Speculative,
                Qwen3PlanBucket::SpeculativeS1K16C8192,
                1,
            ),
        ];
        for (mode, bucket, sequences) in cases {
            let selection = target(mode, bucket);
            let shape = m1_completion_output_shape_v1(selection).unwrap();
            assert_eq!(shape.selection(), selection);
            assert_eq!(shape.sequences(), sequences);
            assert_eq!(
                shape.extent_bytes(),
                u64::from(sequences) * Qwen3LogitsCompactRecordLayoutV1::RECORD_BYTES
            );
        }
    }

    #[test]
    fn invalid_and_stale_selections_fail_closed_even_when_extent_matches() {
        let draft = Qwen3PlanSelection {
            role: Qwen3ModelRole::Draft06B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        assert!(matches!(
            m1_completion_output_shape_v1(draft),
            Err(M1CompletionOutputErrorV1::InvalidTargetSelection { selection })
                if selection == draft
        ));
        let invalid = target(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::DecodeS1C8192);
        assert!(matches!(
            m1_completion_output_shape_v1(invalid),
            Err(M1CompletionOutputErrorV1::InvalidTargetSelection { selection })
                if selection == invalid
        ));

        let exact = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let stale = target(Qwen3ExecutionMode::Prefill, Qwen3PlanBucket::PrefillS1T128);
        let shape = m1_completion_output_shape_v1(exact).unwrap();
        assert_eq!(
            shape.extent_bytes(),
            m1_completion_output_shape_v1(stale).unwrap().extent_bytes()
        );
        assert!(matches!(
            shape.revalidate_selection(stale),
            Err(M1CompletionOutputErrorV1::SelectionDrift { expected, actual })
                if expected == exact && actual == stale
        ));
    }

    #[test]
    fn record_slicing_rejects_extent_and_lane_drift() {
        let shape = m1_completion_output_shape_v1(target(
            Qwen3ExecutionMode::Decode,
            Qwen3PlanBucket::DecodeS8C8192,
        ))
        .unwrap();
        let bytes = vec![0; usize::try_from(shape.extent_bytes()).unwrap()];
        for sequence in 0..shape.sequences() {
            assert_eq!(
                shape.record_bytes(&bytes, sequence).unwrap().len(),
                Qwen3LogitsCompactRecordLayoutV1::RECORD_BYTES_USIZE
            );
        }
        assert!(matches!(
            shape.record_bytes(&bytes[..bytes.len() - 1], 0),
            Err(M1CompletionOutputErrorV1::ReadbackExtentDrift { .. })
        ));
        assert!(matches!(
            shape.record_bytes(&bytes, shape.sequences()),
            Err(M1CompletionOutputErrorV1::SequenceOutOfRange { .. })
        ));
    }

    #[test]
    fn one_exact_record_slice_matches_the_existing_wire_decoder_contract() {
        let selection = target(Qwen3ExecutionMode::Decode, Qwen3PlanBucket::DecodeS1C8192);
        let shape = m1_completion_output_shape_v1(selection).unwrap();
        let request = RequestId::new(3, 7);
        let epoch = CompletionEpoch::new(11);
        let plan_id = Identity::new([9; 32]);
        let token: TokenId = 41;
        let plan = StepPlan::new(request, epoch, plan_id, selection);
        let mut bytes = vec![0; usize::try_from(shape.extent_bytes()).unwrap()];
        bytes[Qwen3LogitsCompactRecordLayoutV1::REQUEST_SLOT_OFFSET..][..4]
            .copy_from_slice(&request.slot().to_le_bytes());
        bytes[Qwen3LogitsCompactRecordLayoutV1::REQUEST_GENERATION_OFFSET..][..4]
            .copy_from_slice(&request.generation().to_le_bytes());
        bytes[Qwen3LogitsCompactRecordLayoutV1::COMPLETION_EPOCH_OFFSET..][..8]
            .copy_from_slice(&epoch.value().to_le_bytes());
        bytes[Qwen3LogitsCompactRecordLayoutV1::PLAN_IDENTITY_OFFSET..][..32]
            .copy_from_slice(plan_id.as_bytes());
        bytes[Qwen3LogitsCompactRecordLayoutV1::EMITTED_TOKEN_COUNT_OFFSET] = 1;
        let token_offset = Qwen3LogitsCompactRecordLayoutV1::token_offset(0).unwrap();
        bytes[token_offset..][..4].copy_from_slice(&token.to_le_bytes());

        let record = shape.record_bytes(&bytes, 0).unwrap();
        let checked = check_inert_completion_record(
            record,
            CompletionWireExpectation::new(
                &plan,
                CompletionWireSemanticExpectation::DirectFinalRow { choice: token },
            ),
        )
        .unwrap();
        assert_eq!(checked.record().request, request);
        assert_eq!(checked.record().epoch, epoch);
        assert_eq!(checked.record().plan_id, plan_id);
    }
}
