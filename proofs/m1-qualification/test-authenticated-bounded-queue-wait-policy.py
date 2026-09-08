#!/usr/bin/env python3
"""Freeze bounded production waits without weakening progress or custody."""

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


def validate(sources: dict[str, str]) -> None:
    queue = sources["queue"]
    rearm = sources["rearm"]
    raw_rearm = sources["raw_rearm"]
    lifecycle = sources["lifecycle"]
    speculative = sources["speculative"]
    serving = sources["serving"]
    rollover = sources["rollover"]
    r33 = sources["r33"]

    timeout_type = require(queue, "pub struct M1QueueWaitTimeoutV1(NonZeroU32);", "opaque timeout")
    require(queue[timeout_type:], "pub const fn new(milliseconds: u32) -> Option<Self>", "zero-rejecting timeout constructor")

    legacy_case = function(queue, "fn wait_case<const N: usize>(", "legacy wait case")
    require(legacy_case, "wait_with_completion_progress_policy::<N, _, _, _>(", "legacy progress policy")
    require(legacy_case, "published.wait(0)", "legacy terminalizer")

    bounded_case = function(queue, "fn wait_for_case<const N: usize>(", "bounded wait case")
    require(bounded_case, "wait_with_completion_progress_deadline_policy::<N, _, _, _>(", "combined policy")
    require(bounded_case, "published.poll_with_progress()", "bounded progress polling")
    require(bounded_case, "|published| published.wait_for(0)", "KFD race-aware terminalizer")
    if "lower.wait_for(timeout_ms)" in bounded_case:
        raise PolicyError("bounded wait bypassed Ferric's progress policy")

    raw_bounded_case = function(lifecycle, "fn wait_for_case<const N: usize>(", "raw bounded wait case")
    require(raw_bounded_case, "wait_with_completion_progress_deadline_policy::<N, _, _, _>(", "raw combined policy")
    require(raw_bounded_case, "published.poll_with_progress()", "raw bounded progress polling")
    require(raw_bounded_case, "|published| published.wait_for(0)", "raw KFD race-aware terminalizer")

    deadline_helper = function(lifecycle, "pub(crate) fn wait_with_completion_progress_deadline_policy<", "deadline helper")
    require(deadline_helper, "std::time::Instant::now()", "monotonic origin")
    require(deadline_helper, "started.elapsed() >= timeout", "absolute wall deadline")
    require(deadline_helper, "terminalize: impl FnOnce(P) -> Result<C, E>", "completion race result")
    core = function(lifecycle, "fn wait_with_completion_progress_policy_core<", "combined core")
    require(core, "ConsecutiveScansWithoutProgress", "stalled-progress bound")
    require(core, "WallClockDeadlineReached", "wall-clock bound")
    terminalize = function(lifecycle, "fn terminalize_completion_progress_policy<", "race-aware terminalizer")
    require(terminalize, "Ok(completed) => Ok(completed)", "terminalizer completion race")

    initial_native = speculative[
        require(speculative, "impl M1InitialQueueEffectsV1 for M1NativeInitialQueueEffectsV1", "native initial effects") :
        require(speculative, "fn execute_initial_round_core<", "initial core")
    ]
    require(initial_native, "published.wait_for(timeout.milliseconds())", "bounded initial production wait")
    rearm_native = speculative[
        require(speculative, "impl M1RearmedQueueEffectsV1 for M1NativeRearmedQueueEffectsV1", "native rearm effects") :
        require(speculative, "fn complete_round_core<", "rearm core")
    ]
    require(rearm_native, "published.wait_for(timeout.milliseconds(), engine)", "bounded rearm production wait")

    executor = function(speculative, "pub struct M1AuthenticatedSpeculativePhysicalExecutorV1", "executor custody")
    require(executor, "queue_wait_timeout: crate::M1QueueWaitTimeoutV1,", "executor timeout custody")
    rollover_published = function(speculative, "pub struct M1AuthenticatedSpeculativeRolloverPublishedV1", "rollover custody")
    require(rollover_published, "queue_wait_timeout: crate::M1QueueWaitTimeoutV1,", "rollover timeout custody")
    bootstrap_retries = function(speculative, "enum M1AuthenticatedSpeculativeBootstrapPreDetachRetryStateV1", "bootstrap retry custody")
    if bootstrap_retries.count("queue_wait_timeout: crate::M1QueueWaitTimeoutV1,") != 3:
        raise PolicyError("timeout is not retained across all three bootstrap retry states")
    rollover_completion = function(speculative, "pub fn complete_round<const C: usize>(", "rollover completion")
    signature = rollover_completion[: rollover_completion.find(") ->")]
    if "queue_wait_timeout:" in signature:
        raise PolicyError("rollover completion permits post-publication timeout substitution")
    require(
        rollover_completion,
        "self.complete_round_with_deadline(engine, controls, |_, timeout| Some(timeout))",
        "deadline-aware rollover completion delegation",
    )
    deadline_rollover_completion = function(
        speculative,
        "pub(crate) fn complete_round_with_deadline<const C: usize, D>(",
        "deadline-aware rollover completion",
    )
    if deadline_rollover_completion.count("queue_wait_timeout,") != 3:
        raise PolicyError("deadline-aware rollover completion lost retained timeout custody")
    require(
        deadline_rollover_completion,
        "deadline_expired(Boundary::BeforeSettlement, queue_wait_timeout)",
        "retained pre-settlement timeout consumption",
    )
    require(
        deadline_rollover_completion,
        "deadline_expired(Boundary::AfterSettlement, queue_wait_timeout)",
        "retained post-settlement timeout consumption",
    )

    submit_rollover = function(rollover, "pub fn submit_m1_authenticated_speculative_rollover_v1<const C: usize>(", "rollover submit")
    require(submit_rollover, "queue_wait_timeout: crate::M1QueueWaitTimeoutV1", "prepublication rollover timeout")

    read_published = function(serving, "    fn read_published(\n", "serving readback")
    if read_published.count(".wait_for(") != 2 or ".wait()" in read_published:
        raise PolicyError("serving initial and rearm waits are not both bounded")
    require(serving, "queue_wait_timeout: crate::M1QueueWaitTimeoutV1,", "serving adapter timeout custody")
    require(r33, "queue_wait_timeout: M1QueueWaitTimeoutV1,", "R33 bind timeout")

    bounded_rearm = function(rearm, "pub fn wait_for<const C: usize>(", "bounded rearm wait")
    deadline = require(bounded_rearm, "match queue.wait_for(timeout_ms)", "rearm deadline delegation")
    fault = require(bounded_rearm, "engine.quarantine_m1_queue_rearm_failure();", "permanent Engine fault")
    retained = require(bounded_rearm, "M1AuthenticatedRearmedQueueProgressFailureV1 {", "rearm custody")
    if not deadline < fault < retained:
        raise PolicyError("rearm wait does not fault before returning terminal custody")
    require(rearm, "self.source.timeout_observation()", "timeout observation forwarding")

    bounded_raw_rearm = function(raw_rearm, "pub fn wait_for<const C: usize>(", "bounded raw rearm wait")
    raw_deadline = require(bounded_raw_rearm, "match queue.wait_for(timeout_ms)", "raw rearm deadline delegation")
    raw_fault = require(bounded_raw_rearm, "engine.quarantine_m1_queue_rearm_failure();", "raw rearm Engine fault")
    raw_retained = require(bounded_raw_rearm, "M1RearmedQueueProgressFailureV1 {", "raw rearm custody")
    if not raw_deadline < raw_fault < raw_retained:
        raise PolicyError("raw rearm wait does not fault before returning terminal custody")
    require(raw_rearm, "self.source.timeout_execution_observation()", "raw timeout observation forwarding")


