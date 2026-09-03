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
    check_inert_completion_record,
    completion_wire::check_inert_qualification_final_completion_record,
    qualification_logits::M1QualificationFinalRowChoicesV1, CompletionWireError,
    CompletionWireExpectation, CompletionWireSemanticExpectation, InertCheckedCompletionRecord,
    M1CompletionOutputErrorV1, M1ObservedCompletionCanarySummaryV1, M1ObservedCompletionImageV1,
    M1ScheduledDispatchV1, M1ValidatedCompletionCanaryReadbackV1,
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
    /// Derived terminal choices did not cover the exact scheduler roster.
    QualificationFinalChoiceCount { expected: usize, actual: usize },
    /// Copied BF16 terminal rows could not produce exact finite choices.
    QualificationFinalLogits(crate::M1QualificationFinalLogitsErrorV1),
    /// Qualification final checking escaped its target-only observation shape.
    QualificationFinalObservationShape,
    /// Generic compact checking was requested for a capture-attached non-prompt lane.
    QualificationCaptureRequiresEvidence { lane: usize },
    /// Generic compact checking was requested while direct-choice evidence was attached.
    DirectDiagnosticCaptureRequiresEvidence,
    /// Generic compact checking was requested while speculative-choice evidence was attached.
    SpeculativeDiagnosticCaptureRequiresEvidence,
    /// Evidence-derived qualification prefill was requested for another selection.
    QualificationPrefillSelection { actual: Qwen3PlanSelection },
    /// A qualification grouping did not cover the exact scheduler roster.
    QualificationMemberCount { expected: usize, actual: usize },
    /// Qualification and ordinary semantics or distinct context declarations
    /// were mixed within one fixed batch.
    QualificationContextDrift { lane: usize },
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
    raw_sha256: [u8; 32],
    completion_canary: Option<Box<M1ObservedCompletionCanarySummaryV1>>,
    completion_canary_readback: Option<Box<M1ValidatedCompletionCanaryReadbackV1>>,
    records: Box<[InertCheckedCompletionRecord]>,
    speculative_lineage: Option<
        crate::authenticated_speculative_executor::M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
    >,
    speculative_rollover_intent: Option<
        crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeRolloverPhysicalIntentV1,
    >,
}

impl M1CheckedCompletionOutputV1 {
    pub(crate) fn retain_speculative_lineage(
        mut self,
        lineage: Option<
            crate::authenticated_speculative_executor::M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
        >,
    ) -> Self {
        debug_assert!(self.speculative_lineage.is_none());
        self.speculative_lineage = lineage;
        self
    }

    pub(crate) const fn speculative_lineage(
        &self,
    ) -> Option<
        &crate::authenticated_speculative_executor::M1AuthenticatedSpeculativePhysicalLineageWitnessV1,
    >{
        self.speculative_lineage.as_ref()
    }

    pub(crate) fn retain_speculative_rollover_intent(
        mut self,
        intent: Option<
            crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeRolloverPhysicalIntentV1,
        >,
    ) -> Self {
        debug_assert!(self.speculative_rollover_intent.is_none());
        self.speculative_rollover_intent = intent;
        self
    }

    pub(crate) const fn speculative_rollover_intent(
        &self,
    ) -> Option<
        &crate::authenticated_queue_rollover::M1AuthenticatedSpeculativeRolloverPhysicalIntentV1,
    > {
        self.speculative_rollover_intent.as_ref()
    }

    pub(crate) fn retain_completion_canary_readback(
        mut self,
        readback: Option<Box<M1ValidatedCompletionCanaryReadbackV1>>,
    ) -> Self {
        debug_assert_eq!(
            self.completion_canary.as_deref().copied(),
            readback
                .as_deref()
                .map(M1ValidatedCompletionCanaryReadbackV1::summary)
        );
        self.completion_canary_readback = readback;
        self
    }

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

    /// SHA-256 of the exact full copied completion image checked into custody.
    #[must_use]
    pub const fn raw_sha256(&self) -> &[u8; 32] {
        &self.raw_sha256
    }

    /// Checked adjacent-guard observation when the completion used opt-in backing.
    #[must_use]
    pub fn completion_canary(&self) -> Option<M1ObservedCompletionCanarySummaryV1> {
        debug_assert_eq!(
            self.completion_canary.is_some(),
            self.completion_canary_readback.is_some()
        );
        self.completion_canary.as_deref().copied()
    }

    /// Checked records in exact scheduler-member order.
    #[must_use]
    pub fn records(&self) -> &[InertCheckedCompletionRecord] {
        &self.records
    }
}

#[cfg(test)]
impl M1CheckedCompletionOutputV1 {
    pub(crate) fn empty_for_rearm_test(
        selection: Qwen3PlanSelection,
        epoch: CompletionEpoch,
    ) -> Self {
        Self {
            selection,
            epoch,
            dispatch_generation: 0,
            data_index: 0,
            offset_bytes: 0,
            extent_bytes: 0,
            raw_sha256: [0; 32],
            completion_canary: None,
            completion_canary_readback: None,
            records: Box::new([]),
            speculative_lineage: None,
            speculative_rollover_intent: None,
        }
    }

