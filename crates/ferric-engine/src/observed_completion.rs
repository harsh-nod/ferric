//! Inert structural observation of one completed M1 K7 output image.
//!
//! This module owns copied host bytes only. It performs bounded wire decoding
//! and canonical inactive-row checks, but deliberately does not compare records
//! with the scheduler roster, validate token semantics, or create completion
//! authority.

use core::fmt;

use fe2o3_service_host::ServiceCompletedReadbackV1;
use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{CompactCompletionRecord, Qwen3PlanSelection, TokenId};
use sha2::{Digest, Sha256};

use crate::{
    completion_wire::decode_completion_record, CompletionWireError, M1CompletionOutputErrorV1,
    M1CompletionOutputShapeV1, M1ObservedCompletionCanarySummaryV1, M1ScheduledDispatchV1,
    M1ValidatedCompletionCanaryReadbackV1,
};

#[derive(Debug)]
enum M1ObservedCompletionBackingV1 {
    Physical(ServiceCompletedReadbackV1),
    PhysicalCanary(Box<M1ValidatedCompletionCanaryReadbackV1>),
    #[allow(dead_code)]
    Test {
        dispatch_generation: u64,
        data_index: usize,
        offset_bytes: u64,
        bytes: Box<[u8]>,
    },
}

impl M1ObservedCompletionBackingV1 {
    const fn dispatch_generation(&self) -> u64 {
        match self {
            Self::Physical(readback) => readback.dispatch_generation(),
            Self::PhysicalCanary(readback) => readback.dispatch_generation(),
            Self::Test {
                dispatch_generation,
                ..
            } => *dispatch_generation,
        }
    }

    const fn data_index(&self) -> usize {
        match self {
            Self::Physical(readback) => readback.data_index(),
            Self::PhysicalCanary(readback) => readback.data_index(),
            Self::Test { data_index, .. } => *data_index,
        }
    }

    const fn offset_bytes(&self) -> u64 {
        match self {
            Self::Physical(readback) => readback.offset_bytes(),
            Self::PhysicalCanary(readback) => readback.interior_offset_bytes(),
            Self::Test { offset_bytes, .. } => *offset_bytes,
        }
    }

    fn bytes(&self) -> &[u8] {
        match self {
            Self::Physical(readback) => readback.bytes(),
            Self::PhysicalCanary(readback) => readback.interior_bytes(),
            Self::Test { bytes, .. } => bytes,
        }
    }

    const fn completion_canary(&self) -> Option<M1ObservedCompletionCanarySummaryV1> {
        match self {
            Self::PhysicalCanary(readback) => Some(readback.summary()),
            Self::Physical(_) | Self::Test { .. } => None,
        }
    }
}

/// One structurally decoded live K7 record without semantic authority.
#[derive(Debug, Eq, PartialEq)]
pub struct M1ObservedCompletionRecordV1 {
    record: CompactCompletionRecord,
}

impl M1ObservedCompletionRecordV1 {
    /// Borrows every decoded wire field.
    #[must_use]
    pub const fn record(&self) -> &CompactCompletionRecord {
        &self.record
    }

    /// Exact untrusted accepted-draft count from the wire.
    #[must_use]
    pub const fn accepted_draft_tokens(&self) -> u8 {
        self.record.accepted_draft_tokens
    }

    /// Exact untrusted emitted-token prefix from the wire.
    #[must_use]
    pub fn emitted_tokens(&self) -> &[TokenId] {
        &self.record.emitted_tokens[..usize::from(self.record.emitted_token_count)]
    }
}

/// Structural rejection while binding copied K7 bytes to retained coordinates.
#[derive(Debug)]
pub enum M1ObservedCompletionImageErrorV1 {
    /// Fixed-batch and completion-output selections no longer agree.
    SelectionDrift {
        expected: Qwen3PlanSelection,
        actual: Qwen3PlanSelection,
    },
    /// The scheduler live roster exceeds the fixed K7 output capacity.
    SchedulerCapacity { members: usize, capacity: usize },
    /// The full byte image or one record slice did not match the retained shape.
    Output(M1CompletionOutputErrorV1),
    /// One scheduled live record failed bounded wire decoding.
    LiveRecord {
        lane: usize,
        source: CompletionWireError,
    },
    /// One byte in an inactive capacity record was not canonical zero.
    InactiveRecordNonzero { lane: usize, record_offset: usize },
}

