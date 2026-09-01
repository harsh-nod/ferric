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


def main() -> None:
    root = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    readback_path = root / "crates/ferric-engine/src/authenticated_physical_readback.rs"
    choices_path = root / "crates/ferric-engine/src/speculative_diagnostic_choices.rs"
    rearm_path = root / "crates/ferric-engine/src/authenticated_queue_rearm.rs"
    queue_path = root / "crates/ferric-engine/src/authenticated_physical_queue.rs"
    readback = readback_path.read_text(encoding="utf-8")
    choices = choices_path.read_text(encoding="utf-8")
    rearm = rearm_path.read_text(encoding="utf-8")
    queue = queue_path.read_text(encoding="utf-8")

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
        "M1AuthenticatedCompletionEvidenceJoinAuthorityV1::SpeculativeK4Diagnostic",
        "specialized diagnostic semantic join",
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
    specialized_join = readback.split("fn authenticated_speculative_k4_semantics", 1)[1].split(
        "impl M1AuthenticatedObservedSpeculativeK4DiagnosticOutputV1", 1
    )[0]
    if "expectations:" in specialized_join:
        fail("specialized diagnostic join accepts caller-supplied semantics")
    require(specialized_join, "choices.draft_choices_for_lane", "draft-choice-only semantics")
    require(specialized_join, "choices.target_choices_for_lane", "target-choice-only semantics")

    require(
        rearm,
        "const fn diagnostic_capture_is_supported(direct: bool, speculative: bool) -> bool {\n    !direct && !speculative\n}",
        "authenticated rearm reset restriction",
    )
    require(
        rearm,
        "generic_authenticated_rearm_rejects_diagnostic_capture_until_reset_exists",
        "authenticated rearm hostile test",
    )

    print(
        "PASS: authenticated first-publication S1/K4 diagnostic bridge remains "
        "bounded, one-copy, data-index checked, partial-non-evidence, and excluded from rearm"
    )


if __name__ == "__main__":
    main()