    pub(crate) fn for_serving_history_test(
        selection: Qwen3PlanSelection,
        epoch: CompletionEpoch,
        records: Box<[InertCheckedCompletionRecord]>,
    ) -> Self {
        Self {
            selection,
            epoch,
            dispatch_generation: 0,
            data_index: 0,
            offset_bytes: 0,
            extent_bytes: 0,
            raw_sha256: [0; 32],
            completion_canary: None,
            completion_canary_readback: None,
            records,
            speculative_lineage: None,
            speculative_rollover_intent: None,
        }
    }
}

pub(crate) fn check_m1_completed_output_v1(
    observed: &M1ObservedCompletionImageV1,
    queue_selection: Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
    expectations: &[CompletionWireExpectation<'_>],
) -> Result<M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1> {
    check_m1_completed_output(observed, queue_selection, scheduled, expectations, None)
}

pub(crate) fn check_m1_qualification_completed_output_v1(
    observed: &M1ObservedCompletionImageV1,
    queue_selection: Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
    expectations: &[CompletionWireExpectation<'_>],
    final_rows: &M1QualificationFinalRowChoicesV1,
) -> Result<M1CheckedCompletionOutputV1, M1CompletedOutputCheckErrorV1> {
    if let Some(lane) = expectations.iter().position(|expectation| {
        !matches!(
            expectation.semantics(),
            CompletionWireSemanticExpectation::QualificationFinalRow { .. }
        )
    }) {
        return Err(M1CompletedOutputCheckErrorV1::QualificationCaptureRequiresEvidence { lane });
    }
    check_m1_completed_output(
        observed,
        queue_selection,
        scheduled,
        expectations,
        Some(final_rows),
    )
}

fn check_m1_completed_output(
    observed: &M1ObservedCompletionImageV1,
    queue_selection: Qwen3PlanSelection,
    scheduled: &M1ScheduledDispatchV1,
    expectations: &[CompletionWireExpectation<'_>],
    qualification_final_rows: Option<&M1QualificationFinalRowChoicesV1>,
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
    if let Some(final_rows) = qualification_final_rows {
        if final_rows.len() != scheduled.member_count() {
            return Err(
                M1CompletedOutputCheckErrorV1::QualificationFinalChoiceCount {
                    expected: scheduled.member_count(),
                    actual: final_rows.len(),
                },
            );
        }
    }

    let qualification_context = expectations
        .first()
        .and_then(|expectation| expectation.semantics().qualification_context());
    if let Some(first) = qualification_context {
        let expected = first.grouping().sequences() as usize;
        if scheduled.member_count() != expected {
            return Err(M1CompletedOutputCheckErrorV1::QualificationMemberCount {
                expected,
                actual: scheduled.member_count(),
            });
        }
        for (lane, expectation) in expectations.iter().copied().enumerate() {
            let Some(context) = expectation.semantics().qualification_context() else {
                return Err(M1CompletedOutputCheckErrorV1::QualificationContextDrift { lane });
            };
            if context.policy_identity() != first.policy_identity()
                || context.grouping() != first.grouping()
                || context.ordinal() != first.ordinal()
                || context.declared_workload_digest() != first.declared_workload_digest()
                || context.step() != first.step()
            {
                return Err(M1CompletedOutputCheckErrorV1::QualificationContextDrift { lane });
            }
        }
    } else if let Some(lane) = expectations
        .iter()
        .position(|expectation| expectation.semantics().qualification_context().is_some())
    {
        return Err(M1CompletedOutputCheckErrorV1::QualificationContextDrift { lane });
    }

    let mut records = Vec::new();
    records.try_reserve_exact(expectations.len()).map_err(|_| {
        M1CompletedOutputCheckErrorV1::Output(M1CompletionOutputErrorV1::ExtentOverflow)
    })?;
    for (lane, expectation) in expectations.iter().copied().enumerate() {
        let plan = expectation.plan();
        if let Some(context) = expectation.semantics().qualification_context() {
            let actual = context.lane().lane_ordinal;
            if usize::try_from(actual) != Ok(lane) {
                return Err(M1CompletedOutputCheckErrorV1::LiveRecord {
                    lane,
                    source: CompletionWireError::QualificationLaneMismatch {
                        expected: u32::try_from(lane).unwrap_or(u32::MAX),
                        actual,
                    },
                });
            }
        }
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
        let Some(expected_request) = scheduled.member(lane) else {
            return Err(M1CompletedOutputCheckErrorV1::ExpectationCount {
                expected: scheduled.member_count(),
                actual: lane,
            });
        };
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
        let checked = match (expectation.semantics(), qualification_final_rows) {
            (CompletionWireSemanticExpectation::QualificationFinalRow { .. }, Some(final_rows)) => {
                let Some(choice) = final_rows.choice(lane) else {
                    return Err(
                        M1CompletedOutputCheckErrorV1::QualificationFinalChoiceCount {
                            expected: scheduled.member_count(),
                            actual: final_rows.len(),
                        },
                    );
                };
                check_inert_qualification_final_completion_record(record_bytes, expectation, choice)
            }
            _ => check_inert_completion_record(record_bytes, expectation),
        }
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
        raw_sha256: *observed.raw_sha256(),
        completion_canary: observed.completion_canary().map(Box::new),
        completion_canary_readback: None,
        records: records.into_boxed_slice(),
        speculative_lineage: None,
        speculative_rollover_intent: None,
    })
}

#[cfg(test)]
mod tests {
    use ferric_qwen_kernels::logits::Qwen3LogitsCompactRecordLayoutV1 as Layout;
    use ferric_spec::{
        m1_qualification_context_plan, Identity, M1QualificationExecutionBindingDeclaration,
        M1QualificationLaneExecutionBinding, M1QualificationLaneGrouping, Qwen3ExecutionMode,
        Qwen3ModelRole, Qwen3PlanBucket, StepPlan, TokenId, QWEN3_VOCABULARY_SIZE,
    };

    use super::*;
    use crate::{
        m1_completion_output_shape_v1,
        qualification_logits::tests::final_row_choices_for_join_test,
        validate_m1_qualification_context_plan_v1, CompletionWireSemanticExpectation,
    };

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

    #[test]
    fn terminal_qualification_succeeds_only_with_logits_derived_choice() {
        let selection = Qwen3PlanSelection {
            role: Qwen3ModelRole::Target8B,
            mode: Qwen3ExecutionMode::Decode,
            bucket: Qwen3PlanBucket::DecodeS1C8192,
        };
        let scheduled = M1ScheduledDispatchV1::for_test(EPOCH, &REQUESTS[..1]);
        let plan = StepPlan::new(REQUESTS[0], EPOCH, PLAN_IDS[0], selection);
        let binding = M1QualificationExecutionBindingDeclaration {
            declared_workload_digest: Identity::new([31; 32]),
            ordered_lanes: vec![M1QualificationLaneExecutionBinding {
                lane_ordinal: 0,
                lane_identity: Identity::new([32; 32]),
                token_sequence_identity: Identity::new([33; 32]),
            }],
        };
        let context_plan =
            m1_qualification_context_plan(M1QualificationLaneGrouping::S1, binding.clone());
        let validated = validate_m1_qualification_context_plan_v1(
            &context_plan,
            M1QualificationLaneGrouping::S1,
            &binding,
        )
        .unwrap();
        let context = validated.step(8_191, 0).unwrap();
        let expectations = [CompletionWireExpectation::new(
            &plan,
            CompletionWireSemanticExpectation::QualificationFinalRow { context: &context },
        )];
        let observed = M1ObservedCompletionImageV1::from_bytes_for_test(
            m1_completion_output_shape_v1(selection).unwrap(),
            selection,
            &scheduled,
            19,
            5,
            384,
            encode(REQUESTS[0], EPOCH, PLAN_IDS[0], 41)
                .to_vec()
                .into_boxed_slice(),
        )
        .unwrap();
        assert!(matches!(
            check_m1_completed_output_v1(&observed, selection, &scheduled, &expectations),
            Err(M1CompletedOutputCheckErrorV1::LiveRecord {
                source: CompletionWireError::QualificationFinalRowRequiresLogitsEvidence,
                ..
            })
        ));

        let mut row = vec![0; usize::try_from(u64::from(QWEN3_VOCABULARY_SIZE) * 2).unwrap()];
        row[82..84].copy_from_slice(&0x3f80_u16.to_le_bytes());
        let choices = final_row_choices_for_join_test(&row);
        let direct_expectations = [CompletionWireExpectation::new(
            &plan,
            CompletionWireSemanticExpectation::DirectFinalRow { choice: 41 },
        )];
        assert!(matches!(
            check_m1_qualification_completed_output_v1(
                &observed,
                selection,
                &scheduled,
                &direct_expectations,
                &choices,
            ),
            Err(M1CompletedOutputCheckErrorV1::QualificationCaptureRequiresEvidence { lane: 0 })
        ));
        let checked = check_m1_qualification_completed_output_v1(
            &observed,
            selection,
            &scheduled,
            &expectations,
            &choices,
        )
        .unwrap();
        assert!(matches!(
            checked.records()[0].semantics(),
            crate::CheckedCompletionSemantics::QualificationFinalRow { token: 41, .. }
        ));

        let substituted = M1ObservedCompletionImageV1::from_bytes_for_test(
            m1_completion_output_shape_v1(selection).unwrap(),
            selection,
            &scheduled,
            19,
            5,
            384,
            encode(REQUESTS[0], EPOCH, PLAN_IDS[0], 42)
                .to_vec()
                .into_boxed_slice(),
        )
        .unwrap();
        assert!(matches!(
            check_m1_qualification_completed_output_v1(
                &substituted,
                selection,
                &scheduled,
                &expectations,
                &choices,
            ),
            Err(M1CompletedOutputCheckErrorV1::LiveRecord {
                source: CompletionWireError::DirectFinalRowMismatch {
                    expected: 41,
                    actual: 42,
                },
                ..
            })
        ));
    }
}