impl fmt::Display for M1ObservedCompletionImageErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 observed completion image rejected: {self:?}")
    }
}

impl std::error::Error for M1ObservedCompletionImageErrorV1 {}

/// Move-only inert byte image captured after exact queue completion and recycle.
///
/// Selection and epoch are retained host context. Records remain untrusted
/// observations until the physical lifecycle consumes this image beside exact
/// target plans and scheduler authority.
#[must_use = "observed completion bytes must be checked, reported, or retained"]
#[derive(Debug)]
pub struct M1ObservedCompletionImageV1 {
    shape: M1CompletionOutputShapeV1,
    epoch: CompletionEpoch,
    raw_sha256: [u8; 32],
    backing: M1ObservedCompletionBackingV1,
    records: Box<[M1ObservedCompletionRecordV1]>,
}

impl M1ObservedCompletionImageV1 {
    /// Exact host-retained selection for the completed fixed batch.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.shape.selection()
    }

    /// Exact scheduler-issued epoch retained beside the copied bytes.
    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    /// Generic dispatch generation that authorized the completed copy.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.backing.dispatch_generation()
    }

    /// Generic addressless data ordinal returned for the bound request.
    #[must_use]
    pub const fn data_index(&self) -> usize {
        self.backing.data_index()
    }

    /// Exact copied offset within the retained host allocation.
    #[must_use]
    pub const fn offset_bytes(&self) -> u64 {
        self.backing.offset_bytes()
    }

    /// Exact full-output byte extent.
    #[must_use]
    pub const fn extent_bytes(&self) -> u64 {
        self.shape.extent_bytes()
    }

    /// SHA-256 of the exact full copied byte image.
    #[must_use]
    pub const fn raw_sha256(&self) -> &[u8; 32] {
        &self.raw_sha256
    }

    /// Exact copied bytes, including canonical inactive records.
    #[must_use]
    pub fn raw_bytes(&self) -> &[u8] {
        self.backing.bytes()
    }

    /// Structurally decoded live records in queue lane order.
    #[must_use]
    pub fn records(&self) -> &[M1ObservedCompletionRecordV1] {
        &self.records
    }

    /// Checked adjacent-guard coordinates and digests for an opt-in guarded copy.
    #[must_use]
    pub const fn completion_canary(&self) -> Option<M1ObservedCompletionCanarySummaryV1> {
        self.backing.completion_canary()
    }

    /// Exact raw 120-byte record for one fixed-capacity lane.
    #[must_use]
    pub fn raw_record_bytes(&self, lane: u32) -> Option<&[u8]> {
        self.shape.record_bytes(self.backing.bytes(), lane).ok()
    }

    pub(crate) const fn shape(&self) -> M1CompletionOutputShapeV1 {
        self.shape
    }

    pub(crate) fn into_completion_canary_readback(
        self,
    ) -> Option<Box<M1ValidatedCompletionCanaryReadbackV1>> {
        match self.backing {
            M1ObservedCompletionBackingV1::PhysicalCanary(readback) => Some(readback),
            M1ObservedCompletionBackingV1::Physical(_)
            | M1ObservedCompletionBackingV1::Test { .. } => None,
        }
    }
}

pub(crate) fn observe_m1_completed_output_v1(
    shape: M1CompletionOutputShapeV1,
    queue_selection: Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
    readback: ServiceCompletedReadbackV1,
) -> Result<
    M1ObservedCompletionImageV1,
    (M1ObservedCompletionImageErrorV1, ServiceCompletedReadbackV1),
> {
    let observed = observe_records(shape, queue_selection, scheduled, readback.bytes());
    let (raw_sha256, records) = match observed {
        Ok(observed) => observed,
        Err(error) => return Err((error, readback)),
    };
    Ok(M1ObservedCompletionImageV1 {
        shape,
        epoch: scheduled.epoch(),
        raw_sha256,
        backing: M1ObservedCompletionBackingV1::Physical(readback),
        records,
    })
}

pub(crate) fn observe_m1_guarded_completed_output_v1(
    shape: M1CompletionOutputShapeV1,
    queue_selection: Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
    readback: Box<M1ValidatedCompletionCanaryReadbackV1>,
) -> Result<
    M1ObservedCompletionImageV1,
    (
        M1ObservedCompletionImageErrorV1,
        Box<M1ValidatedCompletionCanaryReadbackV1>,
    ),
