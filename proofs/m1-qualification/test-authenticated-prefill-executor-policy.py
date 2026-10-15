#!/usr/bin/env python3
"""Freeze the bounded authenticated paired-prefill executor surface."""

from __future__ import annotations

import contextlib
import io
import re
import sys
from pathlib import Path
from typing import NoReturn


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def require(source: str, needle: str, label: str) -> int:
    position = source.find(needle)
    if position < 0:
        fail(f"authenticated prefill executor lost {label}")
    return position


def failure_surface_is_closed(source: str) -> bool:
    methods = re.findall(r"\bfn\s+([A-Za-z_]\w*)\s*(?:<|\()", source)
    if any(
        forbidden in method
        for method in methods
        for forbidden in ["retry", "prepublication", "queue", "released"]
    ):
        return False
    return all(
        required in source
        for required in [
            "pub(crate) fn into_resident_teardown(",
            "self: Box<Self>",
            "M1AuthenticatedResidentQueueTeardownV1",
            "Status::NoQueue => Teardown::no_queue(self)",
            "Status::Released => Teardown::released(self)",
            "Status::Quarantined => Teardown::quarantined(self)",
            "M1CaptureQuarantinedEngineV1<C>",
            "(self.engine, self.stage, self.error, self.retained.0)",
        ]
    )



def validate_resident_outcomes(production: str) -> None:
    compact = lambda text: re.sub(r"\s+", "", text)

    def contains(source: str, expected: str, label: str) -> None:
        if compact(expected) not in compact(source):
            fail(f"resident prefill custody lost {label}")

    wrapper, core = production.split(
        "pub(crate) fn execute_m1_authenticated_s1_t128_paired_prefill_with_deadline_v1", 1
    )
    contains(wrapper, """
        execute_m1_authenticated_s1_t128_paired_prefill_with_deadline_v1(
            prepared, diagnostic_ring_bytes, queue_wait_timeout, false,
            |_, configured| Some(configured),
        )? {
            M1AuthenticatedResidentPrefillOutcomeV1::Continuing(success) => Ok(success),
            M1AuthenticatedResidentPrefillOutcomeV1::Terminal(terminal) => Err(terminal.reject()),
        }
    """, "public continuing-only handoff and terminal rejection")
    contains(core, """
        let successor = M1AuthenticatedS1T128PrefillSuccessorCustodyV1 {
            draft_rollover_page, target_rollover_pages, rollover_intent, request,
            prompt_tokens, policy, diagnostic_ring_bytes, queue_wait_timeout,
        };
    """, "all successor owners before queue creation")
    contains(core, """
        let terminal = retire_prefill_stop(retire_stop_tokens, policy, first_token);
        members.push(if terminal {
            M1DeviceKvCompletionMemberV1::retiring(cache)
        } else {
            M1DeviceKvCompletionMemberV1::continuing(cache)
        });
    """, "terminal/continuing physical member disposition")
    tail = """
        if terminal {
            Ok(M1AuthenticatedResidentPrefillOutcomeV1::Terminal(
                M1AuthenticatedTerminalPrefillV1 {
                    engine, released, first_token, direct_choices, successor,
                },
            ))
        } else {
            Ok(M1AuthenticatedResidentPrefillOutcomeV1::Continuing(
                M1AuthenticatedS1T128PrefillExecutionSuccessV1 {
                    engine, released, first_token, direct_choices, successor,
                },
            ))
        }
    }
    """
    if not compact(core).endswith(compact(tail)):
        fail("resident prefill outcomes do not consume exact rightful custody")
    if compact(core).count("Ok(M1AuthenticatedResidentPrefillOutcomeV1::") != 2 or "returnOk(" in compact(core):
        fail("resident prefill can publish an early or duplicated success")
    contains(production, "retire_stop_tokens && policy.stop_tokens().contains(&token)",
             "explicit stop-retirement decision")
    terminal = production.split(
        "impl<const C: usize> M1AuthenticatedTerminalPrefillV1<C> {", 1
    )[1].split("impl<const C: usize> M1AuthenticatedResidentPrefillOutcomeV1<C>", 1)[0]
    methods = re.findall(r"\bfn\s+([A-Za-z_]\w*)\s*\(", terminal)
    if methods != ["first_token", "policy", "reject", "shutdown_for_resident"]:
        fail("terminal prefill exposes live successor recovery")
    contains(terminal, """
        let teardown = released.destroy_queue_and_retain_step(&mut engine);
        let status = teardown_result_status(&teardown);
        terminal_failure(engine,
            M1AuthenticatedS1T128PrefillExecutionStageV1::PrefillPageRelease,
            M1AuthenticatedS1T128PrefillExecutionErrorV1::LowerRejected,
            status, (teardown, first_token, direct_choices, successor),
        )
    """, "terminal rejection closes and retains all owners")
    contains(terminal, """
        let retained = (first_token, direct_choices, successor);
        match released.shutdown_all_terminal_queue(&mut engine) {
            Ok(closed) => Ok(Teardown::released((engine, closed, retained))),
    """, "all-terminal shutdown with retained evidence")
    contains(terminal, """
        let (error, released) = rejection.into_parts();
        match released.destroy_queue_and_retain_step(&mut engine) {
            Ok(closed) => Err(Teardown::released((engine, closed, retained, error))),
            Err(quarantined) => Err(Teardown::quarantined((
                engine, quarantined, retained, error,
            ))),
        }
    """, "rejected all-terminal shutdown closes without relabeling")
    contains(terminal, """
        Err(crate::M1AuthenticatedReleasedAllTerminalQueueShutdownFailureV1::Quarantined(
            quarantined,
        )) => Err(Teardown::quarantined((engine, quarantined, retained))),
    """, "quarantined all-terminal shutdown remains quarantined")
    if compact(terminal).count("Teardown::quarantined(") != 2:
        fail("terminal teardown quarantine paths drifted")
    outcome = production.split(
        "impl<const C: usize> M1AuthenticatedResidentPrefillOutcomeV1<C> {", 1
    )[1].split("/// Rollover-ready custody", 1)[0]
    contains(outcome, """
        match self {
            Self::Continuing(value) => value.close_for_resident(),
            Self::Terminal(value) => value.reject().into_resident_teardown(),
        }
    """, "resident outcome closure never resurrects terminal custody")
    status = production.split("fn teardown_result_status", 1)[1].split("fn completed_step_closure_status", 1)[0]
    contains(status, """
        if result.is_ok() {
            M1AuthenticatedResidentQueueTeardownStatusV1::Released
        } else {
            M1AuthenticatedResidentQueueTeardownStatusV1::Quarantined
        }
    """, "actual teardown status is not promoted")
    success = production.split(
        "impl<const C: usize> M1AuthenticatedS1T128PrefillExecutionSuccessV1<C> {", 1
    )[1].split("/// Executes one exact authenticated", 1)[0]
    contains(success, """
        successor: M1AuthenticatedS1T128PrefillSuccessorCustodyV1 {
            draft_rollover_page, target_rollover_pages, rollover_intent, request,
            prompt_tokens, policy, diagnostic_ring_bytes, queue_wait_timeout,
        },
    """, "continuing into_parts retains all successor custody")
    contains(success, """
        (engine, released, first_token, direct_choices, draft_rollover_page,
         target_rollover_pages, rollover_intent, request, prompt_tokens, policy,
         diagnostic_ring_bytes, queue_wait_timeout,)
    """, "continuing handoff preserves exact owner order")


