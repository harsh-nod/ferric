#![forbid(unsafe_code)]

//! Bounded refinement model for one physical all-terminal new window.
//!
//! The model starts with both move-only owners present: one released
//! finite-speculative predecessor and the exact queued paired-prefill input.
//! It separates read-only admission from the provider-dequeue commit entry.
//! Admission failure preserves both opaque owner identities for retry; every
//! failure after commit entry preserves them only in terminal custody and
//! requires engine quarantine. Success transfers the same identities to a
//! paired-prefill publication with a rollover observation.
//!
//! This finite model and its source-policy tests do not prove that the
//! production implementation refines the model. In particular, they grant no
//! service, queue/KFD, GPU, artifact, numerical, performance, or M1 authority.

use vstd::prelude::*;

verus! {

/// Complete finite source-shape domain at this proof boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalNewWindowSourceShapeV1 {
    SpeculativeK4,
    SpeculativeK8,
    SpeculativeK16,
    Unsupported,
}

/// Opaque identities for the two owners that cross the commit boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1PhysicalNewWindowCustodyV1 {
    pub released_predecessor: u64,
    pub queued_prefill_input: u64,
}

/// Pure accounting state at an authenticated completed-window boundary.
///
/// Each prior window contributes exactly one inert logical archive record.
/// `active_rounds` belongs only to the completed current window and is reset
/// when its successor becomes current.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedNewWindowHistoryV1 {
    pub prior_windows: u64,
    pub inert_archive_records: u64,
    pub active_rounds: u64,
    pub current_terminal_lineage_retained: bool,
}

/// Caller-selected finite action at the pure authenticated history boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedNewWindowHistoryActionV1 {
    Retry,
    Advance,
}

/// Pure authenticated history outcome. These variants carry no runtime owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedNewWindowHistoryOutcomeV1 {
    Rejected(M1AuthenticatedNewWindowHistoryV1),
    Retained(M1AuthenticatedNewWindowHistoryV1),
    Successor {
        history: M1AuthenticatedNewWindowHistoryV1,
        total_windows: u64,
    },
}

/// Maximum total windows in the authenticated finite-history model.
pub const M1_AUTHENTICATED_NEW_WINDOW_TOTAL_LIMIT_V1: u64 = 20;

/// Explicit finite state for each admission or post-commit condition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalNewWindowConditionV1 {
    Rejected,
    Satisfied,
}

/// Read-only checks represented by the bounded model.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1PhysicalNewWindowPrecommitV1 {
    pub source_shape: M1PhysicalNewWindowSourceShapeV1,
    pub exact_prior_next_plan: M1PhysicalNewWindowConditionV1,
    pub exact_next_epoch: M1PhysicalNewWindowConditionV1,
    pub rearmed_predecessor: M1PhysicalNewWindowConditionV1,
    pub no_parked_members: M1PhysicalNewWindowConditionV1,
    pub history_has_capacity: M1PhysicalNewWindowConditionV1,
    pub predecessor_nonempty_all_terminal: M1PhysicalNewWindowConditionV1,
    /// Each successor has one exact predecessor across current and historical terminal custody.
    pub current_historical_predecessor_set_exact: M1PhysicalNewWindowConditionV1,
    pub ready_roster_exact: M1PhysicalNewWindowConditionV1,
    pub page_geometry_exact: M1PhysicalNewWindowConditionV1,
    pub provider_front_matches: M1PhysicalNewWindowConditionV1,
    pub physical_inputs_exact: M1PhysicalNewWindowConditionV1,
    pub catalog_available: M1PhysicalNewWindowConditionV1,
    pub catalog_matches: M1PhysicalNewWindowConditionV1,
}

/// Fallible operations after provider-dequeue commit entry.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1PhysicalNewWindowPostcommitV1 {
    pub provider_dequeue_succeeds: M1PhysicalNewWindowConditionV1,
    pub physical_submission_succeeds: M1PhysicalNewWindowConditionV1,
    pub published_shape_is_paired_prefill: M1PhysicalNewWindowConditionV1,
    pub rollover_observation_present: M1PhysicalNewWindowConditionV1,
}

/// First terminal phase reached after read-only admission succeeds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalNewWindowTerminalPhaseV1 {
    ProviderCommit,
    PhysicalSubmission,
    PublishedShape,
    RolloverObservation,
}

/// Exact bounded outcome classes at this proof boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1PhysicalNewWindowOutcomeV1 {
    Retryable {
        custody: M1PhysicalNewWindowCustodyV1,
        engine_quarantined: bool,
        provider_input_dequeued: bool,
    },
    Terminal {
        phase: M1PhysicalNewWindowTerminalPhaseV1,
        custody: M1PhysicalNewWindowCustodyV1,
        engine_quarantined: bool,
        provider_input_dequeued: bool,
    },
    PublishedPairedPrefill {
        custody: M1PhysicalNewWindowCustodyV1,
        engine_quarantined: bool,
        provider_input_dequeued: bool,
        rollover_observation_present: bool,
    },
}

pub open spec fn m1_physical_new_window_source_shape_supported_v1(
    shape: M1PhysicalNewWindowSourceShapeV1,
) -> bool {
    shape == M1PhysicalNewWindowSourceShapeV1::SpeculativeK4
        || shape == M1PhysicalNewWindowSourceShapeV1::SpeculativeK8
        || shape == M1PhysicalNewWindowSourceShapeV1::SpeculativeK16
}

pub open spec fn m1_physical_new_window_condition_satisfied_v1(
    condition: M1PhysicalNewWindowConditionV1,
) -> bool {
    match condition {
        M1PhysicalNewWindowConditionV1::Rejected => false,
        M1PhysicalNewWindowConditionV1::Satisfied => true,
    }
}

/// Exact pre-state for archiving one authenticated completed current window.
pub open spec fn m1_authenticated_new_window_history_exact_v1(
    history: M1AuthenticatedNewWindowHistoryV1,
) -> bool {
    history.prior_windows == history.inert_archive_records
        && history.current_terminal_lineage_retained
}

/// Production-aligned total-window admission boundary.
///
/// Eighteen prior records admit a successor with 19 prior records and one
/// active window. Nineteen prior records reject the would-be 21st total window.
pub open spec fn m1_authenticated_new_window_history_can_advance_v1(
    history: M1AuthenticatedNewWindowHistoryV1,
) -> bool {
    m1_authenticated_new_window_history_exact_v1(history)
        && history.prior_windows < M1_AUTHENTICATED_NEW_WINDOW_TOTAL_LIMIT_V1 - 1
}

