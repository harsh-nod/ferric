#![forbid(unsafe_code)]

//! Pure authenticated S1/T128 prefill-bootstrap lifecycle model.
//!
//! This finite model covers one exact R33 bootstrap boundary: a bound active
//! instance admits exactly 128 prompt tokens, one direct paired-prefill output,
//! and a nonzero successor-output budget whose sum equals the R33 output count.
//! It then advances through an unbound transition sentinel to either queue-input
//! ready prepublication or terminal quarantine. Queue-input ready means only that
//! authenticated prepublication custody exists; no queue has been created or
//! published.
//!
//! Every modeled state remains before physical completion. Consequently the
//! model permits no measurement report or served-token state. Exact stop is
//! accepted only for a matching bound instance; the unbound transition sentinel
//! rejects stop unchanged. These statements do not prove that the production
//! Rust implementation refines this model. They grant no liveness, `Instant` or
//! timing, allocation, KFD, queue creation or publication, physical completion or
//! readback, device-memory, report-authenticity, serving, or M1 authority.

use vstd::prelude::*;

verus! {

/// Exact prompt width admitted by the modeled bootstrap.
pub const M1_AUTHENTICATED_PREFILL_BOOTSTRAP_PROMPT_TOKENS_V1: u64 = 128;

/// Exact direct paired-prefill output count outside the successor budget.
pub const M1_AUTHENTICATED_PREFILL_BOOTSTRAP_DIRECT_OUTPUT_TOKENS_V1: u64 = 1;

/// Pure R33 row accounting presented to the bootstrap boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedPrefillBootstrapInputModelV1 {
    pub prompt_tokens: u64,
    pub direct_output_tokens: u64,
    pub successor_output_tokens: u64,
    pub expected_output_tokens: u64,
}

/// Finite phases before any physical-completion authority exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedPrefillBootstrapPhaseModelV1 {
    ReadyForBootstrap,
    Transitioning,
    QueueReadyPrepublication,
    TerminalQuarantine,
    Stopped,
}

/// Pure precompletion lifecycle state.
///
/// Output fields are declared accounting only. `served_output_tokens` records
/// actual modeled serving authority and therefore remains zero throughout this
/// precompletion model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct M1AuthenticatedPrefillBootstrapStateModelV1 {
    pub phase: M1AuthenticatedPrefillBootstrapPhaseModelV1,
    pub instance: u64,
    pub prompt_tokens: u64,
    pub direct_output_tokens: u64,
    pub successor_output_tokens: u64,
    pub expected_output_tokens: u64,
    pub physical_completion_observed: bool,
    pub measurement_reports: u64,
    pub served_output_tokens: u64,
}

/// Exact-stop result retaining the complete source state on rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum M1AuthenticatedPrefillBootstrapStopOutcomeV1 {
    Accepted(M1AuthenticatedPrefillBootstrapStateModelV1),
    Rejected(M1AuthenticatedPrefillBootstrapStateModelV1),
}

/// Mathematical exact S1/T128 and output-accounting admission predicate.
pub open spec fn m1_authenticated_prefill_bootstrap_input_admitted_spec_v1(
    input: M1AuthenticatedPrefillBootstrapInputModelV1,
) -> bool {
    input.prompt_tokens == M1_AUTHENTICATED_PREFILL_BOOTSTRAP_PROMPT_TOKENS_V1
        && input.direct_output_tokens
            == M1_AUTHENTICATED_PREFILL_BOOTSTRAP_DIRECT_OUTPUT_TOKENS_V1
        && input.successor_output_tokens > 0
        && input.successor_output_tokens < u64::MAX
        && input.expected_output_tokens
            == input.successor_output_tokens + input.direct_output_tokens
}

/// Checks exact prompt and direct-plus-successor accounting without overflow.
#[must_use]
pub fn check_m1_authenticated_prefill_bootstrap_input_v1(
    input: M1AuthenticatedPrefillBootstrapInputModelV1,
) -> (admitted: bool)
    ensures admitted == m1_authenticated_prefill_bootstrap_input_admitted_spec_v1(input),
{
    input.prompt_tokens == M1_AUTHENTICATED_PREFILL_BOOTSTRAP_PROMPT_TOKENS_V1
        && input.direct_output_tokens
            == M1_AUTHENTICATED_PREFILL_BOOTSTRAP_DIRECT_OUTPUT_TOKENS_V1
        && input.successor_output_tokens > 0
        && input
            .successor_output_tokens
            .checked_add(input.direct_output_tokens)
            == Some(input.expected_output_tokens)
}

