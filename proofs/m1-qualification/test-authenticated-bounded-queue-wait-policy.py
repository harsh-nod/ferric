#!/usr/bin/env python3
"""Freeze Ferric's authenticated delegation to fe2o3's bounded queue wait."""

from __future__ import annotations

import sys
from pathlib import Path


class PolicyError(RuntimeError):
    """One required bounded-wait source invariant was absent."""


def require(source: str, needle: str, label: str) -> int:
    position = source.find(needle)
    if position < 0:
        raise PolicyError(f"authenticated bounded queue wait lost {label}")
    return position


def function(source: str, signature: str, label: str) -> str:
    if source.count(signature) != 1:
        raise PolicyError(f"authenticated bounded queue wait has non-unique {label}")
    start = source.index(signature)
    opening = source.find("{", start)
    if opening < 0:
        raise PolicyError(f"authenticated bounded queue wait lost {label} body")
    depth = 0
    for offset in range(opening, len(source)):
        token = source[offset]
        if token == "{":
            depth += 1
        elif token == "}":
            depth -= 1
            if depth == 0:
                return source[start : offset + 1]
    raise PolicyError(f"authenticated bounded queue wait has unterminated {label}")


def validate(queue: str, rearm: str, other_engine_sources: tuple[str, ...]) -> None:
    legacy_case = function(queue, "fn wait_case<const N: usize>(", "legacy wait case")
    require(
        legacy_case,
        "wait_with_completion_progress_policy::<N, _, _, _>(",
        "legacy completion-progress diagnostic",
    )
    require(legacy_case, "published.wait(0)", "legacy terminal zero-scan wait")

    bounded_case = function(queue, "fn wait_for_case<const N: usize>(", "bounded wait case")
    require(
        bounded_case,
        "match lower.wait_for(timeout_ms)",
        "exact authenticated fe2o3 deadline delegation",
    )
    require(bounded_case, "operation_failure(", "terminal owner quarantine")
    if "wait_with_completion_progress_policy" in bounded_case or ".wait(0)" in bounded_case:
        raise PolicyError("bounded deadline wait was redirected through legacy diagnostics")

    bounded_public = function(queue, "pub fn wait_for(\n", "physical wait_for facade")
    if bounded_public.count("wait_for_variant(") != 5:
        raise PolicyError("physical wait_for facade does not close all five fixed shapes")
    require(
        bounded_public,
        "timeout_ms: u32",
        "u32 millisecond timeout contract",
    )

    timeout = function(
        queue,
        "pub fn timeout_observation(&self)",
        "physical timeout observation",
    )
    require(
        timeout,
        "self.lower.timeout_observation()",
        "lower addressless timeout observation forwarding",
    )

    legacy_rearm = function(rearm, "pub fn wait<const C: usize>(", "legacy rearm wait")
    require(legacy_rearm, "match queue.wait()", "legacy rearm diagnostic wait")

    bounded_rearm = function(
        rearm,
        "pub fn wait_for<const C: usize>(",
        "bounded rearm wait",
    )
    deadline = require(
        bounded_rearm,
        "match queue.wait_for(timeout_ms)",
        "rearm deadline delegation",
    )
    fault = require(
        bounded_rearm,
        "engine.quarantine_m1_queue_rearm_failure();",
        "permanent Engine fault",
    )
    retained = require(
        bounded_rearm,
        "M1AuthenticatedRearmedQueueProgressFailureV1 {",
        "rearm continuation quarantine",
    )
    if not deadline < fault < retained:
        raise PolicyError("rearm lower failure is not faulted before custody publication")
    for needle, label in [
        ("phase: M1LongLivedQueueRearmProgressPhaseV1::QueueWait", "wait phase"),
        ("source,", "lower failure owner"),
        ("carry,", "continuation custody"),
        ("queue_observation,", "queue observation"),
        ("device,", "device receipt"),
    ]:
        require(bounded_rearm[retained:], needle, label)

    rearm_timeout = function(
        rearm,
        "pub fn timeout_observation(&self)",
        "rearm timeout observation",
    )
    require(
        rearm_timeout,
        "self.source.timeout_observation()",
        "rearm timeout observation forwarding",
    )

    if any(".wait_for(" in source for source in other_engine_sources):
        raise PolicyError("bounded wait was wired into a broader production callsite")


def expect_rejected(
    label: str,
    queue: str,
    rearm: str,
    other_engine_sources: tuple[str, ...],
) -> None:
    try:
        validate(queue, rearm, other_engine_sources)
    except PolicyError:
        return
    raise PolicyError(f"hostile {label} mutation was accepted")


def main() -> None:
    root = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    source_root = root / "crates/ferric-engine/src"
    queue_path = source_root / "authenticated_physical_queue.rs"
    rearm_path = source_root / "authenticated_queue_rearm.rs"
    queue = queue_path.read_text(encoding="utf-8")
    rearm = rearm_path.read_text(encoding="utf-8")
    other_engine_sources = tuple(
        path.read_text(encoding="utf-8")
        for path in sorted(source_root.rglob("*.rs"))
        if path not in {queue_path, rearm_path}
    )

    validate(queue, rearm, other_engine_sources)

    expect_rejected(
        "poll-count substitution",
        queue.replace(
            "match lower.wait_for(timeout_ms)",
            "match lower.wait(0)",
            1,
        ),
        rearm,
        other_engine_sources,
    )
    expect_rejected(
        "timeout-observation erasure",
        queue.replace("self.lower.timeout_observation()", "None", 1),
        rearm,
        other_engine_sources,
    )
    bounded_rearm = function(rearm, "pub fn wait_for<const C: usize>(", "bounded rearm wait")
    expect_rejected(
        "Engine-fault erasure",
        queue,
        rearm.replace(
            bounded_rearm,
            bounded_rearm.replace(
                "engine.quarantine_m1_queue_rearm_failure();\n",
                "",
                1,
            ),
            1,
        ),
        other_engine_sources,
    )
    expect_rejected(
        "broader-callsite wiring",
        queue,
        rearm,
        other_engine_sources + ("published.wait_for(timeout_ms);",),
    )

    print(
        "PASS: authenticated bounded queue wait delegates to fe2o3's monotonic "
        "deadline, forwards timeout observations, quarantines every owner, faults "
        "the rearm Engine, preserves legacy diagnostics, and rejects 4 hostile mutations"
    )


if __name__ == "__main__":
    try:
        main()
    except PolicyError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1) from error
