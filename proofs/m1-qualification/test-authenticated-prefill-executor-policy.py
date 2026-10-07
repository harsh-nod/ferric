#!/usr/bin/env python3
"""Freeze the bounded authenticated paired-prefill executor surface."""

from __future__ import annotations

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


def main() -> None:
    root = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    executor_path = root / "crates/ferric-engine/src/authenticated_prefill_executor.rs"
    lib_path = root / "crates/ferric-engine/src/lib.rs"
    fixture_path = root / "crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs"
    executor = executor_path.read_text(encoding="utf-8")
    lib = lib_path.read_text(encoding="utf-8")
    fixture = fixture_path.read_text(encoding="utf-8")
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
        "Ok(M1AuthenticatedS1T128PrefillExecutionSuccessV1",
    ]
    cursor = 0
    for needle in ordered:
        position = production.find(needle, cursor)
        if position < 0:
            fail(f"authenticated execution ordering lost {needle}")
        cursor = position + len(needle)

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


if __name__ == "__main__":
    main()