/// Checks the flat-history invariant and exact total-window capacity.
#[must_use]
pub fn check_m1_authenticated_new_window_history_transition_v1(
    history: M1AuthenticatedNewWindowHistoryV1,
) -> (admitted: bool)
    ensures admitted == m1_authenticated_new_window_history_can_advance_v1(history),
{
    if history.prior_windows != history.inert_archive_records
        || !history.current_terminal_lineage_retained
    {
        return false;
    }
    let Some(archived_windows) = history.prior_windows.checked_add(1) else {
        return false;
    };
    archived_windows < M1_AUTHENTICATED_NEW_WINDOW_TOTAL_LIMIT_V1
}

/// Mathematical authenticated history transition.
pub open spec fn m1_authenticated_new_window_history_transition_spec_v1(
    history: M1AuthenticatedNewWindowHistoryV1,
    action: M1AuthenticatedNewWindowHistoryActionV1,
) -> M1AuthenticatedNewWindowHistoryOutcomeV1 {
    if !m1_authenticated_new_window_history_can_advance_v1(history) {
        M1AuthenticatedNewWindowHistoryOutcomeV1::Rejected(history)
    } else {
        match action {
            M1AuthenticatedNewWindowHistoryActionV1::Retry => {
                M1AuthenticatedNewWindowHistoryOutcomeV1::Retained(history)
            },
            M1AuthenticatedNewWindowHistoryActionV1::Advance => {
                M1AuthenticatedNewWindowHistoryOutcomeV1::Successor {
                    history: M1AuthenticatedNewWindowHistoryV1 {
                        prior_windows: (history.prior_windows + 1) as u64,
                        inert_archive_records: (history.inert_archive_records + 1) as u64,
                        active_rounds: 0,
                        current_terminal_lineage_retained: false,
                    },
                    total_windows: (history.prior_windows + 2) as u64,
                }
            },
        }
    }
}

pub open spec fn m1_authenticated_new_window_retry_is_exact_v1(
    outcome: M1AuthenticatedNewWindowHistoryOutcomeV1,
    source: M1AuthenticatedNewWindowHistoryV1,
) -> bool {
    match outcome {
        M1AuthenticatedNewWindowHistoryOutcomeV1::Rejected(retained)
        | M1AuthenticatedNewWindowHistoryOutcomeV1::Retained(retained) => {
            retained == source
        },
        M1AuthenticatedNewWindowHistoryOutcomeV1::Successor { .. } => true,
    }
}

pub open spec fn m1_authenticated_new_window_successor_history_exact_v1(
    outcome: M1AuthenticatedNewWindowHistoryOutcomeV1,
    source: M1AuthenticatedNewWindowHistoryV1,
) -> bool {
    match outcome {
        M1AuthenticatedNewWindowHistoryOutcomeV1::Successor {
            history,
            total_windows,
        } => {
            history.prior_windows == source.prior_windows + 1
                && history.inert_archive_records == source.inert_archive_records + 1
                && history.active_rounds == 0
                && !history.current_terminal_lineage_retained
                && total_windows == history.prior_windows + 1
                && total_windows <= M1_AUTHENTICATED_NEW_WINDOW_TOTAL_LIMIT_V1
        },
        _ => true,
    }
}

/// Executes and proves the pure authenticated history/cap transition.
///
/// The caller chooses `Retry` or `Advance`; this theorem therefore establishes
/// finite safety and exact accounting, not liveness. It proves no `Instant`,
/// allocation, KFD, queue, readback, device-memory, or engine-fault behavior.
/// It also proves neither terminal-member cardinality nor payload preservation,
/// and neither production pre-detach nor successor-join retry-owner preservation.
#[must_use]
pub fn advance_m1_authenticated_new_window_history_v1(
    history: M1AuthenticatedNewWindowHistoryV1,
    action: M1AuthenticatedNewWindowHistoryActionV1,
) -> (outcome: M1AuthenticatedNewWindowHistoryOutcomeV1)
    ensures
        outcome == m1_authenticated_new_window_history_transition_spec_v1(history, action),
        m1_authenticated_new_window_retry_is_exact_v1(outcome, history),
        m1_authenticated_new_window_successor_history_exact_v1(outcome, history),
{
    if !check_m1_authenticated_new_window_history_transition_v1(history) {
        return M1AuthenticatedNewWindowHistoryOutcomeV1::Rejected(history);
    }
    match action {
        M1AuthenticatedNewWindowHistoryActionV1::Retry => {
            M1AuthenticatedNewWindowHistoryOutcomeV1::Retained(history)
        },
        M1AuthenticatedNewWindowHistoryActionV1::Advance => {
            let archived_windows = match history.prior_windows.checked_add(1) {
                Some(value) => value,
                None => return M1AuthenticatedNewWindowHistoryOutcomeV1::Rejected(history),
            };
            let total_windows = match archived_windows.checked_add(1) {
                Some(value) => value,
                None => return M1AuthenticatedNewWindowHistoryOutcomeV1::Rejected(history),
            };
            M1AuthenticatedNewWindowHistoryOutcomeV1::Successor {
                history: M1AuthenticatedNewWindowHistoryV1 {
                    prior_windows: archived_windows,
                    inert_archive_records: archived_windows,
                    active_rounds: 0,
                    current_terminal_lineage_retained: false,
                },
                total_windows,
            }
        },
    }
}

/// Converts one finite condition to the Boolean used by the transition.
#[must_use]
pub fn check_m1_physical_new_window_condition_v1(
    condition: M1PhysicalNewWindowConditionV1,
) -> (satisfied: bool)
    ensures
        satisfied == m1_physical_new_window_condition_satisfied_v1(condition),
{
    match condition {
        M1PhysicalNewWindowConditionV1::Rejected => false,
        M1PhysicalNewWindowConditionV1::Satisfied => true,
    }
}

/// Mathematical read-only admission predicate.
pub open spec fn m1_physical_new_window_precommit_admitted_v1(
    checks: M1PhysicalNewWindowPrecommitV1,
) -> bool {
    m1_physical_new_window_source_shape_supported_v1(checks.source_shape)
        && m1_physical_new_window_condition_satisfied_v1(checks.exact_prior_next_plan)
        && m1_physical_new_window_condition_satisfied_v1(checks.exact_next_epoch)
        && m1_physical_new_window_condition_satisfied_v1(checks.rearmed_predecessor)
        && m1_physical_new_window_condition_satisfied_v1(checks.no_parked_members)
        && m1_physical_new_window_condition_satisfied_v1(checks.history_has_capacity)
        && m1_physical_new_window_condition_satisfied_v1(
            checks.predecessor_nonempty_all_terminal,
        )
        && m1_physical_new_window_condition_satisfied_v1(
            checks.current_historical_predecessor_set_exact,
        )
        && m1_physical_new_window_condition_satisfied_v1(checks.ready_roster_exact)
        && m1_physical_new_window_condition_satisfied_v1(checks.page_geometry_exact)
        && m1_physical_new_window_condition_satisfied_v1(checks.provider_front_matches)
        && m1_physical_new_window_condition_satisfied_v1(checks.physical_inputs_exact)
        && m1_physical_new_window_condition_satisfied_v1(checks.catalog_available)
        && m1_physical_new_window_condition_satisfied_v1(checks.catalog_matches)
}

