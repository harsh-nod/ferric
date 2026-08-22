//! Pure validation for one completed M1 K7 output image.
//!
//! Queue custody and generation-bound copying remain in the physical queue
//! lifecycle. This module validates the copied byte image against the exact
//! scheduler roster and caller-supplied per-member semantic expectations. It
//! creates no completion authority.

use core::fmt;

use ferric_spec::completion::CompletionEpoch;
use ferric_spec::{Qwen3PlanSelection, RequestId};

use crate::{
    check_inert_completion_record, CompletionWireError, CompletionWireExpectation,
    InertCheckedCompletionRecord, M1CompletionOutputErrorV1, M1ObservedCompletionImageV1,
    M1ScheduledDispatchV1,
};

/// Fail-closed completed-output structural or semantic diagnostic.
#[derive(Debug)]
pub enum M1CompletedOutputCheckErrorV1 {
    /// Fixed-batch and completion-output selections no longer agree.
    SelectionDrift {
        /// Selection retained by physical fixed-batch custody.
        expected: Qwen3PlanSelection,
        /// Selection retained by completion-output custody.
        actual: Qwen3PlanSelection,
    },
    /// The captured observation was substituted across scheduler epochs.
    ObservationEpochDrift {
        /// Epoch bound when the completed bytes were copied.
        expected: CompletionEpoch,
        /// Epoch retained by the supplied scheduler authority.
        actual: CompletionEpoch,
    },
    /// Caller expectations did not cover exactly the scheduled live members.
    ExpectationCount {
        /// Exact scheduler member count.
        expected: usize,
        /// Supplied expectation count.
        actual: usize,
    },
    /// One expectation selected a different target graph.
    PlanSelectionDrift {
        /// Zero-based live lane.
        lane: usize,
        /// Queue-bound target selection.
        expected: Qwen3PlanSelection,
        /// Plan selection supplied for the lane.
        actual: Qwen3PlanSelection,
    },
    /// One expectation did not name the queue's scheduler-issued epoch.
    PlanEpochDrift {
        /// Zero-based live lane.
        lane: usize,
        /// Queue-bound scheduler epoch.
        expected: CompletionEpoch,
        /// Plan epoch supplied for the lane.
        actual: CompletionEpoch,
    },
    /// One expectation did not preserve the scheduler's exact member order.
    RequestOrderDrift {
        /// Zero-based live lane.
        lane: usize,
        /// Scheduler-selected request at this lane.
        expected: RequestId,
        /// Plan request supplied for this lane.
        actual: RequestId,
    },
    /// The full byte image or one record slice did not match the retained output shape.
    Output(M1CompletionOutputErrorV1),
    /// One live record failed the existing K7 wire and semantic checker.
    LiveRecord {
        /// Zero-based live lane.
        lane: usize,
        /// Exact wire or semantic failure.
        source: CompletionWireError,
    },
    /// One byte in an inactive capacity record was not canonical zero.
    InactiveRecordNonzero {
        /// Zero-based inactive lane.
        lane: usize,
        /// Byte offset within that lane's 120-byte record.
        record_offset: usize,
    },
}

impl fmt::Display for M1CompletedOutputCheckErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "M1 completed output rejected: {self:?}")
    }
}

impl std::error::Error for M1CompletedOutputCheckErrorV1 {}

/// Checked active K7 records and exact generic readback coordinates.
///
/// Inactive capacity rows are absent from `records` only after every byte in
/// those rows was checked as canonical zero. This owner intentionally does not
/// implement `Clone`.
#[must_use = "checked completion records must remain paired with exact completion custody"]
#[derive(Debug)]
pub struct M1CheckedCompletionOutputV1 {
    selection: Qwen3PlanSelection,
    epoch: CompletionEpoch,
    dispatch_generation: u64,
    data_index: usize,
    offset_bytes: u64,
    extent_bytes: u64,
    records: Box<[InertCheckedCompletionRecord]>,
}

impl M1CheckedCompletionOutputV1 {
    /// Exact physical target selection shared by every checked live record.
    #[must_use]
    pub const fn selection(&self) -> Qwen3PlanSelection {
        self.selection
    }