> {
    let observed = observe_records(shape, queue_selection, scheduled, readback.interior_bytes());
    let (raw_sha256, records) = match observed {
        Ok(observed) => observed,
        Err(error) => return Err((error, readback)),
    };
    Ok(M1ObservedCompletionImageV1 {
        shape,
        epoch: scheduled.epoch(),
        raw_sha256,
        backing: M1ObservedCompletionBackingV1::PhysicalCanary(readback),
        records,
    })
}

fn observe_records(
    shape: M1CompletionOutputShapeV1,
    queue_selection: Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
    bytes: &[u8],
) -> Result<([u8; 32], Box<[M1ObservedCompletionRecordV1]>), M1ObservedCompletionImageErrorV1> {
    if shape.selection() != queue_selection {
        return Err(M1ObservedCompletionImageErrorV1::SelectionDrift {
            expected: queue_selection,
            actual: shape.selection(),
        });
    }
    let capacity = shape.sequences() as usize;
    if scheduled.member_count() > capacity {
        return Err(M1ObservedCompletionImageErrorV1::SchedulerCapacity {
            members: scheduled.member_count(),
            capacity,
        });
    }

    let mut records = Vec::new();
    records
        .try_reserve_exact(scheduled.member_count())
        .map_err(|_| {
            M1ObservedCompletionImageErrorV1::Output(M1CompletionOutputErrorV1::ExtentOverflow)
        })?;
    for lane in 0..scheduled.member_count() {
        let record_bytes = shape
            .record_bytes(bytes, u32::try_from(lane).unwrap_or(u32::MAX))
            .map_err(M1ObservedCompletionImageErrorV1::Output)?;
        let record = decode_completion_record(record_bytes)
            .map_err(|source| M1ObservedCompletionImageErrorV1::LiveRecord { lane, source })?;
        records.push(M1ObservedCompletionRecordV1 { record });
    }
    for lane in scheduled.member_count()..capacity {
        let record_bytes = shape
            .record_bytes(bytes, u32::try_from(lane).unwrap_or(u32::MAX))
            .map_err(M1ObservedCompletionImageErrorV1::Output)?;
        if let Some(record_offset) = record_bytes.iter().position(|byte| *byte != 0) {
            return Err(M1ObservedCompletionImageErrorV1::InactiveRecordNonzero {
                lane,
                record_offset,
            });
        }
    }

    let raw_sha256 = Sha256::digest(bytes).into();
    Ok((raw_sha256, records.into_boxed_slice()))
}

#[cfg(test)]
impl M1ObservedCompletionImageV1 {
    pub(crate) fn from_bytes_for_test(
        shape: M1CompletionOutputShapeV1,
        queue_selection: Qwen3PlanSelection,
        scheduled: &M1ScheduledDispatchV1,
        dispatch_generation: u64,
        data_index: usize,
        offset_bytes: u64,
        bytes: Box<[u8]>,
    ) -> Result<Self, (M1ObservedCompletionImageErrorV1, Box<[u8]>)> {
        let observed = observe_records(shape, queue_selection, scheduled, &bytes);
        let (raw_sha256, records) = match observed {
            Ok(observed) => observed,
            Err(error) => return Err((error, bytes)),
        };
        Ok(Self {
            shape,
            epoch: scheduled.epoch(),
            raw_sha256,
            backing: M1ObservedCompletionBackingV1::Test {
                dispatch_generation,
                data_index,
                offset_bytes,
                bytes,
            },
            records,
        })
    }
}

#[cfg(test)]
mod tests {
    use ferric_qwen_kernels::logits::Qwen3LogitsCompactRecordLayoutV1 as Layout;
    use ferric_spec::{Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, RequestId};

    use super::*;
    use crate::m1_completion_output_shape_v1;

    const EPOCH: CompletionEpoch = CompletionEpoch::new(31);
    const REQUEST: RequestId = RequestId::new(4, 8);