/// Executable counterpart of the mathematical read-only admission predicate.
#[must_use]
pub fn check_m1_physical_new_window_precommit_v1(
    checks: M1PhysicalNewWindowPrecommitV1,
) -> (admitted: bool)
    ensures admitted == m1_physical_new_window_precommit_admitted_v1(checks),
{
    matches!(
        checks.source_shape,
        M1PhysicalNewWindowSourceShapeV1::SpeculativeK4
            | M1PhysicalNewWindowSourceShapeV1::SpeculativeK8
            | M1PhysicalNewWindowSourceShapeV1::SpeculativeK16
    ) && check_m1_physical_new_window_condition_v1(checks.exact_prior_next_plan)
        && check_m1_physical_new_window_condition_v1(checks.exact_next_epoch)
        && check_m1_physical_new_window_condition_v1(checks.rearmed_predecessor)
        && check_m1_physical_new_window_condition_v1(checks.no_parked_members)
        && check_m1_physical_new_window_condition_v1(checks.history_has_capacity)
        && check_m1_physical_new_window_condition_v1(
            checks.predecessor_nonempty_all_terminal,
        )
        && check_m1_physical_new_window_condition_v1(
            checks.current_historical_predecessor_set_exact,
        )
        && check_m1_physical_new_window_condition_v1(checks.ready_roster_exact)
        && check_m1_physical_new_window_condition_v1(checks.page_geometry_exact)
        && check_m1_physical_new_window_condition_v1(checks.provider_front_matches)
        && check_m1_physical_new_window_condition_v1(checks.physical_inputs_exact)
        && check_m1_physical_new_window_condition_v1(checks.catalog_available)
        && check_m1_physical_new_window_condition_v1(checks.catalog_matches)
}

/// Mathematical transaction relation used by the executable refinement body.
pub open spec fn m1_physical_new_window_transaction_spec_v1(
    custody: M1PhysicalNewWindowCustodyV1,
    checks: M1PhysicalNewWindowPrecommitV1,
    postcommit: M1PhysicalNewWindowPostcommitV1,
) -> M1PhysicalNewWindowOutcomeV1 {
    if !m1_physical_new_window_precommit_admitted_v1(checks) {
        M1PhysicalNewWindowOutcomeV1::Retryable {
            custody,
            engine_quarantined: false,
            provider_input_dequeued: false,
        }
    } else if !m1_physical_new_window_condition_satisfied_v1(
        postcommit.provider_dequeue_succeeds,
    ) {
        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::ProviderCommit,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: false,
        }
    } else if !m1_physical_new_window_condition_satisfied_v1(
        postcommit.physical_submission_succeeds,
    ) {
        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::PhysicalSubmission,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: true,
        }
    } else if !m1_physical_new_window_condition_satisfied_v1(
        postcommit.published_shape_is_paired_prefill,
    ) {
        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::PublishedShape,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: true,
        }
    } else if !m1_physical_new_window_condition_satisfied_v1(
        postcommit.rollover_observation_present,
    ) {
        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::RolloverObservation,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: true,
        }
    } else {
        M1PhysicalNewWindowOutcomeV1::PublishedPairedPrefill {
            custody,
            engine_quarantined: false,
            provider_input_dequeued: true,
            rollover_observation_present: true,
        }
    }
}

pub open spec fn m1_physical_new_window_retryable_v1(
    outcome: M1PhysicalNewWindowOutcomeV1,
) -> bool {
    matches!(outcome, M1PhysicalNewWindowOutcomeV1::Retryable { .. })
}

pub open spec fn m1_physical_new_window_terminal_v1(
    outcome: M1PhysicalNewWindowOutcomeV1,
) -> bool {
    matches!(outcome, M1PhysicalNewWindowOutcomeV1::Terminal { .. })
}

pub open spec fn m1_physical_new_window_published_v1(
    outcome: M1PhysicalNewWindowOutcomeV1,
) -> bool {
    matches!(outcome, M1PhysicalNewWindowOutcomeV1::PublishedPairedPrefill { .. })
}

pub open spec fn m1_physical_new_window_exact_custody_v1(
    outcome: M1PhysicalNewWindowOutcomeV1,
    expected: M1PhysicalNewWindowCustodyV1,
) -> bool {
    match outcome {
        M1PhysicalNewWindowOutcomeV1::Retryable { custody, .. }
        | M1PhysicalNewWindowOutcomeV1::Terminal { custody, .. }
        | M1PhysicalNewWindowOutcomeV1::PublishedPairedPrefill { custody, .. } => {
            custody == expected
        },
    }
}

pub open spec fn m1_physical_new_window_terminal_is_quarantined_v1(
    outcome: M1PhysicalNewWindowOutcomeV1,
) -> bool {
    match outcome {
        M1PhysicalNewWindowOutcomeV1::Terminal { engine_quarantined, .. } => {
            engine_quarantined
        },
        _ => true,
    }
}