    /// Exact scheduler epoch shared by every checked live record.
    #[must_use]
    pub const fn epoch(&self) -> CompletionEpoch {
        self.epoch
    }

    /// Generic dispatch generation that authorized the completed copy.
    #[must_use]
    pub const fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }

    /// Generic addressless data ordinal returned for the exact bound request.
    #[must_use]
    pub const fn data_index(&self) -> usize {
        self.data_index
    }

    /// Exact checked offset within the retained host allocation.
    #[must_use]
    pub const fn offset_bytes(&self) -> u64 {
        self.offset_bytes
    }

    /// Exact checked full-output byte extent.
    #[must_use]
    pub const fn extent_bytes(&self) -> u64 {
        self.extent_bytes
    }

    /// Checked records in exact scheduler-member order.
    #[must_use]
    pub fn records(&self) -> &[InertCheckedCompletionRecord] {
        &self.records
    }
}

pub(crate) fn check_m1_completed_output_v1(
    observed: &M1ObservedCompletionImageV1,
    queue_selection: Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
    expectations: &[CompletionWireExpectation<'_>],
) -> Result<M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1> {
    if observed.selection() != queue_selection {
        return Err(M1CompletedOutputCheckErrorV1::SelectionDrift {
            expected: queue_selection,
            actual: observed.selection(),
        });
    }
    if observed.epoch() != scheduled.epoch() {
        return Err(M1CompletedOutputCheckErrorV1::ObservationEpochDrift {
            expected: observed.epoch(),
            actual: scheduled.epoch(),
        });
    }
    if expectations.len() != scheduled.member_count() {
        return Err(M1CompletedOutputCheckErrorV1::ExpectationCount {
            expected: scheduled.member_count(),
            actual: expectations.len(),
        });
    }

    let mut records = Vec::new();
    records.try_reserve_exact(expectations.len()).map_err(|_| {
        M1CompletedOutputCheckErrorV1::Output(M1CompletionOutputErrorV1::ExtentOverflow)
    })?;
    for (lane, expectation) in expectations.iter().copied().enumerate() {
        let plan = expectation.plan();
        if plan.selection() != queue_selection {
            return Err(M1CompletedOutputCheckErrorV1::PlanSelectionDrift {
                lane,
                expected: queue_selection,
                actual: plan.selection(),
            });
        }
        if plan.completion_epoch() != scheduled.epoch() {
            return Err(M1CompletedOutputCheckErrorV1::PlanEpochDrift {
                lane,
                expected: scheduled.epoch(),
                actual: plan.completion_epoch(),
            });
        }
        let expected_request = scheduled
            .member(lane)
            .expect("scheduled member count bounds the canonical roster");
        if plan.request() != expected_request {
            return Err(M1CompletedOutputCheckErrorV1::RequestOrderDrift {
                lane,
                expected: expected_request,
                actual: plan.request(),
            });
        }
        let record_bytes = observed
            .raw_record_bytes(u32::try_from(lane).unwrap_or(u32::MAX))
            .ok_or(M1CompletedOutputCheckErrorV1::Output(
                M1CompletionOutputErrorV1::SequenceOutOfRange {
                    sequences: observed.shape().sequences(),
                    actual: u32::try_from(lane).unwrap_or(u32::MAX),
                },
            ))?;
        let checked = check_inert_completion_record(record_bytes, expectation)
            .map_err(|source| M1CompletedOutputCheckErrorV1::LiveRecord { lane, source })?;
        records.push(checked);
    }

    Ok(M1CheckedCompletionOutputV1 {
        selection: queue_selection,
        epoch: scheduled.epoch(),
        dispatch_generation: observed.dispatch_generation(),
        data_index: observed.data_index(),
        offset_bytes: observed.offset_bytes(),
        extent_bytes: observed.extent_bytes(),
        records: records.into_boxed_slice(),
    })
}

#[cfg(test)]
mod tests {
    use ferric_qwen_kernels::logits::Qwen3LogitsCompactRecordLayoutV1 as Layout;
    use ferric_spec::{
        Identity, Qwen3ExecutionMode, Qwen3ModelRole, Qwen3PlanBucket, StepPlan, TokenId,
    };

    use super::*;
    use crate::{m1_completion_output_shape_v1, CompletionWireSemanticExpectation};

    const EPOCH: CompletionEpoch = CompletionEpoch::new(41);
    const OTHER_EPOCH: CompletionEpoch = CompletionEpoch::new(42);
    const REQUESTS: [RequestId; 2] = [RequestId::new(3, 7), RequestId::new(9, 2)];
    const PLAN_IDS: [Identity; 2] = [Identity::new([11; 32]), Identity::new([12; 32])];

    fn selection() -> Qwen3PlanSelection {
        Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS8C8192,
        }
    }

    fn plan(request: RequestId, epoch: CompletionEpoch, plan_id: Identity) -> StepPlan {
        StepPlan::new(request, epoch, plan_id, selection())
    }

    fn encode(
        request: RequestId,
        epoch: CompletionEpoch,
        plan_id: Identity,
        token: TokenId,
    ) -> [u8; Layout::RECORD_BYTES_USIZE] {
        let mut bytes = [0; Layout::RECORD_BYTES_USIZE];
        bytes[Layout::REQUEST_SLOT_OFFSET..Layout::REQUEST_SLOT_OFFSET + 4]
            .copy_from_slice(&request.slot().to_le_bytes());
        bytes[Layout::REQUEST_GENERATION_OFFSET..Layout::REQUEST_GENERATION_OFFSET + 4]
            .copy_from_slice(&request.generation().to_le_bytes());
        bytes[Layout::COMPLETION_EPOCH_OFFSET..Layout::COMPLETION_EPOCH_OFFSET + 8]
            .copy_from_slice(&epoch.value().to_le_bytes());
        bytes[Layout::PLAN_IDENTITY_OFFSET
            ..Layout::PLAN_IDENTITY_OFFSET + Layout::PLAN_IDENTITY_BYTES]
            .copy_from_slice(plan_id.as_bytes());
        bytes[Layout::EMITTED_TOKEN_COUNT_OFFSET] = 1;
        let token_offset = Layout::token_offset(0).expect("first token slot exists");
        bytes[token_offset..token_offset + 4].copy_from_slice(&token.to_le_bytes());
        bytes
    }

    fn exact_bytes() -> Vec<u8> {
        let shape = m1_completion_output_shape_v1(selection()).expect("S8 is supported");
        let mut bytes = vec![0; usize::try_from(shape.extent_bytes()).expect("extent fits")];
        for lane in 0..REQUESTS.len() {
            let start = lane * Layout::RECORD_BYTES_USIZE;
            let end = start + Layout::RECORD_BYTES_USIZE;
            bytes[start..end].copy_from_slice(&encode(
                REQUESTS[lane],
                EPOCH,
                PLAN_IDS[lane],
                TokenId::try_from(17 + lane).expect("test token fits"),
            ));
        }
        bytes
    }

    fn check(
        scheduled: &M1ScheduledDispatchV1,
        bytes: &[u8],
        expectations: &[CompletionWireExpectation<'_>],
    ) -> Result<M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1> {
        let observed = M1ObservedCompletionImageV1::from_bytes_for_test(
            m1_completion_output_shape_v1(selection()).expect("S8 is supported"),
            selection(),
            scheduled,
            19,
            5,
            384,
            bytes.into(),
        )
        .expect("test image is structurally observed");
        check_m1_completed_output_v1(&observed, selection(), scheduled, expectations)
    }

    #[test]
    fn exact_roster_records_and_zero_padding_are_accepted() {
        let plans = [
            plan(REQUESTS[0], EPOCH, PLAN_IDS[0]),
            plan(REQUESTS[1], EPOCH, PLAN_IDS[1]),
        ];
        let expectations = [
            CompletionWireExpectation::new(
                &plans[0],
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
            ),
            CompletionWireExpectation::new(
                &plans[1],
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 18 },
            ),
        ];
        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &REQUESTS);
        let checked = check(&scheduled, &exact_bytes(), &expectations).expect("exact image checks");
        assert_eq!(checked.selection(), selection());
        assert_eq!(checked.epoch(), EPOCH);
        assert_eq!(checked.dispatch_generation(), 19);
        assert_eq!(checked.data_index(), 5);
        assert_eq!(checked.offset_bytes(), 384);
        assert_eq!(checked.extent_bytes(), 8 * Layout::RECORD_BYTES);
        assert_eq!(checked.records().len(), REQUESTS.len());
        assert_eq!(checked.records()[0].record().request, REQUESTS[0]);
        assert_eq!(checked.records()[1].record().request, REQUESTS[1]);
    }

    #[test]
    fn roster_count_order_epoch_and_selection_drift_fail_closed() {
        let exact_plans = [
            plan(REQUESTS[0], EPOCH, PLAN_IDS[0]),
            plan(REQUESTS[1], EPOCH, PLAN_IDS[1]),
        ];
        let one = [CompletionWireExpectation::new(
            &exact_plans[0],
            CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
        )];
        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &REQUESTS);
        assert!(matches!(
            check(&scheduled, &exact_bytes(), &one),
            Err(M1CompletedOutputCheckErrorV1::ExpectationCount {
                expected: 2,
                actual: 1
            })
        ));

        let hostile_plans = [
            plan(REQUESTS[1], EPOCH, PLAN_IDS[0]),
            plan(REQUESTS[1], OTHER_EPOCH, PLAN_IDS[1]),
        ];
        let hostile = [
            CompletionWireExpectation::new(
                &hostile_plans[0],
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
            ),
            CompletionWireExpectation::new(
                &exact_plans[1],
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 18 },
            ),
        ];
        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &REQUESTS);
        assert!(matches!(
            check(&scheduled, &exact_bytes(), &hostile),
            Err(M1CompletedOutputCheckErrorV1::RequestOrderDrift { lane: 0, .. })
        ));

        let hostile = [
            CompletionWireExpectation::new(
                &exact_plans[0],
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
            ),
            CompletionWireExpectation::new(
                &hostile_plans[1],
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 18 },
            ),
        ];
        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &REQUESTS);
        assert!(matches!(
            check(&scheduled, &exact_bytes(), &hostile),
            Err(M1CompletedOutputCheckErrorV1::PlanEpochDrift { lane: 1, .. })
        ));

        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &REQUESTS);
        let wrong_selection = Qwen3PlanSelection {
            bucket: Qwen3PlanBucket::DecodeS1C8192,
            ..selection()
        };
        let observed = M1ObservedCompletionImageV1::from_bytes_for_test(
            m1_completion_output_shape_v1(selection()).expect("S8 is supported"),
            selection(),
            &scheduled,
            19,
            5,
            384,
            exact_bytes().into_boxed_slice(),
        )
        .expect("test image is structurally observed");
        assert!(matches!(
            check_m1_completed_output_v1(&observed, wrong_selection, &scheduled, &[],),
            Err(M1CompletedOutputCheckErrorV1::SelectionDrift { .. })
        ));

        let other_scheduled = M1ScheduledDispatchV1::for_test(OTHER_EPOCH, &REQUESTS);
        assert!(matches!(
            check_m1_completed_output_v1(&observed, selection(), &other_scheduled, &[],),
            Err(M1CompletedOutputCheckErrorV1::ObservationEpochDrift { .. })
        ));
    }

    #[test]
    fn observed_token_semantic_mismatch_fails_closed() {
        let plans = [
            plan(REQUESTS[0], EPOCH, PLAN_IDS[0]),
            plan(REQUESTS[1], EPOCH, PLAN_IDS[1]),
        ];
        let expectations = [
            CompletionWireExpectation::new(
                &plans[0],
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 17 },
            ),
            CompletionWireExpectation::new(
                &plans[1],
                CompletionWireSemanticExpectation::DirectFinalRow { choice: 18 },
            ),
        ];

        let mut bytes = exact_bytes();
        let token_offset = Layout::token_offset(0).expect("first token exists");
        bytes[token_offset..token_offset + 4].copy_from_slice(&99_u32.to_le_bytes());
        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &REQUESTS);
        assert!(matches!(
            check(&scheduled, &bytes, &expectations),
            Err(M1CompletedOutputCheckErrorV1::LiveRecord { lane: 0, .. })
        ));
    }
}