    fn selection() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS8C8192,
        }
    }

    fn exact_bytes() -> Box<[u8]> {
        let shape = m1_completion_output_shape_v1(selection()).expect("S8 shape exists");
        let mut bytes = vec![0; usize::try_from(shape.extent_bytes()).expect("extent fits")];
        bytes[Layout::REQUEST_SLOT_OFFSET..][..4].copy_from_slice(&REQUEST.slot().to_le_bytes());
        bytes[Layout::REQUEST_GENERATION_OFFSET..][..4]
            .copy_from_slice(&REQUEST.generation().to_le_bytes());
        bytes[Layout::COMPLETION_EPOCH_OFFSET..][..8].copy_from_slice(&EPOCH.value().to_le_bytes());
        bytes[Layout::PLAN_IDENTITY_OFFSET..][..Layout::PLAN_IDENTITY_BYTES]
            .copy_from_slice(Identity::new([7; 32]).as_bytes());
        bytes[Layout::EMITTED_TOKEN_COUNT_OFFSET] = 1;
        let token = Layout::token_offset(0).expect("first token exists");
        bytes[token..][..4].copy_from_slice(&23_u32.to_le_bytes());
        bytes.into_boxed_slice()
    }

    fn observe_test_bytes(
        queue_selection: Qwen3PlanSelection,
        scheduled: &M1ScheduledDispatchV1,
        bytes: Box<[u8]>,
    ) -> Result<M1ObservedCompletionImageV1, (M1ObservedCompletionImageErrorV1, Box<[u8]>)> {
        M1ObservedCompletionImageV1::from_bytes_for_test(
            m1_completion_output_shape_v1(selection()).expect("S8 shape exists"),
            queue_selection,
            scheduled,
            17,
            3,
            256,
            bytes,
        )
    }

    #[test]
    fn exact_image_decodes_raw_observations_without_semantic_authority() {
        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &[REQUEST]);
        let raw = exact_bytes();
        let observed = observe_test_bytes(selection(), &scheduled, raw.clone())
            .expect("exact structural image observes");
        let expected_sha: [u8; 32] = Sha256::digest(&raw).into();
        assert_eq!(observed.records().len(), 1);
        assert_eq!(observed.records()[0].record().request, REQUEST);
        assert_eq!(observed.records()[0].accepted_draft_tokens(), 0);
        assert_eq!(observed.records()[0].emitted_tokens(), &[23]);
        assert_eq!(observed.raw_sha256(), &expected_sha);
        assert_eq!(observed.dispatch_generation(), 17);
        assert_eq!(observed.data_index(), 3);
        assert_eq!(observed.offset_bytes(), 256);
        assert_eq!(observed.raw_record_bytes(0).map(<[u8]>::len), Some(120));
        assert!(observed.raw_record_bytes(8).is_none());
    }

    #[test]
    fn substitution_truncation_and_inactive_corruption_fail_closed() {
        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &[REQUEST]);
        let wrong_selection = Qwen3PlanSelection {
            bucket: Qwen3PlanBucket::DecodeS1C8192,
            ..selection()
        };
        let substituted = exact_bytes();
        let substituted_pointer = substituted.as_ptr();
        let (error, retained) = observe_test_bytes(wrong_selection, &scheduled, substituted)
            .expect_err("selection substitution is rejected");
        assert!(matches!(
            error,
            M1ObservedCompletionImageErrorV1::SelectionDrift { .. }
        ));
        assert_eq!(retained.as_ptr(), substituted_pointer);

        let mut truncated = exact_bytes().into_vec();
        truncated.pop();
        let truncated = truncated.into_boxed_slice();
        let truncated_pointer = truncated.as_ptr();
        let (error, retained) = observe_test_bytes(selection(), &scheduled, truncated)
            .expect_err("truncation is rejected");
        assert!(matches!(
            error,
            M1ObservedCompletionImageErrorV1::Output(
                M1CompletionOutputErrorV1::ReadbackExtentDrift { .. }
            )
        ));
        assert_eq!(retained.as_ptr(), truncated_pointer);

        let mut inactive = exact_bytes().into_vec();
        inactive[Layout::RECORD_BYTES_USIZE + 9] = 1;
        let inactive = inactive.into_boxed_slice();
        let inactive_pointer = inactive.as_ptr();
        let (error, retained) = observe_test_bytes(selection(), &scheduled, inactive)
            .expect_err("inactive corruption is rejected");
        assert!(matches!(
            error,
            M1ObservedCompletionImageErrorV1::InactiveRecordNonzero {
                lane: 1,
                record_offset: 9,
            }
        ));
        assert_eq!(retained.as_ptr(), inactive_pointer);
    }
}