/// Executes and proves the finite physical-new-window transition.
///
/// The selected body establishes the exact read-only retry boundary, opaque
/// two-owner custody preservation, post-commit quarantine, and the unique
/// paired-prefill success condition for K4, K8, and K16 source shapes.
#[must_use]
pub fn execute_m1_physical_new_window_transaction_v1(
    custody: M1PhysicalNewWindowCustodyV1,
    checks: M1PhysicalNewWindowPrecommitV1,
    postcommit: M1PhysicalNewWindowPostcommitV1,
) -> (outcome: M1PhysicalNewWindowOutcomeV1)
    ensures
        outcome == m1_physical_new_window_transaction_spec_v1(custody, checks, postcommit),
        m1_physical_new_window_exact_custody_v1(outcome, custody),
        m1_physical_new_window_retryable_v1(outcome)
            == !m1_physical_new_window_precommit_admitted_v1(checks),
        m1_physical_new_window_terminal_v1(outcome) ==> (
            m1_physical_new_window_precommit_admitted_v1(checks)
                && m1_physical_new_window_terminal_is_quarantined_v1(outcome)
        ),
        m1_physical_new_window_terminal_v1(outcome) == (
            m1_physical_new_window_precommit_admitted_v1(checks)
                && !(
                    m1_physical_new_window_condition_satisfied_v1(
                        postcommit.provider_dequeue_succeeds,
                    )
                        && m1_physical_new_window_condition_satisfied_v1(
                            postcommit.physical_submission_succeeds,
                        )
                        && m1_physical_new_window_condition_satisfied_v1(
                            postcommit.published_shape_is_paired_prefill,
                        )
                        && m1_physical_new_window_condition_satisfied_v1(
                            postcommit.rollover_observation_present,
                        )
                )
        ),
        m1_physical_new_window_published_v1(outcome) == (
            m1_physical_new_window_precommit_admitted_v1(checks)
                && m1_physical_new_window_condition_satisfied_v1(
                    postcommit.provider_dequeue_succeeds,
                )
                && m1_physical_new_window_condition_satisfied_v1(
                    postcommit.physical_submission_succeeds,
                )
                && m1_physical_new_window_condition_satisfied_v1(
                    postcommit.published_shape_is_paired_prefill,
                )
                && m1_physical_new_window_condition_satisfied_v1(
                    postcommit.rollover_observation_present,
                )
        ),
{
    let admitted = check_m1_physical_new_window_precommit_v1(checks);
    let provider_dequeue_succeeds = check_m1_physical_new_window_condition_v1(
        postcommit.provider_dequeue_succeeds,
    );
    let physical_submission_succeeds = check_m1_physical_new_window_condition_v1(
        postcommit.physical_submission_succeeds,
    );
    let published_shape_is_paired_prefill = check_m1_physical_new_window_condition_v1(
        postcommit.published_shape_is_paired_prefill,
    );
    let rollover_observation_present = check_m1_physical_new_window_condition_v1(
        postcommit.rollover_observation_present,
    );
    if !admitted {
        M1PhysicalNewWindowOutcomeV1::Retryable {
            custody,
            engine_quarantined: false,
            provider_input_dequeued: false,
        }
    } else if !provider_dequeue_succeeds {
        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::ProviderCommit,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: false,
        }
    } else if !physical_submission_succeeds {
        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::PhysicalSubmission,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: true,
        }
    } else if !published_shape_is_paired_prefill {
        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::PublishedShape,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: true,
        }
    } else if !rollover_observation_present {
        M1PhysicalNewWindowOutcomeV1::Terminal {
            phase: M1PhysicalNewWindowTerminalPhaseV1::RolloverObservation,
            custody,
            engine_quarantined: true,
            provider_input_dequeued: true,
        }
    } else {
        M1PhysicalNewWindowOutcomeV1::PublishedPairedPrefill {
            custody,
            engine_quarantined: false,
            provider_input_dequeued: true,
            rollover_observation_present: true,
        }
    }
}

} // verus!

#[cfg(test)]
mod source_policy_tests {
    use super::*;

    const MODEL_SOURCE: &str = include_str!("physical_new_window.rs");
    const OPERATIONS_SOURCE: &str =
        include_str!("../../crates/ferric-engine/src/m1_serving_physical_operations.rs");
    const QUEUE_REARM_SOURCE: &str =
        include_str!("../../crates/ferric-engine/src/m1_queue_rearm.rs");
    const INPUT_PROVIDER_SOURCE: &str =
        include_str!("../../crates/ferric-engine/src/m1_serving_physical_input_provider.rs");
    const REGISTRY_SOURCE: &str =
        include_str!("../../crates/ferric-engine/src/m1_serving_registry.rs");
    const DEVICE_CACHE_SOURCE: &str =
        include_str!("../../crates/ferric-engine/src/device_cache.rs");
    const PAGED_KV_SOURCE: &str =
        include_str!("../../crates/ferric-spec/src/paged_kv_refinement.rs");
    const AUTHENTICATED_ROLLOVER_SOURCE: &str =
        include_str!("../../crates/ferric-engine/src/authenticated_queue_rollover.rs");
    const AUTHENTICATED_EXECUTOR_SOURCE: &str =
        include_str!("../../crates/ferric-engine/src/authenticated_speculative_executor.rs");

    fn unique_offset(source: &str, needle: &str) -> usize {
        let mut matches = source.match_indices(needle);
        let Some((offset, _)) = matches.next() else {
            panic!("physical-new-window source-policy anchor is absent: {needle}");
        };
        assert!(
            matches.next().is_none(),
            "physical-new-window source-policy anchor is not unique: {needle}"
        );
        offset
    }

    fn has_ordered_history_take_and_reset(source: &str) -> bool {
        let unique = |needle: &str| {
            let mut matches = source.match_indices(needle);
            let offset = matches.next().map(|(offset, _)| offset)?;
            matches.next().is_none().then_some(offset)
        };
        let Some(attach) = unique(".attach_physical_history(history)") else {
            return false;
        };
        let Some(select) = unique("let history = resident_phase_storage.as_mut().map_or(") else {
            return false;
        };
        let Some(take) = unique("&mut storage.successor_round_history,") else {
            return false;
        };
        let Some(reset) =
            unique("crate::m1_queue_rearm::M1RearmRoundHistoryV1::Empty,\n            )")
        else {
            return false;
        };
        let Some(continue_with_history) = unique(
            "M1AuthenticatedSpeculativePriorWindowContinuationV1 {\n            terminal,\n            history,",
        ) else {
            return false;
        };
        attach < select && select < take && take < reset && reset < continue_with_history
    }

    #[test]
    fn model_outcome_constructors_are_unique() {
        for constructor in [
            "M1PhysicalNewWindowOutcomeV1::Retryable {\n            custody,\n            engine_quarantined: false,",
            "phase: M1PhysicalNewWindowTerminalPhaseV1::ProviderCommit,\n            custody,\n            engine_quarantined: true,",
            "phase: M1PhysicalNewWindowTerminalPhaseV1::PhysicalSubmission,\n            custody,\n            engine_quarantined: true,",
            "phase: M1PhysicalNewWindowTerminalPhaseV1::PublishedShape,\n            custody,\n            engine_quarantined: true,",
            "phase: M1PhysicalNewWindowTerminalPhaseV1::RolloverObservation,\n            custody,\n            engine_quarantined: true,",
            "M1PhysicalNewWindowOutcomeV1::PublishedPairedPrefill {\n            custody,\n            engine_quarantined: false,",
        ] {
            // Each constructor occurs once in the spec and once in the executable body.
            assert_eq!(MODEL_SOURCE.matches(constructor).count(), 2, "{constructor}");
        }
    }

