#!/usr/bin/env python3
"""Freeze the bounded authenticated first-publication S1/K4 bridge surface."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import NoReturn


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def require(source: str, needle: str, label: str) -> int:
    position = source.find(needle)
    if position < 0:
        fail(f"authenticated S1/K4 bridge lost {label}")
    return position


def reject_panicking_production(source: str, label: str) -> None:
    for token in [".expect(", "expect!", "panic!", "unreachable!", "todo!"]:
        if token in source:
            fail(f"{label} contains forbidden production token {token}")


def has_ordered_effective_prefill_wait(source: str) -> bool:
    """Require the deadline-derived timeout to cross the one wait effect."""
    if source.count("published.wait_for(") != 1:
        return False
    if "published.wait_for(queue_wait_timeout.milliseconds())" in source:
        return False
    derivation = (
        "let Some(wait_timeout) = deadline_expired("
        "Boundary::BeforeCompletionWait, queue_wait_timeout)"
    )
    wait = "let completed = match published.wait_for(wait_timeout.milliseconds())"
    ordered = [
        derivation,
        wait,
        "if deadline_expired(Boundary::AfterCompletionWait, queue_wait_timeout).is_none()",
    ]
    tail = source
    for needle in ordered:
        position = tail.find(needle)
        if position < 0:
            return False
        tail = tail[position + len(needle) :]
    derivation_end = source.find(derivation) + len(derivation)
    wait_position = source.find(wait, derivation_end)
    before_wait = source[derivation_end:wait_position]
    exact_rejection = (
        "\n    else {\n"
        "        let closure = published.close_in_flight();\n"
        "        let queue_status = physical_closure_status(&closure);\n"
        "        return Err(terminal_failure(\n"
        "            engine,\n"
        "            M1AuthenticatedS1T128PrefillExecutionStageV1::QueueWait,\n"
        "            M1AuthenticatedS1T128PrefillExecutionErrorV1::DeadlineExpired,\n"
        "            queue_status,\n"
        "            (closure, cache, successor),\n"
        "        ));\n"
        "    };\n    "
    )
    if before_wait != exact_rejection:
        return False
    return True


def main() -> None:
    root = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    readback_path = root / "crates/ferric-engine/src/authenticated_physical_readback.rs"
    choices_path = root / "crates/ferric-engine/src/speculative_diagnostic_choices.rs"
    direct_choices_path = root / "crates/ferric-engine/src/direct_diagnostic_choices.rs"
    lifecycle_path = root / "crates/ferric-engine/src/physical_queue_lifecycle.rs"
    rearm_path = root / "crates/ferric-engine/src/authenticated_queue_rearm.rs"
    queue_path = root / "crates/ferric-engine/src/authenticated_physical_queue.rs"
    prefill_executor_path = root / "crates/ferric-engine/src/authenticated_prefill_executor.rs"
    fixture_path = root / "crates/ferric-engine/src/bin/ferric-m1-qualification-capture.rs"
    readback = readback_path.read_text(encoding="utf-8")
    choices = choices_path.read_text(encoding="utf-8")
    direct_choices = direct_choices_path.read_text(encoding="utf-8")
    lifecycle = lifecycle_path.read_text(encoding="utf-8")
    rearm = rearm_path.read_text(encoding="utf-8")
    queue = queue_path.read_text(encoding="utf-8")
    prefill_executor = prefill_executor_path.read_text(encoding="utf-8")
    fixture = fixture_path.read_text(encoding="utf-8")

    if readback.count("pub fn observe_speculative_k4_diagnostic_choices(") != 1:
        fail("authenticated S1/K4 observation entry point is absent or duplicated")
    impl_position = require(
        readback,
        "impl M1AuthenticatedObservedCompletionOutputV1 {",
        "first-generation observed-owner implementation",
    )
    observe_position = require(
        readback,
        "pub fn observe_speculative_k4_diagnostic_choices(",
        "diagnostic observation transition",
    )
    if impl_position > observe_position:
        fail("diagnostic observation is not owned by first-generation compact custody")
    if "observe_speculative_k4_diagnostic_choices" in rearm or "observe_speculative_k4_diagnostic_choices" in queue:
        fail("diagnostic observation leaked onto queue/rearm typestates")

    generation_guard = require(
        readback,
        "if !is_authenticated_s1_k4_first_dispatch_generation(dispatch_generation)",
        "dispatch-generation-one preflight",
    )
    owner_derivation = require(
        readback[generation_guard:],
        ".speculative_diagnostic_choices()",
        "post-generation retained choice owner",
    ) + generation_guard
    first_copy = require(
        readback,
        "read_authenticated_speculative_k4_choice(&mut self, range_name, range)",
        "first completed choice copy",
    )
    if not generation_guard < owner_derivation < first_copy:
        fail("dispatch generation is not rejected before owner derivation and copying")
    for hostile in ["[0, 2, u64::MAX]", "NotFirstDispatchGeneration { actual: u64 }"]:
        require(readback, hostile, "hostile reused-generation rejection")

    order = require(
        readback,
        '["draft-0", "draft-1", "draft-2", "draft-3"]',
        "exact ordered draft range roster",
    )
    target = require(
        readback[order:],
        'read_authenticated_speculative_k4_choice(&mut self, "target", target_range)',
        "target copy after draft rows",
    )
    require(readback, "let request = lower.completed_read_request(range);", "range-bound request")
    require(readback, "lower.read_completed(request)", "completed-copy transition")
    require(readback, "copies.try_reserve_exact(5)", "bounded five-copy custody")

    require(choices, "draft_data_index: usize", "retained draft data ordinal")
    require(choices, "target_data_index: usize", "retained target data ordinal")
    require(choices, "ReadbackDataIndex { expected: usize, actual: usize }", "data-index rejection")
    require(choices, "readback.data_index()", "completed-readback data-index check")
    require(
        choices,
        "validate_readback_coordinates((31, 7, 64, 16), (31, 8, 64, 16))",
        "hostile data-index test",
    )
    require(direct_choices, "data_index: usize", "retained direct-choice data ordinal")
    require(
        direct_choices,
        "let data_index = allocations.allocation_count();",
        "direct-choice data ordinal derivation",
    )
    require(
        direct_choices,
        "ReadbackDataIndex {",
        "direct-choice data-index rejection",
    )
    require(
        direct_choices,
        "readback.data_index()",
        "completed direct-choice data-index check",
    )
    require(
        direct_choices,
        "readback_generation_data_index_offset_and_extent_are_exact",
        "hostile direct-choice data-index test",
    )
    direct_validation = direct_choices.split("fn validate_readback_coordinates(", 1)[1].split(
        "#[cfg(test)]", 1
    )[0]
    data_index_check = require(
        direct_validation,
        "if actual_data_index != expected_data_index {",
        "direct-choice data-index inequality",
    )
    offset_check = require(
        direct_validation,
        "if actual_offset != expected_offset {",
        "direct-choice offset inequality",
    )
    if data_index_check > offset_check:
        fail("direct-choice data index is not rejected before offset validation")
    require(
        direct_choices,
        "validate_readback_coordinates(0, (7, 31, 128, 4), (7, 32, 128, 4))",
        "hostile direct-choice data-index mutation anchor",
    )

    require(readback, '"partial-non-evidence"', "explicit authority demotion")
    require(
        readback,
        "enum M1AuthenticatedCompletionEvidenceJoinAuthorityV1",
        "private evidence-authority enum",
    )
    if "pub enum M1AuthenticatedCompletionEvidenceJoinAuthorityV1" in readback:
        fail("authenticated diagnostic evidence authority became public")
    require(
        readback,
        "M1AuthenticatedCompletionEvidenceJoinAuthorityV1::SpeculativeDiagnostic",
        "specialized diagnostic semantic join",
    )
    require(
        readback,
        "M1AuthenticatedCompletionEvidenceJoinAuthorityV1::DirectDiagnostic",
        "specialized direct semantic join",
    )
    require(
        readback,
        "M1AuthenticatedCompletionEvidenceJoinAuthorityV1::Generic",
        "unchanged generic semantic join",
    )
    observation_failure_impl = readback.split(
        "impl M1AuthenticatedSpeculativeK4DiagnosticObservationFailureV1 {", 1
    )[1].split(
        "pub struct M1AuthenticatedSpeculativeK4DiagnosticObservationTeardownSuccessV1", 1
    )[0]
    if "pub fn retry" in observation_failure_impl or "pub fn into_parts" in observation_failure_impl:
        fail("diagnostic observation failure exposes retry or compact-owner recovery")
    require(
        observation_failure_impl,
        "destroy_queue_and_retain_evidence",
        "closed observation-failure teardown",
    )
    semantic_failure_impl = readback.split(
        "impl M1AuthenticatedSpeculativeK4DiagnosticCompletedReadbackJoinFailureV1 {", 1
    )[1].split(
        "pub struct M1AuthenticatedSpeculativeK4DiagnosticSemanticTeardownSuccessV1", 1
    )[0]
    if "pub fn into_parts" in semantic_failure_impl or "expectations:" in semantic_failure_impl:
        fail("diagnostic semantic failure exposes its generic owner or caller semantics")
    require(semantic_failure_impl, "pub fn retry(\n        self,", "no-argument semantic retry")
    require(
        semantic_failure_impl,
        "destroy_queue_and_retain_evidence",
        "closed semantic-failure teardown",
    )
    specialized_join = readback.split("fn authenticated_speculative_semantics", 1)[1].split(
        "impl M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1", 1
    )[0]
    if "expectations:" in specialized_join:
        fail("specialized diagnostic join accepts caller-supplied semantics")
    require(specialized_join, "choices.draft_choices_for_lane", "draft-choice-only semantics")
    require(specialized_join, "choices.target_choices_for_lane", "target-choice-only semantics")

    if readback.count("pub fn observe_direct_diagnostic_choices(") != 1:
        fail("authenticated direct observation entry point is absent or duplicated")
    direct_observe = require(
        readback,
        "pub fn observe_direct_diagnostic_choices(",
        "authenticated direct observation transition",
    )
    if impl_position > direct_observe:
        fail("direct observation is not owned by authenticated compact custody")
    if "observe_direct_diagnostic_choices" in rearm or "observe_direct_diagnostic_choices" in queue:
        fail("direct observation leaked onto queue/rearm typestates")
    require(
        lifecycle,
        "pub(crate) fn prepare_m1_direct_diagnostic_ranges_v1(",
        "shared direct range preparation policy",
    )
    require(
        readback,
        "prepare_m1_direct_diagnostic_ranges_v1(",
        "authenticated reuse of direct range policy",
    )
    require(
        readback,
        "case.case.step().target_active_lengths()",
        "queue-retained direct active lengths",
    )
    direct_observation_failure_impl = readback.split(
        "impl M1AuthenticatedDirectDiagnosticObservationFailureV1 {", 1
    )[1].split(
        "pub struct M1AuthenticatedDirectDiagnosticObservationTeardownSuccessV1", 1
    )[0]
    if "pub fn retry" in direct_observation_failure_impl or "pub fn into_parts" in direct_observation_failure_impl:
        fail("direct observation failure exposes retry or compact-owner recovery")
    require(
        direct_observation_failure_impl,
        "destroy_queue_and_retain_evidence",
        "closed direct observation-failure teardown",
    )
    direct_semantic_failure_impl = readback.split(
        "impl M1AuthenticatedDirectDiagnosticCompletedReadbackJoinFailureV1 {", 1
    )[1].split(
        "pub struct M1AuthenticatedDirectDiagnosticSemanticTeardownSuccessV1", 1
    )[0]
    if "pub fn into_parts" in direct_semantic_failure_impl or "expectations:" in direct_semantic_failure_impl:
        fail("direct semantic failure exposes generic authority or caller semantics")
    require(direct_semantic_failure_impl, "pub fn retry(self)", "no-argument direct retry")
    require(
        direct_semantic_failure_impl,
        "destroy_queue_and_retain_evidence",
        "closed direct semantic-failure teardown",
    )
    direct_join = readback.split("fn authenticated_direct_semantics", 1)[1].split(
        "fn authenticated_speculative_semantics", 1
    )[0]
    if "expectations:" in direct_join:
        fail("specialized direct join accepts caller-supplied semantics")
    require(direct_join, "choices.choices()", "direct-choice-only semantics")
    require(
        direct_join,
        "if live > semantics.len()",
        "typed direct choice-capacity rejection",
    )
    if "debug_assert" in direct_join:
        fail("direct semantic capacity relies on a debug-only assertion")
    require(
        readback,
        "fn private_direct_authority_requires_exactly_one_direct_attachment()",
        "exact direct attachment regression",
    )
    require(
        readback,
        "fn generic_readback_denies_diagnostic_capture_routes()",
        "generic direct-attachment rejection regression",
    )
    require(
        readback,
        "fn oversized_direct_choice_custody_fails_before_semantic_slice()",
        "hostile oversized-choice regression",
    )

    direct_observation = readback[direct_observe:observe_position]
    direct_helpers = readback.split(
        "fn prepare_authenticated_direct_diagnostic_ranges", 1
    )[1].split("type AuthenticatedSpeculativeDiagnosticInputsV1", 1)[0]
    direct_custody = readback.split(
        "pub struct M1AuthenticatedObservedDirectDiagnosticOutputV1", 1
    )[1].split("/// First-publication authenticated S1/K4", 1)[0]
    shared_ranges = lifecycle.split(
        "pub(crate) fn prepare_m1_direct_diagnostic_ranges_v1(", 1
    )[1].split("/// One exact recycled queue generation", 1)[0]
    for label, production in [
        ("authenticated direct observation", direct_observation),
        ("authenticated direct helpers", direct_helpers),
        ("authenticated direct custody", direct_custody),
        ("authenticated direct semantic join", direct_join),
        ("shared direct range preparation", shared_ranges),
    ]:
        reject_panicking_production(production, label)

    if "FERRIC_M1_ROLLOVER_PREFILL_TOKEN" in fixture:
        fail("MI300X rollover fixture still accepts an external prefill-token oracle")
    deadline_helper = (
        "pub(crate) fn "
        "execute_m1_authenticated_s1_t128_paired_prefill_with_deadline_v1"
    )
    prefill_production = prefill_executor.split("#[cfg(test)]", 1)[0]
    if prefill_production.count(deadline_helper) != 1:
        fail("authenticated S1/K4 bridge lost unique deadline-aware prefill executor")
    deadline_execution = prefill_production.split(deadline_helper, 1)[1]
    if not has_ordered_effective_prefill_wait(deadline_execution):
        fail("authenticated S1/K4 bridge lost bounded effective prefill wait")

    effective_wait = "published.wait_for(wait_timeout.milliseconds())"
    old_configured_wait = "published.wait_for(queue_wait_timeout.milliseconds())"
    hostile_configured = deadline_execution.replace(
        effective_wait, old_configured_wait, 1
    )
    if has_ordered_effective_prefill_wait(hostile_configured):
        fail("bounded prefill checker accepts configured-timeout substitution")
    hostile_substitute = deadline_execution.replace(
        "Boundary::BeforeCompletionWait, queue_wait_timeout)",
        "Boundary::BeforeCompletionWait, "
        "M1QueueWaitTimeoutV1::new(1).unwrap_or(queue_wait_timeout))",
        1,
    )
    if has_ordered_effective_prefill_wait(hostile_substitute):
        fail("bounded prefill checker accepts substituted deadline input")
    hostile_wait_shadow = deadline_execution.replace(
        "let completed = match published.wait_for(wait_timeout.milliseconds())",
        "let wait_timeout = queue_wait_timeout;\n    "
        "let completed = match published.wait_for(wait_timeout.milliseconds())",
        1,
    )
    if has_ordered_effective_prefill_wait(hostile_wait_shadow):
        fail("bounded prefill checker accepts effective-timeout shadowing")
    hostile_configured_shadow = deadline_execution.replace(
        "let completed = match published.wait_for(wait_timeout.milliseconds())",
        "let queue_wait_timeout = wait_timeout;\n    "
        "let completed = match published.wait_for(wait_timeout.milliseconds())",
        1,
    )
    if has_ordered_effective_prefill_wait(hostile_configured_shadow):
        fail("bounded prefill checker accepts configured-timeout shadowing")
    hostile_match_shadow = deadline_execution.replace(
        "let completed = match published.wait_for(wait_timeout.milliseconds())",
        "match queue_wait_timeout {\n"
        "        wait_timeout => {\n"
        "            let completed = match published.wait_for(wait_timeout.milliseconds())",
        1,
    )
    match_body, match_close = hostile_match_shadow.rsplit("\n}", 1)
    hostile_match_shadow = match_body + "\n        }\n    }\n}" + match_close
    if has_ordered_effective_prefill_wait(hostile_match_shadow):
        fail("bounded prefill checker accepts match-pattern timeout shadowing")
    hostile_closure_shadow = deadline_execution.replace(
        "let completed = match published.wait_for(wait_timeout.milliseconds())",
        "(|wait_timeout| {\n"
        "        let completed = match published.wait_for(wait_timeout.milliseconds())",
        1,
    )
    closure_body, closure_close = hostile_closure_shadow.rsplit("\n}", 1)
    hostile_closure_shadow = (
        closure_body + "\n    })(queue_wait_timeout)\n}" + closure_close
    )
    if has_ordered_effective_prefill_wait(hostile_closure_shadow):
        fail("bounded prefill checker accepts closure-parameter timeout shadowing")
    hostile_reordered = deadline_execution.replace(
        "Boundary::BeforeCompletionWait", "Boundary::DeadlineSwap", 1
    ).replace("Boundary::AfterCompletionWait", "Boundary::BeforeCompletionWait", 1)
    hostile_reordered = hostile_reordered.replace(
        "Boundary::DeadlineSwap", "Boundary::AfterCompletionWait", 1
    )
    if has_ordered_effective_prefill_wait(hostile_reordered):
        fail("bounded prefill checker accepts reordered deadline boundaries")
    hostile_missing_post = deadline_execution.replace(
        "Boundary::AfterCompletionWait", "Boundary::BeforeCompletionWait", 1
    )
    if has_ordered_effective_prefill_wait(hostile_missing_post):
        fail("bounded prefill checker accepts a missing post-wait deadline boundary")

    require(
        prefill_executor,
        "observed.observe_direct_diagnostic_choices()",
        "authenticated direct prefill observation",
    )
    require(prefill_executor, "direct.check_completion()", "evidence-authorized direct join")
    require(
        prefill_executor,
        "let [choice] = choices.choices() else",
        "checked direct-choice anchor derivation",
    )
    fixture_execution = fixture.split(
        "fn admitted_mi300x_runs_public_authenticated_rollover_executor()", 1
    )[1].split("#[test]", 1)[0]
    require(
        fixture_execution,
        "execute_m1_authenticated_s1_t128_paired_prefill_v1(",
        "MI300X production prefill executor",
    )
    if "observed.observe_direct_diagnostic_choices()" in fixture_execution:
        fail("MI300X fixture duplicates authenticated direct observation")

    require(
        rearm,
        "const fn diagnostic_capture_is_supported(direct: bool, _speculative: bool) -> bool {\n    !direct\n}",
        "authenticated rearm reset restriction",
    )
    require(
        rearm,
        "authenticated_rearm_rejects_direct_and_preserves_speculative_capture",
        "authenticated rearm hostile test",
    )

    print(
        "PASS: authenticated first-publication S1/K4 diagnostic bridge remains "
        "bounded, one-copy, data-index checked, direct-join authenticated, "
        "partial-non-evidence, and excluded from rearm"
    )


if __name__ == "__main__":
    main()
