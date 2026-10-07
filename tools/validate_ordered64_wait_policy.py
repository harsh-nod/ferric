#!/usr/bin/env python3
"""Check ordered64 wait-policy evidence, not native or performance admission.

The caller must independently validate the transcript, whole-process packet
schedule, running binaries, placement, parity and supervision. In particular,
the context is not a receipt this module can authenticate on its own.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat


U64 = (1 << 64) - 1
LIMIT = 128 * 1024
CONTEXT_LIMIT = 1024 * 1024
RUNTIME_SCHEMA = "FerricOrdered64RuntimeCountersV1"
TERMINAL_SCHEMA = "Fe2o3Ordered64WaitPolicyDiagnosticV1"
CONTROL_PROFILE = "prefill16-decode-ordered64-runtime-counters-v1"
ACTIVE_PROFILE = "prefill16-decode-ordered64-active-poll-10ms-v1"
COUNTERS = (
    "commands", "command_ns", "full_currentness_checks", "full_currentness_ns",
    "operational_currentness_checks", "operational_currentness_ns",
    "kernel_admissions", "kernel_admission_ns", "dispatches", "dispatch_prepare_ns",
    "dispatch_publish_ns", "dispatch_wait_ns", "completion_polls", "reads",
    "read_bytes", "read_ns", "writes", "write_bytes", "write_ns",
)
POLICY_COUNTERS = (
    "completed_batches", "spin_pauses", "fallback_sleeps",
    "completed_without_fallback", "completed_after_fallback",
)
WAIT_POLICY = {
    "policy": "ActivePoll10msV1",
    "worker_entry": "--diagnostic-active-poll-10ms",
    "active_window_ns": 10_000_000,
    "fallback_sleep_ns": 50_000,
    "scope": "ordinary ordered64 only; unchanged validation and original deadline",
    "terminal_schema": TERMINAL_SCHEMA,
    "terminal_stream": "worker stderr; independent native harness must bind PID/device/counts",
    "cpu_scope": "single worker thread may actively poll for up to 10ms per group; no aggregate CPU-time claim",
    "performance_qualified": False,
}
METADATA = {
    "runtime_profiling": True,
    "runtime_counter_schema": RUNTIME_SCHEMA,
    "runtime_counter_scope": "two cumulative worker host-wall snapshots around the complete workload; scopes overlap and snapshot boundaries differ; not GPU timestamps or isolated IOCTL time",
    "host_timing_schema": "FerricOrdered64HostTimingV1",
    "diagnostic_max_model_batches": 256,
    "host_diagnostic_scope": "overlapping controller and worker wall durations; not GPU time; instrumentation perturbs execution",
}
RECORD_HEADER = {
    **METADATA,
    "schema": RUNTIME_SCHEMA,
    "authority": "none",
    "performance_qualified": False,
    "benchmark_admitted": False,
    "serving_admitted": False,
    "wave_target_mode": "combined",
    "measurement": "cumulative overlapping worker host-wall counters; not GPU timestamps",
    "command_accounting": "commands and command_ns deltas include the earlier snapshot command and exclude the final snapshot command; currentness deltas exclude the earlier snapshot check_idle and include the final snapshot check_idle",
    "workload_scope": "all live requests between snapshots, including any warmup; not a benchmark measurement",
    "dispatch_prepare_scope": "pure preparation only; excludes aggregate fences, payload copy and staging",
}
CONTEXT_KEYS = {
    "schema", "active_poll", "controller_pid", "worker_pid", "device_unique_id",
    "controller_sha256", "worker_sha256", "expected_ordered_groups",
    "expected_dispatches", "setup", "closed", "cleanup",
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def exact(actual, expected, message):
    require(type(actual) is type(expected), message)
    if type(expected) is dict:
        require(actual.keys() == expected.keys(), message)
        for key in expected:
            exact(actual[key], expected[key], message + ": " + key)
    elif type(expected) is list:
        require(len(actual) == len(expected), message)
        for left, right in zip(actual, expected):
            exact(left, right, message)
    else:
        require(actual == expected, message)


def fields(actual, expected, message):
    require(type(actual) is dict, message)
    for key, value in expected.items():
        require(key in actual, message + ": missing " + key)
        exact(actual[key], value, message + ": " + key)


def integer(value, label, maximum=U64):
    require(type(value) is int and 0 <= value <= maximum, "bounded integer: " + label)
    return value


def digest(value):
    require(type(value) is str and re.fullmatch(r"[0-9a-f]{64}", value) is not None,
            "SHA256 must be 64 lowercase hexadecimal characters")
    return value


def decode(raw):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON key: " + key)
            result[key] = value
        return result

    return json.loads(raw.decode("utf-8"), object_pairs_hook=unique,
                      parse_constant=lambda _: require(False, "nonfinite JSON"))


def metadata(value, active):
    fields(value, {**METADATA, "live_profile": ACTIVE_PROFILE if active else CONTROL_PROFILE},
           "wait-policy profile")
    if active:
        require("ordered64_wait_policy" in value, "missing active policy annotation")
        exact(value["ordered64_wait_policy"], WAIT_POLICY, "active policy annotation")
    else:
        require("ordered64_wait_policy" not in value, "active annotation in default arm")


def validate_context(context):
    require(type(context) is dict and set(context) == CONTEXT_KEYS, "closed policy context")
    exact(context["schema"], "FerricOrdered64WaitPolicyContextV1", "context schema")
    require(type(context["active_poll"]) is bool, "explicit boolean policy selector")
    for key in ("controller_pid", "worker_pid"):
        require(integer(context[key], key, 0xffffffff) > 0, "positive process ID")
    require(context["controller_pid"] != context["worker_pid"], "distinct controller and worker")
    integer(context["device_unique_id"], "device unique ID")
    groups = integer(context["expected_ordered_groups"], "whole-process ordered groups")
    dispatches = integer(context["expected_dispatches"], "whole-process dispatches")
    require(0 < groups <= dispatches, "nonempty independently checked packet schedule")
    for key in ("controller_sha256", "worker_sha256"):
        digest(context[key])

    # A terminal record can precede a failed flush/exit, and successful EOF need
    # not acknowledge Close. Require both the controller close and owned reap.
    fields(context["cleanup"], {
        "owned_pgid": context["controller_pid"], "returncode": 0,
        "owned_group_absent": True, "child_reaped": True, "cleanup_ok": True,
        "term_sent": False, "kill_sent": False, "errors": [],
    }, "unsignaled successful controller reap")
    setup, closed = context["setup"], context["closed"]
    active = context["active_poll"]
    for value in (setup, closed):
        metadata(value, active)
        fields(value, {
            "worker_pids": [context["worker_pid"]], "performance_qualified": False,
            "benchmark_admitted": False, "serving_admitted": False,
        }, "same diagnostic worker lifecycle")
    fields(setup, {
        "schema": "FerricQwen3TpBatchSetupV2", "authority": "none",
        "tensor_parallel": 1, "device_unique_ids": [context["device_unique_id"]],
        "serving_qualified": False,
        "controller_sha256": context["controller_sha256"],
        "worker_sha256": context["worker_sha256"],
        "running_worker_sha256": [context["worker_sha256"]],
    }, "setup identity")
    metadata(setup.get("performance_profile"), active)
    fields(closed, {
        "schema": "FerricQwen3TpBatchClosedV2", "authority": "none",
        "execution_completed": True, "all_workers_exited": True,
        "rank_dispatch_counts": [dispatches],
    }, "explicit successful close")


def validate_snapshots(records, context):
    counters = []
    active = context["active_poll"]
    expected_keys = set(RECORD_HEADER) | {
        "live_profile", "phase", "snapshots", "counter_delta", "placement",
    }
    if active:
        expected_keys.add("ordered64_wait_policy")
    for ordinal, phase in enumerate(("before_workload", "after_workload")):
        record = records[ordinal]
        require(type(record) is dict and set(record) == expected_keys, "closed runtime record")
        fields(record, {**RECORD_HEADER, "phase": phase}, "runtime record semantics")
        metadata(record, active)
        require(type(record["placement"]) is dict, "placement object for caller validation")
        snapshots = record["snapshots"]
        require(type(snapshots) is list and len(snapshots) == 1, "one TP1 snapshot")
        expected = {
            "schema": "FerricRuntimeDiagnosticSnapshotV1", "authority": "none",
            "performance_qualified": False,
            "scope": "cumulative overlapping worker host-wall counters, not GPU timestamps",
            "process_id": context["worker_pid"], "device_unique_id": context["device_unique_id"],
            "rank": 0, "ordinal": ordinal,
        }
        snapshot = snapshots[0]
        require(type(snapshot) is dict and set(snapshot) == set(expected) | {"counters"},
                "closed runtime snapshot")
        fields(snapshot, expected, "snapshot identity")
        current = snapshot["counters"]
        require(type(current) is dict and set(current) == set(COUNTERS), "nineteen runtime counters")
        for key, value in current.items():
            integer(value, key)
        exact(current["dispatches"], 0 if ordinal == 0 else context["expected_dispatches"],
              "snapshots enclose all process dispatches")
        delta = {} if ordinal == 0 else {key: current[key] - counters[0][key] for key in COUNTERS}
        for key, value in delta.items():
            integer(value, "delta " + key)
        exact(record["counter_delta"], delta, "explicit cumulative counter delta")
        counters.append(current)
    require(delta["commands"] > 0 and delta["command_ns"] > 0, "nonempty runtime interval")
    return delta


def validate_terminal(record, context, delta):
    expected = {
        "schema": TERMINAL_SCHEMA, "policy": "ActivePoll10msV1",
        "worker_pid": context["worker_pid"], "device_unique_id": context["device_unique_id"],
        "scope": "successful ordered64 batches in this worker process",
        "active_window_ns": 10_000_000, "fallback_sleep_ns": 50_000,
        "closed_cleanly": True,
    }
    require(type(record) is dict and set(record) == set(expected) | {"counters"},
            "closed terminal policy record")
    fields(record, expected, "terminal policy identity")
    counters = record["counters"]
    require(type(counters) is dict and set(counters) == set(POLICY_COUNTERS), "five policy counters")
    for key, value in counters.items():
        integer(value, key)
    exact(counters["completed_batches"], context["expected_ordered_groups"],
          "whole-process completed ordered groups")
    completed = integer(counters["completed_without_fallback"] + counters["completed_after_fallback"],
                        "completed group sum")
    exact(completed, counters["completed_batches"], "fallback partition")
    require(counters["fallback_sleeps"] >= counters["completed_after_fallback"]
            and (counters["fallback_sleeps"] == 0) == (counters["completed_after_fallback"] == 0),
            "fallback sleep accounting")
    # Snapshot dispatch totals and the checked close enclose all process groups.
    # Direct dispatch polls may add to this count; never require equality.
    active_polls = integer(completed + counters["spin_pauses"] + counters["fallback_sleeps"],
                           "active completion polls")
    require(delta["completion_polls"] >= active_polls, "active polls exceed enclosing runtime counter")
    return counters


def validate_capture(raw, context):
    """Check raw stderr against independently retained context after clean reap.

    This checks policy evidence only. It neither launches a process nor grants
    native admission; the owning harness must validate the remaining evidence.
    """
    validate_context(context)
    require(type(raw) is bytes and 0 < len(raw) <= LIMIT and raw.endswith(b"\n"),
            "complete bounded stderr")
    lines = raw[:-1].split(b"\n")
    active = context["active_poll"]
    require(len(lines) == (3 if active else 2) and all(lines), "exact runtime/terminal record count")
    records = [decode(line) for line in lines]
    delta = validate_snapshots(records[:2], context)
    terminal = None
    if active:
        require(len(lines[2]) + 1 <= 1024, "terminal record including LF exceeds core bound")
        terminal = validate_terminal(records[2], context, delta)
    return {
        "schema": "FerricOrdered64WaitPolicyEvidenceV1",
        "policy_evidence_consistent": True,
        "policy": "ActivePoll10msV1" if active else "Sleep50usV1",
        "controller_pid": context["controller_pid"], "worker_pid": context["worker_pid"],
        "device_unique_id": context["device_unique_id"],
        "completed_ordered_groups": context["expected_ordered_groups"],
        "runtime_counter_delta": delta, "terminal_counters": terminal,
        "stderr_sha256": hashlib.sha256(raw).hexdigest(),
        "native_execution_accepted": False, "performance_qualified": False,
        "vendor_comparison": False, "gpu_time_measured": False,
        "scope": "Policy consistency only; caller must independently bind context, validate complete raw token/byte replay, actual binary identities, placement/CPU samples, host timing and resource supervision.",
    }


def read_bound(path, expected, maximum):
    digest(expected)
    require(path.is_absolute() and path.resolve(strict=True) == path, "canonical input path")
    fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
    with os.fdopen(fd, "rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= maximum, "bounded regular input")
        raw = stream.read(maximum + 1)
        after = os.fstat(stream.fileno())
    current = path.stat()
    require(len(raw) == before.st_size and all(
        getattr(before, key) == getattr(after, key) == getattr(current, key)
        for key in ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    ), "input changed while reading")
    require(hashlib.sha256(raw).hexdigest() == expected, "input SHA256 mismatch")
    return raw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("stderr", "context"):
        parser.add_argument("--" + name, type=Path, required=True)
        parser.add_argument("--" + name + "-sha256", required=True)
    args = parser.parse_args()
    raw = read_bound(args.stderr, args.stderr_sha256, LIMIT)
    context = decode(read_bound(args.context, args.context_sha256, CONTEXT_LIMIT))
    result = validate_capture(raw, context)
    result["context_sha256"] = args.context_sha256
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))


if __name__ == "__main__":
    main()