    #[test]
    fn production_source_pins_read_only_checks_before_provider_commit() {
        let start = OPERATIONS_SOURCE
            .find("    fn quiescent_new_window(")
            .expect("production new-window operation is absent");
        let end = OPERATIONS_SOURCE[start..]
            .find("    fn read_published(")
            .map(|offset| start + offset)
            .expect("production new-window operation end is absent");
        let source = &OPERATIONS_SOURCE[start..end];

        let provider_preflight =
            unique_offset(source, "preflight_paired_prefill_new_window(batch)");
        let physical_preflight = unique_offset(
            source,
            "preflight_m1_all_terminal_paired_prefill_new_window_v1(",
        );
        let catalog = unique_offset(source, "self.runner.content_bound_program_catalog_v1()");
        let catalog_join = unique_offset(
            source,
            "catalog.catalog_id() != released.current_released().queue().custody().catalog_id()",
        );
        let commit = unique_offset(
            source,
            "// Commit: the provider input and released queue cross together.",
        );
        let dequeue = unique_offset(source, ".take_paired_prefill_new_window(batch)");
        let submit = unique_offset(
            source,
            "submit_m1_all_terminal_paired_prefill_new_window_v1(",
        );
        let terminal_publication = unique_offset(
            source,
            "M1ServingPhysicalRunnerTerminalLowerCustodyV1::NewWindowPublication",
        );
        let paired_shape = unique_offset(
            source,
            "published.shape() != M1PhysicalFixedBatchShapeV1::PairedPrefill",
        );
        let rollover = unique_offset(source, "published.rollover_observation().is_none()");
        let activate = unique_offset(source, "self.active_plan = Some(next);");

        assert!(provider_preflight < physical_preflight);
        assert!(physical_preflight < catalog);
        assert!(catalog < catalog_join);
        assert!(catalog_join < commit);
        assert!(commit < dequeue);
        assert!(dequeue < submit);
        assert!(submit < terminal_publication);
        assert!(terminal_publication < paired_shape);
        assert!(paired_shape < rollover);
        assert!(rollover < activate);
        assert_eq!(
            source[commit..]
                .matches("M1ServingPhysicalOperationFailureV1::Retryable")
                .count(),
            0,
        );
    }

    #[test]
    fn production_submit_wrapper_quarantines_every_post_commit_failure() {
        let submit = unique_offset(
            QUEUE_REARM_SOURCE,
            "pub(crate) fn submit_m1_all_terminal_paired_prefill_new_window_v1",
        );
        let quarantine = unique_offset(
            QUEUE_REARM_SOURCE,
            "fn quarantine_failed_new_window_submission<'a, T, const C: usize>",
        );
        let next_impl = QUEUE_REARM_SOURCE[quarantine..]
            .find("impl M1RearmedPublishedQueueV1 {")
            .map(|offset| quarantine + offset)
            .expect("post-quarantine published implementation is absent");
        let submit_source = &QUEUE_REARM_SOURCE[submit..quarantine];
        let quarantine_source = &QUEUE_REARM_SOURCE[quarantine..next_impl];

        let inner = unique_offset(
            submit_source,
            "submit_m1_all_terminal_paired_prefill_new_window_inner_v1(",
        );
        let closure = unique_offset(
            submit_source,
            "quarantine_failed_new_window_submission(engine, result)",
        );
        let error_check = unique_offset(quarantine_source, "if result.is_err() {");
        let engine_quarantine = unique_offset(
            quarantine_source,
            "engine.quarantine_m1_queue_rearm_failure();",
        );
        assert!(inner < closure);
        assert!(error_check < engine_quarantine);
    }

    #[test]
    fn production_preflight_pins_current_historical_predecessor_set() {
        let request_start = QUEUE_REARM_SOURCE
            .find("fn m1_new_window_request_is_exact_successor_v1(")
            .expect("new-window request successor helper is absent");
        let set_start = QUEUE_REARM_SOURCE[request_start..]
            .find("fn m1_new_window_terminal_predecessor_set_matches_v1<I, J>(")
            .map(|offset| request_start + offset)
            .expect("new-window predecessor-set helper is absent");
        let preflight_start = QUEUE_REARM_SOURCE[set_start..]
            .find("pub(crate) fn preflight_m1_all_terminal_paired_prefill_new_window_v1")
            .map(|offset| set_start + offset)
            .expect("new-window preflight is absent");
        let preflight_end = QUEUE_REARM_SOURCE[preflight_start..]
            .find("struct M1NewWindowRetainedCustodyV1")
            .map(|offset| preflight_start + offset)
            .expect("new-window preflight end is absent");
        let request_source = &QUEUE_REARM_SOURCE[request_start..set_start];
        let set_source = &QUEUE_REARM_SOURCE[set_start..preflight_start];
        let preflight_source = &QUEUE_REARM_SOURCE[preflight_start..preflight_end];

        let same_slot = unique_offset(request_source, "predecessor.slot() == replacement.slot()");
        let exact_generation = unique_offset(
            request_source,
            "predecessor.generation().checked_add(1) == Some(replacement.generation())",
        );
        let nonempty = unique_offset(set_source, "if replacements.is_empty()");
        let every_current = unique_offset(set_source, "!current.clone().all(|predecessor|");
        let current_successor = unique_offset(
            set_source,
            "m1_new_window_request_is_exact_successor_v1(predecessor, replacement)",
        );
        let exact_prior_generation =
            unique_offset(set_source, "replacement.generation().checked_sub(1)");
        let distinct_slots = unique_offset(
            set_source,
            ".any(|prior| prior.slot() == replacement.slot())",
        );
        let current_count = unique_offset(set_source, "let current_matches = current");
        let historical_count = unique_offset(set_source, "let lineage_matches = lineage");
        let unique_union = unique_offset(set_source, "current_matches + lineage_matches == 1");
        let admitted_set = unique_offset(
            preflight_source,
            "m1_new_window_terminal_predecessor_set_matches_v1(",
        );
        let current_members = unique_offset(
            preflight_source,
            ".map(M1ReleasedDeviceKvMemberV1::request)",
        );
        let historical_terminal = unique_offset(
            preflight_source,
            "released.terminal.iter().map(|member| member.request())",
        );
        let replacement_set = unique_offset(
            preflight_source,
            "\n            batch.requests(),\n        )\n        && engine.live_count()",
        );

        assert!(same_slot < exact_generation);
        assert!(nonempty < every_current);
        assert!(every_current < current_successor);
        assert!(current_successor < exact_prior_generation);
        assert!(exact_prior_generation < distinct_slots);
        assert!(distinct_slots < current_count);
        assert!(current_count < historical_count);
        assert!(historical_count < unique_union);
        assert!(!set_source.contains(".zip(replacements)"));
        assert!(admitted_set < current_members);
        assert!(current_members < historical_terminal);
        assert!(historical_terminal < replacement_set);
    }