/// Monotone rank for the finite bootstrap phases.
pub open spec fn m1_authenticated_prefill_bootstrap_phase_rank_spec_v1(
    phase: M1AuthenticatedPrefillBootstrapPhaseModelV1,
) -> u8 {
    match phase {
        M1AuthenticatedPrefillBootstrapPhaseModelV1::ReadyForBootstrap => 0,
        M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning => 1,
        M1AuthenticatedPrefillBootstrapPhaseModelV1::QueueReadyPrepublication
        | M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine => 2,
        M1AuthenticatedPrefillBootstrapPhaseModelV1::Stopped => 3,
    }
}

/// Executable phase rank used by the model's monotonicity contracts.
#[must_use]
pub fn m1_authenticated_prefill_bootstrap_phase_rank_v1(
    phase: M1AuthenticatedPrefillBootstrapPhaseModelV1,
) -> (rank: u8)
    ensures rank == m1_authenticated_prefill_bootstrap_phase_rank_spec_v1(phase),
{
    match phase {
        M1AuthenticatedPrefillBootstrapPhaseModelV1::ReadyForBootstrap => 0,
        M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning => 1,
        M1AuthenticatedPrefillBootstrapPhaseModelV1::QueueReadyPrepublication
        | M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine => 2,
        M1AuthenticatedPrefillBootstrapPhaseModelV1::Stopped => 3,
    }
}

/// No physical completion, report, or served token exists at this boundary.
pub open spec fn m1_authenticated_prefill_bootstrap_is_precompletion_silent_v1(
    state: M1AuthenticatedPrefillBootstrapStateModelV1,
) -> bool {
    !state.physical_completion_observed
        && state.measurement_reports == 0
        && state.served_output_tokens == 0
}

/// Well-formed states reachable inside this finite model.
pub open spec fn m1_authenticated_prefill_bootstrap_state_well_formed_v1(
    state: M1AuthenticatedPrefillBootstrapStateModelV1,
) -> bool {
    m1_authenticated_prefill_bootstrap_is_precompletion_silent_v1(state)
        && match state.phase {
            M1AuthenticatedPrefillBootstrapPhaseModelV1::ReadyForBootstrap => {
                state.prompt_tokens == 0
                    && state.direct_output_tokens == 0
                    && state.successor_output_tokens == 0
                    && state.expected_output_tokens == 0
            },
            M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning
            | M1AuthenticatedPrefillBootstrapPhaseModelV1::QueueReadyPrepublication => {
                m1_authenticated_prefill_bootstrap_input_admitted_spec_v1(
                    M1AuthenticatedPrefillBootstrapInputModelV1 {
                        prompt_tokens: state.prompt_tokens,
                        direct_output_tokens: state.direct_output_tokens,
                        successor_output_tokens: state.successor_output_tokens,
                        expected_output_tokens: state.expected_output_tokens,
                    },
                )
            },
            M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine
            | M1AuthenticatedPrefillBootstrapPhaseModelV1::Stopped => true,
        }
}

/// Constructs the exact pre-bootstrap state for one bound R33 instance.
#[must_use]
pub fn m1_authenticated_prefill_bootstrap_ready_state_v1(
    instance: u64,
) -> (state: M1AuthenticatedPrefillBootstrapStateModelV1)
    ensures
        state.phase == M1AuthenticatedPrefillBootstrapPhaseModelV1::ReadyForBootstrap,
        state.instance == instance,
        m1_authenticated_prefill_bootstrap_state_well_formed_v1(state),
{
    M1AuthenticatedPrefillBootstrapStateModelV1 {
        phase: M1AuthenticatedPrefillBootstrapPhaseModelV1::ReadyForBootstrap,
        instance,
        prompt_tokens: 0,
        direct_output_tokens: 0,
        successor_output_tokens: 0,
        expected_output_tokens: 0,
        physical_completion_observed: false,
        measurement_reports: 0,
        served_output_tokens: 0,
    }
}