def validate(executor: str, lib: str, fixture: str) -> None:
    production = executor.split("#[cfg(test)]", 1)[0]

    entry = "pub fn execute_m1_authenticated_s1_t128_paired_prefill_v1"
    if production.count(entry) != 1:
        fail("production entry point is absent or duplicated")
    require(
        production,
        "prepared: M1AuthenticatedS1T128PrefillPrepublicationV1<C>",
        "authenticated prepublication input",
    )
    require(production, "diagnostic_ring_bytes: u32", "explicit diagnostic ring bytes")
    require(
        production,
        "queue_wait_timeout: M1QueueWaitTimeoutV1",
        "typed bounded wait input",
    )

    ordered = [
        "M1AuthenticatedPhysicalQueueSessionV1::create(",
        ".submit()",
        "let Some(wait_timeout) = deadline_expired(Boundary::BeforeCompletionWait, queue_wait_timeout)",
        "published.wait_for(wait_timeout.milliseconds())",
        "completed.recycle()",
        "recycled.observe_completion()",
        "failure.retry()",
        "observed.observe_direct_diagnostic_choices()",
        "direct.check_completion()",
        "require_one_authenticated_prefill_choice(direct.choices())",
        "complete_m1_authenticated_physical_step_v1(",
        "release_m1_authenticated_completed_step_kv_pages_v1(physical)",
        "Ok(M1AuthenticatedResidentPrefillOutcomeV1::Terminal(",
    ]
    cursor = 0
    for needle in ordered:
        position = production.find(needle, cursor)
        if position < 0:
            fail(f"authenticated execution ordering lost {needle}")
        cursor = position + len(needle)

    validate_resident_outcomes(production)

    for forbidden in [
        ".wait()",
        "CompletionWireSemanticExpectation",
        "FERRIC_M1_ROLLOVER_PREFILL_TOKEN",
        ".expect(",
        ".unwrap(",
        "expect!",
        "panic!",
        "unreachable!",
        "todo!",
    ]:
        if forbidden in production:
            fail(f"production source contains forbidden token {forbidden}")

    observation_start = require(
        production, "let observed = match recycled.observe_completion()", "compact observation"
    )
    observation_end = require(
        production[observation_start:],
        "let direct = match observed.observe_direct_diagnostic_choices()",
        "direct observation after compact observation",
    ) + observation_start
    observation = production[observation_start:observation_end]
    if production.count(".retry()") != 1 or observation.count("failure.retry()") != 1:
        fail("compact observation must contain the only production retry")
    require(
        observation,
        "(*failure).destroy_queue_and_retain_evidence(&mut engine)",
        "terminal teardown after exhausted or closed observation retry",
    )

    require(
        production,
        "failure.close_without_authority(&mut engine)",
        "submit-currentness queue closure",
    )
    if production.count("destroy_queue_and_retain_evidence(&mut engine)") < 3:
        fail("observation, direct-join, or cardinality queue closure is absent")
    require(
        production,
        "close_m1_authenticated_completed_step_outcome_v1(",
        "physical-completion queue closure",
    )
    require(
        production,
        "physical.destroy_queue_and_retain_completion(&mut engine)",
        "page-release queue closure",
    )

    failure_impl = production.split(
        "impl<const C: usize> M1AuthenticatedS1T128PrefillExecutionFailureV1<C>", 1
    )[1].split("fn terminal_failure", 1)[0]
    if not failure_surface_is_closed(failure_impl):
        fail("terminal failure surface exposes recovery or loses closed teardown custody")
    for method in ["retry", "into_prepublication", "queue", "released"]:
        hostile = failure_impl.replace("pub const fn stage(", f"pub const fn {method}(", 1)
        if hostile == failure_impl or failure_surface_is_closed(hostile):
            fail(f"terminal failure policy accepts forbidden method {method}")
    hostile = failure_impl.replace(
        "Status::Quarantined => Teardown::quarantined(self)",
        "Status::Quarantined => Teardown::released(self)",
        1,
    )
    if hostile == failure_impl or failure_surface_is_closed(hostile):
        fail("terminal failure policy accepts quarantine relabeling")
    require(
        production,
        "OpaqueM1AuthenticatedS1T128PrefillExecutionCustodyV1(Box<dyn fmt::Debug>)",
        "opaque retained failure custody",
    )
    require(
        production,
        "engine: engine.into_m1_capture_quarantine()",
        "consumed Engine quarantine",
    )
    require(production, "let [choice] = choices.choices() else", "exact one-choice gate")
    require(
        executor,
        "fn hostile_direct_choice_cardinalities_fail_closed()",
        "hostile zero/two-choice regression",
    )
    require(executor, "fn recover_live(failure:", "compile-fail owner-extraction guard")
    require(executor, "let _ = failure.into_prepublication();", "closed failure compile-fail")

    for retained in [
        "engine: Engine<C>",
        "released: M1AuthenticatedReleasedCompletedStepV1",
        "first_token: TokenId",
        "direct_choices: M1ObservedDirectDiagnosticChoicesV1",
        "draft_rollover_page: DeviceKvPageLease",
        "target_rollover_pages: Vec<DeviceKvPageLease>",
        "rollover_intent: M1AuthenticatedSpeculativeRolloverIntentV1",
        "prompt_tokens: Box<[TokenId]>",
        "policy: M1SpeculativeGenerationPolicyV1",
    ]:
        require(production, retained, f"rollover success custody {retained}")
    require(production, "pub fn into_parts(", "linear success handoff")
    require(
        production,
        "This module does not\n//! execute rollover, observe clocks, publish R33 evidence, or claim serving.",
        "bounded nonclaims",
    )

    require(lib, "mod authenticated_prefill_executor;", "module registration")
    require(
        lib,
        "execute_m1_authenticated_s1_t128_paired_prefill_v1",
        "public executor export",
    )
    fixture = fixture.split(
        "fn admitted_mi300x_runs_public_authenticated_rollover_executor()", 1
    )[1].split("#[test]", 1)[0]
    require(
        fixture,
        "execute_m1_authenticated_s1_t128_paired_prefill_v1(",
        "MI300X fixture production prefill execution",
    )
    cursor = 0
    for needle in [
        "let request = executed.request();",
        "let anchor = executed.first_token();",
        "let reconciled = reconcile_m1_authenticated_s1_t128_prefill_registry_v1(",
        "            executed,\n        )",
        "reconciled.schedule_first_speculative_round(rollover_inputs)",
        "scheduled.prepare(&logical_runner)",
        "prepared.publish()",
        ".complete_round(vec![M1SpeculativeMemberControlV1::continuing(request)])",
        "assert_eq!(completed.first_token(), anchor);",
    ]:
        position = fixture.find(needle, cursor)
        if position < 0:
            fail(f"MI300X fixture authenticated registry handoff lost {needle}")
        cursor = position + len(needle)
    if "executed.into_parts()" in fixture:
        fail("MI300X fixture bypasses the authenticated registry handoff")
    require(
        fixture,
        "assert!(failure.engine_quarantined());",
        "closed MI300X fixture failure",
    )
    for duplicated in [
        "M1AuthenticatedPhysicalQueueSessionV1::create",
        "observed.observe_direct_diagnostic_choices()",
        "complete_m1_authenticated_physical_step_v1",
        "release_m1_authenticated_completed_step_kv_pages_v1",
    ]:
        if duplicated in fixture:
            fail(f"MI300X fixture duplicates production transition {duplicated}")
    print(
        "PASS: authenticated paired-prefill execution is bounded, direct-evidence "
        "authorized, queue-closing, rollover-ready, and explicitly non-serving"
    )


