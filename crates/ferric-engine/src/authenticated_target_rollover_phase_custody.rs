//! Direct phase-custody witness for authenticated target-decode rollover.
//!
//! The private typestates are carried by the production rollover owners. They
//! establish source-level succession of schedule, cache-reselection,
//! preparation, and submit-entry custody tokens. They do not model or prove
//! opaque fe2o3 queue/KFD effects or the operations adjacent to those tokens.

use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum M1AuthenticatedTargetRolloverPhaseV1 {
    Scheduled,
    Reselected,
    Prepared,
    SubmitEntry,
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedTargetRolloverScheduledCustodyV1 {
    phase: M1AuthenticatedTargetRolloverPhaseV1,
}

impl M1AuthenticatedTargetRolloverScheduledCustodyV1 {
    pub(crate) fn phase(&self) -> (phase: M1AuthenticatedTargetRolloverPhaseV1)
        ensures phase == self.phase_spec(),
    {
        self.phase
    }

    pub closed spec fn phase_spec(&self) -> M1AuthenticatedTargetRolloverPhaseV1 {
        self.phase
    }
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedTargetRolloverReselectedCustodyV1 {
    phase: M1AuthenticatedTargetRolloverPhaseV1,
}

impl M1AuthenticatedTargetRolloverReselectedCustodyV1 {
    pub(crate) fn phase(&self) -> (phase: M1AuthenticatedTargetRolloverPhaseV1)
        ensures phase == self.phase_spec(),
    {
        self.phase
    }

    pub closed spec fn phase_spec(&self) -> M1AuthenticatedTargetRolloverPhaseV1 {
        self.phase
    }
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedTargetRolloverPreparedCustodyV1 {
    phase: M1AuthenticatedTargetRolloverPhaseV1,
}

impl M1AuthenticatedTargetRolloverPreparedCustodyV1 {
    pub(crate) fn phase(&self) -> (phase: M1AuthenticatedTargetRolloverPhaseV1)
        ensures phase == self.phase_spec(),
    {
        self.phase
    }

    pub closed spec fn phase_spec(&self) -> M1AuthenticatedTargetRolloverPhaseV1 {
        self.phase
    }
}

#[derive(Debug)]
pub(crate) struct M1AuthenticatedTargetRolloverSubmitEntryCustodyV1 {
    phase: M1AuthenticatedTargetRolloverPhaseV1,
}

impl M1AuthenticatedTargetRolloverSubmitEntryCustodyV1 {
    pub(crate) fn phase(&self) -> (phase: M1AuthenticatedTargetRolloverPhaseV1)
        ensures phase == self.phase_spec(),
    {
        self.phase
    }

    pub closed spec fn phase_spec(&self) -> M1AuthenticatedTargetRolloverPhaseV1 {
        self.phase
    }
}

pub(crate) fn begin_m1_authenticated_target_rollover_scheduled_custody_v1(
) -> (result: M1AuthenticatedTargetRolloverScheduledCustodyV1)
    ensures
        result.phase_spec() == M1AuthenticatedTargetRolloverPhaseV1::Scheduled,
{
    M1AuthenticatedTargetRolloverScheduledCustodyV1 {
        phase: M1AuthenticatedTargetRolloverPhaseV1::Scheduled,
    }
}

pub(crate) fn advance_m1_authenticated_target_rollover_reselected_custody_v1(
    scheduled: M1AuthenticatedTargetRolloverScheduledCustodyV1,
) -> (result: M1AuthenticatedTargetRolloverReselectedCustodyV1)
    ensures
        result.phase_spec() == M1AuthenticatedTargetRolloverPhaseV1::Reselected,
{
    let _phase = scheduled.phase();
    M1AuthenticatedTargetRolloverReselectedCustodyV1 {
        phase: M1AuthenticatedTargetRolloverPhaseV1::Reselected,
    }
}

pub(crate) fn advance_m1_authenticated_target_rollover_prepared_custody_v1(
    reselected: M1AuthenticatedTargetRolloverReselectedCustodyV1,
) -> (result: M1AuthenticatedTargetRolloverPreparedCustodyV1)
    ensures
        result.phase_spec() == M1AuthenticatedTargetRolloverPhaseV1::Prepared,
{
    let _phase = reselected.phase();
    M1AuthenticatedTargetRolloverPreparedCustodyV1 {
        phase: M1AuthenticatedTargetRolloverPhaseV1::Prepared,
    }
}

/// Consumes the preparation typestate exactly where live submission begins.
///
/// This verified transformer covers the custody token only. The adjacent
/// preparation, packet publication, and device execution remain separate
/// source-policy and runtime obligations.
pub(crate) fn establish_m1_authenticated_target_rollover_submit_entry_custody_v1(
    prepared: M1AuthenticatedTargetRolloverPreparedCustodyV1,
) -> (result: M1AuthenticatedTargetRolloverSubmitEntryCustodyV1)
    ensures
        result.phase_spec() == M1AuthenticatedTargetRolloverPhaseV1::SubmitEntry,
{
    let _phase = prepared.phase();
    let result = M1AuthenticatedTargetRolloverSubmitEntryCustodyV1 {
        phase: M1AuthenticatedTargetRolloverPhaseV1::SubmitEntry,
    };
    let _phase = result.phase();
    result
}

} // verus!

#[cfg(test)]
mod source_policy_tests {
    const CUSTODY_SOURCE: &str = include_str!("authenticated_target_rollover_phase_custody.rs");
    const ROLLOVER_SOURCE: &str = include_str!("authenticated_queue_rollover.rs");

    fn unique_offset(source: &str, needle: &str) -> usize {
        let mut matches = source.match_indices(needle);
        let Some((offset, _)) = matches.next() else {
            panic!("source-policy anchor is absent: {needle}");
        };
        assert!(
            matches.next().is_none(),
            "source-policy anchor is not unique: {needle}"
        );
        offset
    }

    #[test]
    fn phase_custody_has_no_alternate_constructor_sites() {
        for constructor in [
            "M1AuthenticatedTargetRolloverScheduledCustodyV1 {\n        phase: M1AuthenticatedTargetRolloverPhaseV1::Scheduled,",
            "M1AuthenticatedTargetRolloverReselectedCustodyV1 {\n        phase: M1AuthenticatedTargetRolloverPhaseV1::Reselected,",
            "M1AuthenticatedTargetRolloverPreparedCustodyV1 {\n        phase: M1AuthenticatedTargetRolloverPhaseV1::Prepared,",
            "M1AuthenticatedTargetRolloverSubmitEntryCustodyV1 {\n        phase: M1AuthenticatedTargetRolloverPhaseV1::SubmitEntry,",
        ] {
            assert_eq!(CUSTODY_SOURCE.matches(constructor).count(), 1, "{constructor}");
        }
    }

    #[test]
    fn source_policy_pins_phase_custody_to_success_edges() {
        let schedule_start = unique_offset(
            ROLLOVER_SOURCE,
            "pub fn schedule_m1_authenticated_target_decode_rollover_v1",
        );
        let prepare_start = unique_offset(
            ROLLOVER_SOURCE,
            "pub fn prepare_m1_authenticated_target_decode_rollover_v1",
        );
        let submit_start = unique_offset(
            ROLLOVER_SOURCE,
            "pub fn submit_m1_authenticated_target_decode_rollover_v1",
        );
        let serving_start = unique_offset(
            ROLLOVER_SOURCE,
            "pub enum M1AuthenticatedTargetDecodeServingFailureV1",
        );
        let schedule_source = &ROLLOVER_SOURCE[schedule_start..prepare_start];
        let prepare_source = &ROLLOVER_SOURCE[prepare_start..submit_start];
        let submit_source = &ROLLOVER_SOURCE[submit_start..serving_start];

        let dispatch = unique_offset(
            schedule_source,
            "let scheduled = match engine.dispatch_m1_exact_ready",
        );
        let scheduled_custody = unique_offset(
            schedule_source,
            "begin_m1_authenticated_target_rollover_scheduled_custody_v1();",
        );
        let reselect = unique_offset(
            schedule_source,
            "if let Err(source) = selected.reselect_quiescent",
        );
        let reselected_custody = unique_offset(
            schedule_source,
            "advance_m1_authenticated_target_rollover_reselected_custody_v1(phase_custody);",
        );
        let prepared = unique_offset(
            prepare_source,
            "let prepared = match crate::prepare_m1_scheduled_workspace_images_v1",
        );
        let prepared_custody = unique_offset(
            prepare_source,
            "advance_m1_authenticated_target_rollover_prepared_custody_v1(phase_custody);",
        );
        let rejected_submission = unique_offset(submit_source, "if !preflight {");
        let submit_entry_custody = unique_offset(
            submit_source,
            "establish_m1_authenticated_target_rollover_submit_entry_custody_v1(phase_custody);",
        );
        let physical_transition = unique_offset(
            submit_source,
            "let (old_shape, lower, witness, operations, custody) = queue.into_rearm_parts();",
        );

        assert!(dispatch < scheduled_custody);
        assert!(scheduled_custody < reselect);
        assert!(reselect < reselected_custody);
        assert!(prepared < prepared_custody);
        assert!(rejected_submission < submit_entry_custody);
        assert!(submit_entry_custody < physical_transition);
        assert!(schedule_start < prepare_start);
        assert!(prepare_start < submit_start);
        assert!(submit_start < serving_start);
    }
}