/// Mathematical admission-to-transition or quarantine relation.
pub open spec fn m1_authenticated_prefill_bootstrap_begin_spec_v1(
    state: M1AuthenticatedPrefillBootstrapStateModelV1,
    input: M1AuthenticatedPrefillBootstrapInputModelV1,
) -> M1AuthenticatedPrefillBootstrapStateModelV1 {
    match state.phase {
        M1AuthenticatedPrefillBootstrapPhaseModelV1::ReadyForBootstrap => {
            M1AuthenticatedPrefillBootstrapStateModelV1 {
                phase: if m1_authenticated_prefill_bootstrap_input_admitted_spec_v1(input) {
                    M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning
                } else {
                    M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine
                },
                instance: state.instance,
                prompt_tokens: input.prompt_tokens,
                direct_output_tokens: input.direct_output_tokens,
                successor_output_tokens: input.successor_output_tokens,
                expected_output_tokens: input.expected_output_tokens,
                physical_completion_observed: false,
                measurement_reports: 0,
                served_output_tokens: 0,
            }
        },
        _ => state,
    }
}

/// Admits one exact bootstrap input or moves it to terminal quarantine.
#[must_use]
pub fn begin_m1_authenticated_prefill_bootstrap_v1(
    state: M1AuthenticatedPrefillBootstrapStateModelV1,
    input: M1AuthenticatedPrefillBootstrapInputModelV1,
) -> (next: M1AuthenticatedPrefillBootstrapStateModelV1)
    requires m1_authenticated_prefill_bootstrap_state_well_formed_v1(state),
    ensures
        next == m1_authenticated_prefill_bootstrap_begin_spec_v1(state, input),
        m1_authenticated_prefill_bootstrap_state_well_formed_v1(next),
        m1_authenticated_prefill_bootstrap_phase_rank_spec_v1(next.phase)
            >= m1_authenticated_prefill_bootstrap_phase_rank_spec_v1(state.phase),
        m1_authenticated_prefill_bootstrap_is_precompletion_silent_v1(next),
{
    match state.phase {
        M1AuthenticatedPrefillBootstrapPhaseModelV1::ReadyForBootstrap => {
            M1AuthenticatedPrefillBootstrapStateModelV1 {
                phase: if check_m1_authenticated_prefill_bootstrap_input_v1(input) {
                    M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning
                } else {
                    M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine
                },
                instance: state.instance,
                prompt_tokens: input.prompt_tokens,
                direct_output_tokens: input.direct_output_tokens,
                successor_output_tokens: input.successor_output_tokens,
                expected_output_tokens: input.expected_output_tokens,
                physical_completion_observed: false,
                measurement_reports: 0,
                served_output_tokens: 0,
            }
        },
        _ => state,
    }
}

/// Mathematical transition completion relation.
pub open spec fn m1_authenticated_prefill_bootstrap_finish_spec_v1(
    state: M1AuthenticatedPrefillBootstrapStateModelV1,
    preparation_succeeded: bool,
) -> M1AuthenticatedPrefillBootstrapStateModelV1 {
    match state.phase {
        M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning => {
            M1AuthenticatedPrefillBootstrapStateModelV1 {
                phase: if preparation_succeeded {
                    M1AuthenticatedPrefillBootstrapPhaseModelV1::QueueReadyPrepublication
                } else {
                    M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine
                },
                ..state
            }
        },
        _ => state,
    }
}