def main() -> None:
    root = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    executor = (root / "crates/ferric-engine/src/authenticated_prefill_executor.rs").read_text(encoding="utf-8")
    lib = (root / "crates/ferric-engine/src/lib.rs").read_text(encoding="utf-8")
    fixture = (root / "crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs").read_text(encoding="utf-8")
    validate(executor, lib, fixture)
    mutations = [
        ("branch swap", "\n    if terminal {\n        Ok(M1AuthenticatedResidentPrefillOutcomeV1::Terminal(", "\n    if !terminal {\n        Ok(M1AuthenticatedResidentPrefillOutcomeV1::Terminal("),
        ("wrong outcome", "Ok(M1AuthenticatedResidentPrefillOutcomeV1::Terminal(", "Ok(M1AuthenticatedResidentPrefillOutcomeV1::Continuing("),
        ("completion bypass", "let physical = complete_m1_authenticated_physical_step_v1(", "let physical = bypass_completion("),
        ("release bypass", "match release_m1_authenticated_completed_step_kv_pages_v1(physical)", "match bypass_release(physical)"),
        ("continuing custody", "M1AuthenticatedS1T128PrefillExecutionSuccessV1 {\n                engine,\n                released,", "M1AuthenticatedS1T128PrefillExecutionSuccessV1 {\n                engine,"),
        ("member disposition", "M1DeviceKvCompletionMemberV1::retiring(cache)", "M1DeviceKvCompletionMemberV1::continuing(cache)"),
        ("terminal close", "match released.shutdown_all_terminal_queue(&mut engine)", "match bypass_terminal_close(&mut engine)"),
        ("terminal quarantine promotion", "Err(Teardown::quarantined((", "Err(Teardown::released(("),
        ("retry duplicate", "failure.retry()", "{ let _ = failure.retry(); failure.retry() }"),
        ("terminal resurrection", "M1AuthenticatedResidentPrefillOutcomeV1::Terminal(terminal) => Err(terminal.reject())", "M1AuthenticatedResidentPrefillOutcomeV1::Terminal(terminal) => Ok(terminal)"),
        ("lost terminal evidence", "let retained = (first_token, direct_choices, successor);", "let retained = (first_token, successor);"),
        ("outcome closure bypass", "Self::Terminal(value) => value.reject().into_resident_teardown()", "Self::Terminal(value) => value.close_for_resident()"),
        ("cardinality bypass", "let [choice] = choices.choices() else", "let [choice, _] = choices.choices() else"),
        ("early success", "let physical = complete_m1_authenticated_physical_step_v1(", "return Ok(fake_authority());\n    let physical = complete_m1_authenticated_physical_step_v1("),
    ]
    for label, old, new in mutations:
        if old not in executor:
            fail(f"negative control fixture missing: {label}")
        hostile = executor.replace(old, new, 1)
        with contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            try:
                validate(hostile, lib, fixture)
            except SystemExit as error:
                if error.code == 1:
                    continue
                raise
        fail(f"authenticated prefill policy accepted mutation: {label}")
    for label, old, new in [
        ("registry bypass", "reconciled.schedule_first_speculative_round(rollover_inputs)", "executed.into_parts()"),
        ("generic fake fixture", "scheduled.prepare(&logical_runner)", "generic_fake_prepare(&logical_runner)"),
    ]:
        if old not in fixture:
            fail(f"fixture negative control missing: {label}")
        with contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            try:
                validate(executor, lib, fixture.replace(old, new, 1))
            except SystemExit as error:
                if error.code == 1:
                    continue
                raise
        fail(f"authenticated fixture policy accepted mutation: {label}")
    print("PASS: 14 resident-outcome/custody and 2 fixture hostile mutations rejected")


if __name__ == "__main__":
    main()