    #[test]
    fn registry_revalidates_every_completed_predecessor_and_exact_successor() {
        let reserve_start = REGISTRY_SOURCE
            .find("    pub fn reserve_completed_window_replacement(")
            .expect("completed-window reservation is absent");
        let reserve_end = REGISTRY_SOURCE[reserve_start..]
            .find("    pub fn restore_completed_window_replacement(")
            .map(|offset| reserve_start + offset)
            .expect("completed-window reservation end is absent");
        let validate_start = REGISTRY_SOURCE
            .find("fn validate_completed_window_predecessor(")
            .expect("completed-window predecessor validator is absent");
        let validate_end = REGISTRY_SOURCE[validate_start..]
            .find("fn validate_plan_transition(")
            .map(|offset| validate_start + offset)
            .expect("completed-window predecessor validator end is absent");
        let reserve = &REGISTRY_SOURCE[reserve_start..reserve_end];
        let validate = &REGISTRY_SOURCE[validate_start..validate_end];

        let every_entry = unique_offset(
            reserve,
            "for (index, entry) in self.entries.iter().enumerate()",
        );
        let completed_validation = unique_offset(
            reserve,
            "validate_completed_window_predecessor(entry, prior, self.completed_epoch, index)",
        );
        let exact_slot = unique_offset(reserve, "request.slot() != predecessor.request.slot()");
        let exact_generation =
            unique_offset(reserve, "predecessor.request.generation().checked_add(1)");
        let complete_frontier =
            unique_offset(reserve, "self.submitted_epoch != self.completed_epoch");
        assert!(every_entry < completed_validation);
        assert!(completed_validation < exact_slot);
        assert!(exact_slot < exact_generation);
        assert!(exact_generation < complete_frontier);

        let exact_plan = unique_offset(validate, "if entry.plan != prior");
        let completed_phase = unique_offset(
            validate,
            "M1ServingRequestPhaseV1::Retired {\n        quiescence: M1ServingQuiescenceV1::Completed(epoch),",
        );
        let nonzero_bounded_epoch = unique_offset(
            validate,
            "epoch.value() == 0 || epoch.value() > completed_epoch",
        );
        let exact_quiescence = unique_offset(validate, "entry.last_quiescence != Some(epoch)");
        assert!(exact_plan < completed_phase);
        assert!(completed_phase < nonzero_bounded_epoch);
        assert!(nonzero_bounded_epoch < exact_quiescence);
        assert!(!validate.contains("epoch.value() != completed_epoch"));
    }

    #[test]
    fn new_window_inputs_bind_batch_rows_and_live_runner_plan_identities() {
        let physical_start = INPUT_PROVIDER_SOURCE
            .find("    pub(crate) fn physical_inputs_match(")
            .expect("new-window physical-input matcher is absent");
        let runner_start = INPUT_PROVIDER_SOURCE[physical_start..]
            .find("    pub(crate) fn logical_runner_plan_identities_match(")
            .map(|offset| physical_start + offset)
            .expect("new-window runner-plan matcher is absent");
        let runner_end = INPUT_PROVIDER_SOURCE[runner_start..]
            .find("    pub(crate) fn into_parts(")
            .map(|offset| runner_start + offset)
            .expect("new-window runner-plan matcher end is absent");
        let physical = &INPUT_PROVIDER_SOURCE[physical_start..runner_start];
        let runner = &INPUT_PROVIDER_SOURCE[runner_start..runner_end];

        let binding = unique_offset(
            physical,
            ".matches(batch.plan(), batch.requests(), batch.epoch())",
        );
        let paired_shape = unique_offset(
            physical,
            "batch.plan().shape() == M1PhysicalFixedBatchShapeV1::PairedPrefill",
        );
        let draft_role = unique_offset(physical, "batch.plan().draft(),");
        let target_role = unique_offset(physical, "batch.plan().target(),");
        let rows = unique_offset(
            physical,
            "paired_prefill_new_window_rows_match(&self.draft_prefill, &self.target_prefill)",
        );
        assert!(binding < paired_shape);
        assert!(paired_shape < draft_role);
        assert!(draft_role < target_role);
        assert!(target_role < rows);

        let published = unique_offset(
            runner,
            "let Ok(published) = runner.plan(inputs.selection())",
        );
        let live = unique_offset(runner, "usize::try_from(inputs.live_lane_count())");
        let live_prefix = unique_offset(runner, ".take(live)");
        let plan_identity = unique_offset(
            runner,
            "plan.is_some_and(|plan| plan.plan_id() == &published.plan_id)",
        );
        assert!(published < live);
        assert!(live < live_prefix);
        assert!(live_prefix < plan_identity);

        let preflight_start = QUEUE_REARM_SOURCE
            .find("pub(crate) fn preflight_m1_all_terminal_paired_prefill_new_window_v1")
            .expect("new-window preflight is absent");
        let preflight_end = QUEUE_REARM_SOURCE[preflight_start..]
            .find("struct M1NewWindowRetainedCustodyV1")
            .map(|offset| preflight_start + offset)
            .expect("new-window preflight end is absent");
        let preflight = &QUEUE_REARM_SOURCE[preflight_start..preflight_end];
        let input_check = unique_offset(preflight, "input.physical_inputs_match(batch)");
        let runner_check = unique_offset(
            preflight,
            "input.logical_runner_plan_identities_match(runner)",
        );
        assert!(input_check < runner_check);
    }

    #[test]
    fn new_window_page_generations_flow_from_full_ledgers_into_both_role_states() {
        let snapshot_start = DEVICE_CACHE_SOURCE
            .find("fn new_window_page_generation_snapshot_from_ledger(")
            .expect("new-window ledger snapshot helper is absent");
        let snapshot_end = DEVICE_CACHE_SOURCE[snapshot_start..]
            .find("#[cfg(test)]\nimpl DeviceKvPageLease")
            .map(|offset| snapshot_start + offset)
            .expect("new-window ledger snapshot helper end is absent");
        let snapshot = &DEVICE_CACHE_SOURCE[snapshot_start..snapshot_end];
        let full_range = unique_offset(
            snapshot,
            "for physical_index in 0..M1_KV_PHYSICAL_PAGE_SLOTS",
        );
        let global_index = unique_offset(
            snapshot,
            "let global_index = global_page_index(request, physical_index)?;",
        );
        let ledger_lookup = unique_offset(snapshot, ".get(global_index)");
        let free_generation = unique_offset(snapshot, "free_page_generation(state)?");
        let snapshot_write = unique_offset(
            snapshot,
            "generations[physical_index as usize] = free_page_generation(state)?;",
        );
        assert!(full_range < global_index);
        assert!(global_index < ledger_lookup);
        assert!(ledger_lookup < snapshot_write);
        assert!(snapshot_write < free_generation);

        let custody_start = DEVICE_CACHE_SOURCE
            .find("    fn new_window_page_generation_snapshot(\n")
            .expect("role-local new-window snapshot is absent");
        let custody_end = DEVICE_CACHE_SOURCE[custody_start..]
            .find("    pub(crate) fn revalidate_page_return_authority(")
            .map(|offset| custody_start + offset)
            .expect("new-window device-cache constructor end is absent");
        let custody = &DEVICE_CACHE_SOURCE[custody_start..custody_end];
        let role_ledger = unique_offset(
            custody,
            "new_window_page_generation_snapshot_from_ledger(self.page_ledger(role), request)",
        );
        let target_snapshot = unique_offset(
            custody,
            ".new_window_page_generation_snapshot(request, Qwen3ModelRole::Target8B)",
        );
        let draft_snapshot = unique_offset(
            custody,
            ".new_window_page_generation_snapshot(request, Qwen3ModelRole::Draft06B)",
        );
        let paired_cache =
            unique_offset(custody, "ActiveDeviceKvCache::new_with_page_generations(");
        assert!(role_ledger < target_snapshot);
        assert!(target_snapshot < draft_snapshot);
        assert!(draft_snapshot < paired_cache);

        let role_cache_start = DEVICE_CACHE_SOURCE
            .find("    fn new_with_page_generations(\n        request: RequestId,")
            .expect("role cache page-generation constructor is absent");
        let role_cache_end = DEVICE_CACHE_SOURCE[role_cache_start..]
            .find("    fn from_physical(")
            .map(|offset| role_cache_start + offset)
            .expect("role cache page-generation constructor end is absent");
        let role_cache = &DEVICE_CACHE_SOURCE[role_cache_start..role_cache_end];
        unique_offset(
            role_cache,
            "PhysicalKvState::new_with_page_generations(request, selection, *page_generations)?",
        );

        let spec_start = PAGED_KV_SOURCE
            .find("    pub fn new_with_page_generations(")
            .expect("verified physical KV page-generation constructor is absent");
        let spec_end = PAGED_KV_SOURCE[spec_start..]
            .find("    #[must_use]\n    pub const fn logical_state(")
            .map(|offset| spec_start + offset)
            .expect("verified physical KV page-generation constructor end is absent");
        let spec = &PAGED_KV_SOURCE[spec_start..spec_end];
        let validate_all = spec
            .find("while position < M1_KV_PHYSICAL_PAGE_SLOTS")
            .expect("physical KV generation validation loop is absent");
        let reject_zero =
            unique_offset(spec, "return Err(PhysicalKvError::PageGenerationMismatch);");
        let seed_slots = spec
            .match_indices("while position < M1_KV_PHYSICAL_PAGE_SLOTS")
            .nth(1)
            .map(|(offset, _)| offset)
            .expect("physical KV slot seeding loop is absent");
        let copied_generation = unique_offset(spec, "generation: page_generations[position],");
        assert!(validate_all < reject_zero);
        assert!(reject_zero < seed_slots);
        assert!(seed_slots < copied_generation);
    }