/// Finishes preparation at queue-input-ready prepublication or quarantine.
#[must_use]
pub fn finish_m1_authenticated_prefill_bootstrap_v1(
    state: M1AuthenticatedPrefillBootstrapStateModelV1,
    preparation_succeeded: bool,
) -> (next: M1AuthenticatedPrefillBootstrapStateModelV1)
    requires m1_authenticated_prefill_bootstrap_state_well_formed_v1(state),
    ensures
        next == m1_authenticated_prefill_bootstrap_finish_spec_v1(
            state,
            preparation_succeeded,
        ),
        m1_authenticated_prefill_bootstrap_state_well_formed_v1(next),
        m1_authenticated_prefill_bootstrap_phase_rank_spec_v1(next.phase)
            >= m1_authenticated_prefill_bootstrap_phase_rank_spec_v1(state.phase),
        m1_authenticated_prefill_bootstrap_is_precompletion_silent_v1(next),
{
    match state.phase {
        M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning => {
            M1AuthenticatedPrefillBootstrapStateModelV1 {
                phase: if preparation_succeeded {
                    M1AuthenticatedPrefillBootstrapPhaseModelV1::QueueReadyPrepublication
                } else {
                    M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine
                },
                ..state
            }
        },
        _ => state,
    }
}

/// Mathematical exact-stop relation for this precompletion lifecycle.
pub open spec fn m1_authenticated_prefill_bootstrap_stop_spec_v1(
    state: M1AuthenticatedPrefillBootstrapStateModelV1,
    instance: u64,
) -> M1AuthenticatedPrefillBootstrapStopOutcomeV1 {
    match state.phase {
        M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning => {
            M1AuthenticatedPrefillBootstrapStopOutcomeV1::Rejected(state)
        },
        _ => {
            if state.instance == instance {
                M1AuthenticatedPrefillBootstrapStopOutcomeV1::Accepted(
                    M1AuthenticatedPrefillBootstrapStateModelV1 {
                        phase: M1AuthenticatedPrefillBootstrapPhaseModelV1::Stopped,
                        ..state
                    },
                )
            } else {
                M1AuthenticatedPrefillBootstrapStopOutcomeV1::Rejected(state)
            }
        },
    }
}

/// Extracts the retained state from either exact-stop outcome.
pub open spec fn m1_authenticated_prefill_bootstrap_stop_state_v1(
    outcome: M1AuthenticatedPrefillBootstrapStopOutcomeV1,
) -> M1AuthenticatedPrefillBootstrapStateModelV1 {
    match outcome {
        M1AuthenticatedPrefillBootstrapStopOutcomeV1::Accepted(state)
        | M1AuthenticatedPrefillBootstrapStopOutcomeV1::Rejected(state) => state,
    }
}

/// Applies exact stop without allowing the unbound transition sentinel to stop.
#[must_use]
pub fn stop_m1_authenticated_prefill_bootstrap_v1(
    state: M1AuthenticatedPrefillBootstrapStateModelV1,
    instance: u64,
) -> (outcome: M1AuthenticatedPrefillBootstrapStopOutcomeV1)
    requires m1_authenticated_prefill_bootstrap_state_well_formed_v1(state),
    ensures
        outcome == m1_authenticated_prefill_bootstrap_stop_spec_v1(state, instance),
        m1_authenticated_prefill_bootstrap_state_well_formed_v1(
            m1_authenticated_prefill_bootstrap_stop_state_v1(outcome),
        ),
        m1_authenticated_prefill_bootstrap_phase_rank_spec_v1(
            m1_authenticated_prefill_bootstrap_stop_state_v1(outcome).phase,
        ) >= m1_authenticated_prefill_bootstrap_phase_rank_spec_v1(state.phase),
        m1_authenticated_prefill_bootstrap_is_precompletion_silent_v1(
            m1_authenticated_prefill_bootstrap_stop_state_v1(outcome),
        ),
{
    match state.phase {
        M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning => {
            M1AuthenticatedPrefillBootstrapStopOutcomeV1::Rejected(state)
        },
        _ => {
            if state.instance == instance {
                M1AuthenticatedPrefillBootstrapStopOutcomeV1::Accepted(
                    M1AuthenticatedPrefillBootstrapStateModelV1 {
                        phase: M1AuthenticatedPrefillBootstrapPhaseModelV1::Stopped,
                        ..state
                    },
                )
            } else {
                M1AuthenticatedPrefillBootstrapStopOutcomeV1::Rejected(state)
            }
        },
    }
}