def expect_rejected(label: str, sources: dict[str, str]) -> None:
    try:
        validate(sources)
    except PolicyError:
        return
    raise PolicyError(f"hostile {label} mutation was accepted")


def mutated(sources: dict[str, str], name: str, old: str, new: str) -> dict[str, str]:
    changed = dict(sources)
    changed[name] = changed[name].replace(old, new, 1)
    return changed


def main() -> None:
    root = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    engine = root / "crates/ferric-engine/src"
    paths = {
        "queue": engine / "authenticated_physical_queue.rs",
        "rearm": engine / "authenticated_queue_rearm.rs",
        "raw_rearm": engine / "m1_queue_rearm.rs",
        "lifecycle": engine / "physical_queue_lifecycle.rs",
        "speculative": engine / "authenticated_speculative_executor.rs",
        "serving": engine / "m1_serving_physical_operations.rs",
        "rollover": engine / "authenticated_queue_rollover.rs",
        "r33": root / "adapters/m1-engineering-execution-v1/src/r33_lifecycle.rs",
    }
    sources = {name: path.read_text(encoding="utf-8") for name, path in paths.items()}
    validate(sources)

    for label, name, old, new in [
        ("progress-policy bypass", "queue", "wait_with_completion_progress_deadline_policy::<N, _, _, _>(", "lower.wait_for("),
        ("KFD terminalizer erasure", "queue", "|published| published.wait_for(0)", "|published| published.wait(0)"),
        ("completion-race panic", "lifecycle", "Ok(completed) => Ok(completed)", "Ok(_) => unreachable!()"),
        ("initial unbounded wait", "speculative", "published.wait_for(timeout.milliseconds())", "published.wait()"),
        ("executor timeout erasure", "speculative", "pub struct M1AuthenticatedSpeculativePhysicalExecutorV1 {\n    coordinator: M1SpeculativeGenerationLoopV1,\n    released: M1AuthenticatedLongLivedQueueReleasedRoundV1,\n    lineage: M1AuthenticatedSpeculativeCausalLineageV1,\n    queue_wait_timeout: crate::M1QueueWaitTimeoutV1,", "pub struct M1AuthenticatedSpeculativePhysicalExecutorV1 {\n    coordinator: M1SpeculativeGenerationLoopV1,\n    released: M1AuthenticatedLongLivedQueueReleasedRoundV1,\n    lineage: M1AuthenticatedSpeculativeCausalLineageV1,"),
        ("serving unbounded wait", "serving", ".wait_for(self.queue_wait_timeout.milliseconds())", ".wait()"),
        ("serving unbounded raw rearm", "serving", "published.wait_for(self.queue_wait_timeout.milliseconds(), self.engine)", "published.wait(self.engine)"),
        ("R33 timeout erasure", "r33", "queue_wait_timeout: M1QueueWaitTimeoutV1,", ""),
    ]:
        expect_rejected(label, mutated(sources, name, old, new))

    rearm_wait = function(sources["rearm"], "pub fn wait_for<const C: usize>(", "bounded rearm wait")
    unfaulted = rearm_wait.replace("engine.quarantine_m1_queue_rearm_failure();", "", 1)
    changed = dict(sources)
    changed["rearm"] = changed["rearm"].replace(rearm_wait, unfaulted, 1)
    expect_rejected("rearm Engine fault erasure", changed)

    raw_rearm_wait = function(sources["raw_rearm"], "pub fn wait_for<const C: usize>(", "bounded raw rearm wait")
    raw_unfaulted = raw_rearm_wait.replace("engine.quarantine_m1_queue_rearm_failure();", "", 1)
    changed = dict(sources)
    changed["raw_rearm"] = changed["raw_rearm"].replace(raw_rearm_wait, raw_unfaulted, 1)
    expect_rejected("raw rearm Engine fault erasure", changed)

    completion = function(sources["speculative"], "pub fn complete_round<const C: usize>(", "rollover completion")
    substituted = completion.replace(
        "engine: &mut Engine<C>,",
        "engine: &mut Engine<C>,\n        queue_wait_timeout: crate::M1QueueWaitTimeoutV1,",
        1,
    )
    changed = dict(sources)
    changed["speculative"] = changed["speculative"].replace(completion, substituted, 1)
    expect_rejected("post-publication rollover timeout substitution", changed)

    deadline_completion = function(
        sources["speculative"],
        "pub(crate) fn complete_round_with_deadline<const C: usize, D>(",
        "deadline-aware rollover completion",
    )
    erased_timeout = deadline_completion.replace("queue_wait_timeout,", "", 1)
    changed = dict(sources)
    changed["speculative"] = changed["speculative"].replace(
        deadline_completion, erased_timeout, 1
    )
    expect_rejected("retained rollover timeout erasure", changed)

    print(
        "PASS: authenticated production waits combine progress and monotonic wall bounds, "
        "terminalize through race-aware KFD wait_for(0), retain timeout custody, fault rearm "
        "Engine state, and reject 12 hostile mutations"
    )


if __name__ == "__main__":
    try:
        main()
    except PolicyError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1) from error