    #[test]
    fn speculative_retirement_preflights_then_fail_stops_after_mutation() {
        let helper_start = OPERATIONS_SOURCE
            .find("fn retire_preflighted_speculative_engine_members<const C: usize>(")
            .expect("speculative Engine retirement helper is absent");
        let helper_end = OPERATIONS_SOURCE[helper_start..]
            .find("impl<'a, const C: usize, P> M1ServingPhysicalOperationsV1")
            .map(|offset| helper_start + offset)
            .expect("speculative Engine retirement helper end is absent");
        let helper = &OPERATIONS_SOURCE[helper_start..helper_end];
        let preflight = unique_offset(helper, "if permit.members().len() != dispositions.len()");
        let roster = unique_offset(
            helper,
            "engine.pending_member(lane) == Some(member.request())",
        );
        let preflight_reject = unique_offset(
            helper,
            "return Err(M1SpeculativeEngineRetirementErrorV1::Preflight);",
        );
        let first_mutation = unique_offset(helper, "engine.retire(member.request())");
        let commit_reject = unique_offset(
            helper,
            "return Err(M1SpeculativeEngineRetirementErrorV1::Commit);",
        );
        assert!(preflight < roster);
        assert!(roster < preflight_reject);
        assert!(preflight_reject < first_mutation);
        assert!(first_mutation < commit_reject);

        let settle_start = OPERATIONS_SOURCE[helper_end..]
            .find("    fn settle_speculative_readback(")
            .map(|offset| helper_end + offset)
            .expect("speculative settlement implementation is absent");
        let settle_end = OPERATIONS_SOURCE[settle_start..]
            .find("    fn settle_readback(")
            .map(|offset| settle_start + offset)
            .expect("speculative settlement implementation end is absent");
        let settle = &OPERATIONS_SOURCE[settle_start..settle_end];
        let helper_call = unique_offset(
            settle,
            "retire_preflighted_speculative_engine_members(self.engine, permit, &dispositions)",
        );
        let commit_failure =
            unique_offset(settle, "Err(M1SpeculativeEngineRetirementErrorV1::Commit)");
        let quarantine = settle
            .find("self.engine.quarantine_m1_queue_rearm_failure();")
            .expect("speculative retirement quarantine is absent");
        let sealed = settle
            .find("self.phase = M1ServingPhysicalRunnerAdapterPhaseV1::Sealed;")
            .expect("speculative retirement seal is absent");
        let terminal = unique_offset(
            settle,
            "M1ServingPhysicalRunnerOperationErrorV1::SpeculativeRetirementCommit",
        );
        let lower_settle =
            unique_offset(settle, "match self.settle_readback(custody, dispositions)");
        assert!(helper_call < commit_failure);
        assert!(commit_failure < quarantine);
        assert!(quarantine < sealed);
        assert!(sealed < terminal);
        assert!(terminal < lower_settle);
        assert!(settle[lower_settle..].contains("if retired_any"));
        assert!(settle[lower_settle..].contains("self.engine.quarantine_m1_queue_rearm_failure();"));
        assert!(settle[lower_settle..].contains("M1ServingPhysicalOperationFailureV1::Terminal"));
    }

    #[test]
    fn production_reincarnates_appends_then_exactly_dispatches_requested_roster() {
        let submit_start = QUEUE_REARM_SOURCE
            .find("fn submit_m1_all_terminal_paired_prefill_new_window_inner_v1")
            .expect("new-window inner submission is absent");
        let submit_end = QUEUE_REARM_SOURCE[submit_start..]
            .find("pub(crate) fn submit_m1_all_terminal_paired_prefill_new_window_v1")
            .map(|offset| submit_start + offset)
            .expect("new-window inner submission end is absent");
        let submit = &QUEUE_REARM_SOURCE[submit_start..submit_end];
        let loop_start = unique_offset(submit, "for replacement in 0..batch.requests().len()");
        let reincarnate = unique_offset(submit, "engine.reincarnate_next_retiring()");
        let lane_match = unique_offset(submit, ".position(|requested| *requested == successor)");
        let append = unique_offset(submit, "engine.append_tentative(successor, 1)");
        let all_replaced = unique_offset(submit, "replaced_lanes[lane] = true;");
        let exact_dispatch = unique_offset(
            submit,
            "engine.dispatch_m1_exact_ready(batch.epoch(), batch.requests())",
        );
        assert!(loop_start < reincarnate);
        assert!(reincarnate < lane_match);
        assert!(lane_match < append);
        assert!(append < all_replaced);
        assert!(all_replaced < exact_dispatch);
    }