/// Proves the exact successful S1/T128 prepublication accounting boundary.
#[must_use]
pub fn m1_authenticated_s1_t128_prefill_prepublication_theorem_v1(
    instance: u64,
    successor_output_tokens: u64,
) -> (state: M1AuthenticatedPrefillBootstrapStateModelV1)
    requires
        successor_output_tokens > 0,
        successor_output_tokens < u64::MAX,
    ensures
        state.phase
            == M1AuthenticatedPrefillBootstrapPhaseModelV1::QueueReadyPrepublication,
        state.instance == instance,
        state.prompt_tokens == M1_AUTHENTICATED_PREFILL_BOOTSTRAP_PROMPT_TOKENS_V1,
        state.direct_output_tokens
            == M1_AUTHENTICATED_PREFILL_BOOTSTRAP_DIRECT_OUTPUT_TOKENS_V1,
        state.successor_output_tokens == successor_output_tokens,
        state.expected_output_tokens == successor_output_tokens + 1,
        m1_authenticated_prefill_bootstrap_is_precompletion_silent_v1(state),
{
    let source = m1_authenticated_prefill_bootstrap_ready_state_v1(instance);
    let input = M1AuthenticatedPrefillBootstrapInputModelV1 {
        prompt_tokens: M1_AUTHENTICATED_PREFILL_BOOTSTRAP_PROMPT_TOKENS_V1,
        direct_output_tokens: M1_AUTHENTICATED_PREFILL_BOOTSTRAP_DIRECT_OUTPUT_TOKENS_V1,
        successor_output_tokens,
        expected_output_tokens: successor_output_tokens + 1,
    };
    let transitioning = begin_m1_authenticated_prefill_bootstrap_v1(source, input);
    finish_m1_authenticated_prefill_bootstrap_v1(transitioning, true)
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    const ENGINE_SOURCE: &str =
        include_str!("../../crates/ferric-engine/src/authenticated_prefill_bootstrap.rs");
    const R33_BACKEND_SOURCE: &str =
        include_str!("../../adapters/m1-engineering-execution-v1/src/r33_production_backend.rs");
    const MODEL_SOURCE: &str = include_str!("authenticated_prefill_bootstrap.rs");

    fn exact_input(successor_output_tokens: u64) -> M1AuthenticatedPrefillBootstrapInputModelV1 {
        M1AuthenticatedPrefillBootstrapInputModelV1 {
            prompt_tokens: 128,
            direct_output_tokens: 1,
            successor_output_tokens,
            expected_output_tokens: successor_output_tokens + 1,
        }
    }

    fn ready() -> M1AuthenticatedPrefillBootstrapStateModelV1 {
        m1_authenticated_prefill_bootstrap_ready_state_v1(7)
    }

    fn assert_precompletion_silent(state: M1AuthenticatedPrefillBootstrapStateModelV1) {
        assert!(!state.physical_completion_observed);
        assert_eq!(state.measurement_reports, 0);
        assert_eq!(state.served_output_tokens, 0);
    }

    fn unique_offset(source: &str, needle: &str) -> usize {
        assert_eq!(
            source.matches(needle).count(),
            1,
            "non-unique source token: {needle}"
        );
        source.find(needle).expect("source token missing")
    }

    #[test]
    fn exact_admission_requires_s1_t128_and_direct_plus_successor_accounting() {
        assert!(check_m1_authenticated_prefill_bootstrap_input_v1(
            exact_input(32)
        ));
        for rejected in [
            M1AuthenticatedPrefillBootstrapInputModelV1 {
                prompt_tokens: 127,
                ..exact_input(32)
            },
            M1AuthenticatedPrefillBootstrapInputModelV1 {
                prompt_tokens: 129,
                ..exact_input(32)
            },
            M1AuthenticatedPrefillBootstrapInputModelV1 {
                direct_output_tokens: 0,
                ..exact_input(32)
            },
            M1AuthenticatedPrefillBootstrapInputModelV1 {
                successor_output_tokens: 0,
                expected_output_tokens: 1,
                ..exact_input(32)
            },
            M1AuthenticatedPrefillBootstrapInputModelV1 {
                expected_output_tokens: 32,
                ..exact_input(32)
            },
            M1AuthenticatedPrefillBootstrapInputModelV1 {
                successor_output_tokens: u64::MAX,
                expected_output_tokens: 0,
                ..exact_input(32)
            },
        ] {
            assert!(!check_m1_authenticated_prefill_bootstrap_input_v1(rejected));
        }
    }

    #[test]
    fn successful_prepublication_is_monotone_and_precompletion_silent() {
        let source = ready();
        let transitioning = begin_m1_authenticated_prefill_bootstrap_v1(source, exact_input(32));
        assert_eq!(
            transitioning.phase,
            M1AuthenticatedPrefillBootstrapPhaseModelV1::Transitioning
        );
        assert!(
            m1_authenticated_prefill_bootstrap_phase_rank_v1(transitioning.phase)
                > m1_authenticated_prefill_bootstrap_phase_rank_v1(source.phase)
        );
        let prepared = finish_m1_authenticated_prefill_bootstrap_v1(transitioning, true);
        assert_eq!(
            prepared.phase,
            M1AuthenticatedPrefillBootstrapPhaseModelV1::QueueReadyPrepublication
        );
        assert_eq!(prepared.prompt_tokens, 128);
        assert_eq!(prepared.direct_output_tokens, 1);
        assert_eq!(prepared.successor_output_tokens, 32);
        assert_eq!(prepared.expected_output_tokens, 33);
        assert_precompletion_silent(prepared);
        assert_eq!(
            prepared,
            m1_authenticated_s1_t128_prefill_prepublication_theorem_v1(7, 32)
        );
    }

    #[test]
    fn failed_admission_or_preparation_is_terminal_and_silent() {
        let invalid = begin_m1_authenticated_prefill_bootstrap_v1(
            ready(),
            M1AuthenticatedPrefillBootstrapInputModelV1 {
                prompt_tokens: 127,
                ..exact_input(32)
            },
        );
        assert_eq!(
            invalid.phase,
            M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine
        );
        assert_precompletion_silent(invalid);

        let transitioning = begin_m1_authenticated_prefill_bootstrap_v1(ready(), exact_input(32));
        let rejected = finish_m1_authenticated_prefill_bootstrap_v1(transitioning, false);
        assert_eq!(
            rejected.phase,
            M1AuthenticatedPrefillBootstrapPhaseModelV1::TerminalQuarantine
        );
        assert_precompletion_silent(rejected);
    }

    #[test]
    fn unbound_transition_rejects_stop_but_quarantine_accepts_only_exact_stop() {
        let transitioning = begin_m1_authenticated_prefill_bootstrap_v1(ready(), exact_input(32));
        assert_eq!(
            stop_m1_authenticated_prefill_bootstrap_v1(transitioning, 7),
            M1AuthenticatedPrefillBootstrapStopOutcomeV1::Rejected(transitioning)
        );

        let quarantined = finish_m1_authenticated_prefill_bootstrap_v1(transitioning, false);
        assert_eq!(
            stop_m1_authenticated_prefill_bootstrap_v1(quarantined, 8),
            M1AuthenticatedPrefillBootstrapStopOutcomeV1::Rejected(quarantined)
        );
        let M1AuthenticatedPrefillBootstrapStopOutcomeV1::Accepted(stopped) =
            stop_m1_authenticated_prefill_bootstrap_v1(quarantined, 7)
        else {
            panic!("exact bound stop must be accepted");
        };
        assert_eq!(
            stopped.phase,
            M1AuthenticatedPrefillBootstrapPhaseModelV1::Stopped
        );
        assert_eq!(
            stop_m1_authenticated_prefill_bootstrap_v1(stopped, 7),
            M1AuthenticatedPrefillBootstrapStopOutcomeV1::Accepted(stopped)
        );
    }

    #[test]
    fn production_sources_pin_exact_accounting_prepublication_and_stop_boundaries() {
        let width = unique_offset(ENGINE_SOURCE, "const PREFILL_WIDTH: usize = 128;");
        let ready = unique_offset(ENGINE_SOURCE, "engine.append_tentative(request, 1)");
        let active = unique_offset(ENGINE_SOURCE, "let active_tokens = 128_u32;");
        let prepare = unique_offset(
            ENGINE_SOURCE,
            "runner.prepare_first_step(allocated, recipe, completion)",
        );
        let success = unique_offset(
            ENGINE_SOURCE,
            "Ok(M1AuthenticatedS1T128PrefillPrepublicationV1 {",
        );
        assert!(width < ready);
        assert!(ready < active);
        assert!(active < prepare);
        assert!(prepare < success);
        for nonclaim in ["create or submit a queue", "read a token", "claim serving"] {
            assert!(ENGINE_SOURCE.contains(nonclaim));
        }

        let accounting = unique_offset(
            R33_BACKEND_SOURCE,
            "u64::from(input.maximum_successor_output_tokens()).checked_add(1)",
        );
        let measure_start = unique_offset(R33_BACKEND_SOURCE, "    fn measure(\n");
        let measure_end = measure_start
            + R33_BACKEND_SOURCE[measure_start..]
                .find("    fn stop(\n")
                .expect("R33 measure end is absent");
        let measure = &R33_BACKEND_SOURCE[measure_start..measure_end];
        let preparation = unique_offset(
            measure,
            "prepare_m1_authenticated_s1_t128_prefill_prepublication_v1(",
        );
        let execution_mode = unique_offset(
            measure,
            "let M1R33AuthenticatedExecutionModeV1::TargetWindow {",
        );
        let bootstrap_rejection = &measure[preparation..execution_mode];
        let bootstrap_fault = preparation
            + unique_offset(
                bootstrap_rejection,
                "self.state.state = BackendStateV1::Faulted {",
            );
        let bootstrap_reject = preparation
            + unique_offset(
                bootstrap_rejection,
                "return Err(fault(FAULT_BOOTSTRAP_REJECTED));",
            );
        let execution = unique_offset(
            measure,
            "match execute_m1_authenticated_s1_t128_target_window_v1(",
        );
        let report = unique_offset(measure, "let report = M1R33MeasurementReportV1 {");
        let validation = unique_offset(
            measure,
            "if report.validate_against(&window.row.expected_work).is_err() {",
        );
        let success = unique_offset(measure, "Ok(report)");
        let validated_success = &measure[validation..success];
        let validation_reject_offset = unique_offset(
            validated_success,
            "return Err(fault(FAULT_EXECUTION_REJECTED));",
        );
        let validation_fault = validation
            + unique_offset(
                &validated_success[..validation_reject_offset],
                "self.state.state = BackendStateV1::Faulted {",
            );
        let validation_reject = validation + validation_reject_offset;
        let terminal_fault = validation_reject
            + unique_offset(
                &measure[validation_reject..success],
                "self.state.state = BackendStateV1::Faulted {",
            );
        assert!(accounting < measure_start);
        assert!(preparation < bootstrap_fault);
        assert!(bootstrap_fault < bootstrap_reject);
        assert!(bootstrap_reject < execution_mode);
        assert!(execution_mode < execution);
        assert!(execution < report);
        assert!(report < validation);
        assert!(validation < validation_fault);
        assert!(validation_fault < validation_reject);
        assert!(validation_reject < terminal_fault);
        assert!(terminal_fault < success);

        let binding = unique_offset(
            R33_BACKEND_SOURCE,
            "Self::Transitioning | Self::Dormant { .. } => None,",
        );
        let transition_rejection = unique_offset(
            R33_BACKEND_SOURCE,
            "BackendStateV1::Transitioning => return Err(fault(FAULT_NOT_ACTIVE)),",
        );
        assert!(binding < transition_rejection);
    }

    #[test]
    fn model_keeps_runtime_and_serving_effects_as_explicit_nonclaims() {
        for excluded in [
            "do not prove that the production",
            "Rust implementation refines this model",
            "liveness",
            "`Instant`",
            "timing",
            "allocation",
            "KFD",
            "queue creation or publication",
            "physical completion or",
            "readback",
            "device-memory",
            "report-authenticity",
            "serving",
            "M1 authority",
        ] {
            assert!(
                MODEL_SOURCE.contains(excluded),
                "missing nonclaim: {excluded}"
            );
        }
    }
}