    #[test]
    fn queued_new_window_has_one_public_constructor_and_exact_front_take() {
        assert_eq!(
            INPUT_PROVIDER_SOURCE
                .matches("pub fn paired_prefill_new_window(input:")
                .count(),
            1,
        );
        let lookup = unique_offset(
            INPUT_PROVIDER_SOURCE,
            "pub(crate) fn paired_prefill_new_window_input(",
        );
        let take = unique_offset(
            INPUT_PROVIDER_SOURCE,
            "pub(crate) fn take_paired_prefill_new_window(",
        );
        let after = unique_offset(
            INPUT_PROVIDER_SOURCE,
            "pub fn pending_generation_count(&self) -> usize",
        );
        let take_source = &INPUT_PROVIDER_SOURCE[take..after];
        let recheck = unique_offset(
            take_source,
            "if !self.preflight_paired_prefill_new_window(batch)",
        );
        let pop = unique_offset(take_source, "self.pending.pop_front()");
        assert!(lookup < take);
        assert!(recheck < pop);
    }

    fn authenticated_history(prior_windows: u64) -> M1AuthenticatedNewWindowHistoryV1 {
        M1AuthenticatedNewWindowHistoryV1 {
            prior_windows,
            inert_archive_records: prior_windows,
            active_rounds: 7,
            current_terminal_lineage_retained: true,
        }
    }

    #[test]
    fn authenticated_history_admits_18_and_rejects_19() {
        assert!(check_m1_authenticated_new_window_history_transition_v1(
            authenticated_history(18),
        ));
        assert!(!check_m1_authenticated_new_window_history_transition_v1(
            authenticated_history(19),
        ));
        assert_eq!(M1_AUTHENTICATED_NEW_WINDOW_TOTAL_LIMIT_V1, 20);
    }

    #[test]
    fn authenticated_history_rejects_archive_mismatch_and_missing_lineage() {
        let mut archive_mismatch = authenticated_history(18);
        archive_mismatch.inert_archive_records = 17;
        assert!(!check_m1_authenticated_new_window_history_transition_v1(
            archive_mismatch,
        ));

        let mut missing_current_lineage = authenticated_history(18);
        missing_current_lineage.current_terminal_lineage_retained = false;
        assert!(!check_m1_authenticated_new_window_history_transition_v1(
            missing_current_lineage,
        ));
    }

    #[test]
    fn authenticated_retry_and_successor_accounting_are_exact() {
        let source = authenticated_history(18);
        assert_eq!(
            advance_m1_authenticated_new_window_history_v1(
                source,
                M1AuthenticatedNewWindowHistoryActionV1::Retry,
            ),
            M1AuthenticatedNewWindowHistoryOutcomeV1::Retained(source),
        );

        let M1AuthenticatedNewWindowHistoryOutcomeV1::Successor {
            history,
            total_windows,
        } = advance_m1_authenticated_new_window_history_v1(
            source,
            M1AuthenticatedNewWindowHistoryActionV1::Advance,
        )
        else {
            panic!("an admitted authenticated history must advance");
        };
        assert_eq!(history.prior_windows, 19);
        assert_eq!(history.inert_archive_records, 19);
        assert_ne!(
            history.prior_windows, 20,
            "archive must increment exactly once"
        );
        assert_eq!(history.active_rounds, 0);
        assert!(!history.current_terminal_lineage_retained);
        assert_eq!(total_windows, 20);
    }

    #[test]
    fn authenticated_production_history_cap_and_reset_are_pinned() {
        let cap = unique_offset(
            AUTHENTICATED_ROLLOVER_SOURCE,
            "pub const M1_MAX_AUTHENTICATED_SPECULATIVE_WINDOWS_V1: usize = 20;",
        );
        let admission = unique_offset(
            AUTHENTICATED_ROLLOVER_SOURCE,
            "archived_windows < M1_MAX_AUTHENTICATED_SPECULATIVE_WINDOWS_V1",
        );
        let archive = unique_offset(
            AUTHENTICATED_ROLLOVER_SOURCE,
            "M1AuthenticatedSpeculativeCompletedWindowHistoryV1::archive(",
        );
        let append = unique_offset(
            AUTHENTICATED_ROLLOVER_SOURCE,
            "prior_windows.push(archived);",
        );
        let join_start = AUTHENTICATED_ROLLOVER_SOURCE
            .find("pub(crate) fn schedule_m1_authenticated_speculative_new_window_successor_v1")
            .expect("authenticated successor join is absent");
        let join_end = AUTHENTICATED_ROLLOVER_SOURCE[join_start..]
            .find("\nfn schedule_m1_authenticated_speculative_rollover_pending_v1")
            .map(|offset| join_start + offset)
            .expect("authenticated successor join end is absent");
        let join = &AUTHENTICATED_ROLLOVER_SOURCE[join_start..join_end];
        assert!(cap < admission);
        assert!(admission < archive);
        assert!(archive < append);
        assert!(has_ordered_history_take_and_reset(join));
        assert!(!join.contains("history: crate::m1_queue_rearm::M1RearmRoundHistoryV1::Empty,"));

        let missing_take = join.replacen(
            "&mut storage.successor_round_history,",
            "&mut storage.successor_lineage_seeds,",
            1,
        );
        assert!(!has_ordered_history_take_and_reset(&missing_take));
        let reordered_reset = join.replacen(
            "&mut storage.successor_round_history,\n                crate::m1_queue_rearm::M1RearmRoundHistoryV1::Empty,",
            "crate::m1_queue_rearm::M1RearmRoundHistoryV1::Empty,\n                &mut storage.successor_round_history,",
            1,
        );
        assert_ne!(reordered_reset, join);
        assert!(!has_ordered_history_take_and_reset(&reordered_reset));

        let record = unique_offset(
            AUTHENTICATED_EXECUTOR_SOURCE,
            "pub(crate) struct M1AuthenticatedSpeculativeCompletedWindowHistoryV1",
        );
        let archive_impl = unique_offset(
            AUTHENTICATED_EXECUTOR_SOURCE,
            "    pub(crate) fn archive(\n",
        );
        let initially_unattached =
            unique_offset(AUTHENTICATED_EXECUTOR_SOURCE, "physical_history: None,");
        assert!(record < archive_impl);
        assert!(archive_impl < initially_unattached);
    }

    #[test]
    fn authenticated_history_proof_keeps_runtime_effects_as_nonclaims() {
        for excluded in [
            "not liveness",
            "`Instant`",
            "allocation",
            "KFD",
            "queue",
            "readback",
            "device-memory",
            "engine-fault",
            "terminal-member cardinality",
            "payload preservation",
            "pre-detach",
            "successor-join retry-owner preservation",
        ] {
            assert!(
                MODEL_SOURCE.contains(excluded),
                "missing nonclaim: {excluded}"
            );
        }
        assert!(MODEL_SOURCE.contains("do not prove that the"));
        assert!(MODEL_SOURCE.contains("production implementation refines the model"));
    }
}
