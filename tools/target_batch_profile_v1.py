#!/usr/bin/env python3
"""Validate a separate target-only BF16 runtime-counter diagnostic, never a benchmark."""

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
import types


CORE_SHA256 = "90cd589fe9b98660f6efb3400775cd269af236688706f37eaedee2d12e92b975"
CONTROLLER_SHA256 = "a28848eadc9aa7a34f21e7909b881c1aa8cdbf7d9bc8f788645504ced77357c2"
EXPECTATION_SCHEMA = "FerricTargetBatchRuntimeProfileExpectationV1"
DIAGNOSTIC_STATUS = "unqualified cumulative overlapping host-wall snapshots; deltas include the earlier snapshot command"
NUMERICAL_STATUS = "Diagnostic runtime host-wall counters enabled; timings are not performance qualified"
COMMON_NUMERICAL_STATUS = "Contracted; independently compare emitted token IDs; not a serving qualification"
MEASUREMENT = "cumulative overlapping host-wall counters; not GPU timestamps"
RANK_SCOPE = "cumulative overlapping worker host-wall counters, not GPU timestamps"
STDERR_PREFIX = b"additional resident transposed weight bytes: 0\n"
COUNTERS = set("commands command_ns full_currentness_checks full_currentness_ns operational_currentness_checks operational_currentness_ns kernel_admissions kernel_admission_ns dispatches dispatch_prepare_ns dispatch_publish_ns dispatch_wait_ns completion_polls reads read_bytes read_ns writes write_bytes write_ns".split())
ENVELOPE = {"schema", "authority", "phase", "measurement", "performance_qualified", "ranks"}
RANK = {"schema", "authority", "performance_qualified", "scope", "process_id",
        "device_unique_id", "rank", "ordinal", "counters"}
U64_MAX = (1 << 64) - 1


def load_core(path):
    # Execute the exact bytes checked here, not a second path-based import.
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or not 0 < info.st_size <= 131072:
            raise ValueError("common checker type/extent")
        data = os.read(fd, 131073)
        if len(data) != info.st_size or hashlib.sha256(data).hexdigest() != CORE_SHA256:
            raise ValueError("common checker source pin")
    finally:
        os.close(fd)
    module = types.ModuleType("ferric_frozen_target_batch_profile_core_v1")
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


core = load_core(Path(__file__).with_name("target_batch_decode_v1.py"))


def checked_plan(plan):
    core.fields(plan, core.PLAN | {"common_comparator_sha256"}, "profile expectation")
    core.same(plan["schema"], EXPECTATION_SCHEMA, "profile expectation schema")
    core.same(plan["common_comparator_sha256"], CORE_SHA256, "common comparator identity")
    core.same(plan["controller_sha256"], CONTROLLER_SHA256, "diagnostic controller identity")
    core.same(plan["new_tokens"], 32, "profile requires 32 output tokens")
    core.same(plan["collective"], "device-tp1-v3", "profile device residual")
    core.same(plan["host_timing_enabled"], False, "separate host timing disabled")
    core.same(plan["performance_profile"], {
        "runtime_cache_admission": True, "runtime_operational": True,
        "runtime_profiling": True, "dispatch_sequences": False, "queue_rollover": False,
        "projection": "baseline", "attention": "baseline",
    }, "exact diagnostic execution options")
    core.checked_plan(common_plan(plan))
    return plan


def common_plan(plan):
    normalized = copy.deepcopy(plan)
    normalized.pop("common_comparator_sha256")
    normalized["schema"] = "FerricTargetBatchDecodeExpectationV1"
    normalized["performance_profile"]["runtime_profiling"] = False
    return normalized


def expected_workload(plan):
    return core.expected_workload(checked_plan(plan))


def snapshots(records, setup, plan):
    core.require(type(records) is list and len(records) == 2, "exact diagnostic snapshot roster")
    counters = []
    for ordinal, (record, phase) in enumerate(zip(records, ("before_workload", "after_workload"))):
        core.fields(record, ENVELOPE, "runtime diagnostic envelope")
        for key, value in {"schema": "FerricQwen3TpRuntimeDiagnosticV1", "authority": "none",
                           "phase": phase, "measurement": MEASUREMENT, "performance_qualified": False}.items():
            core.same(record[key], value, "runtime envelope " + key)
        core.require(type(record["ranks"]) is list and len(record["ranks"]) == 1, "one diagnostic rank")
        rank = record["ranks"][0]
        core.fields(rank, RANK, "rank diagnostic")
        for key, value in {"schema": "FerricRuntimeDiagnosticSnapshotV1", "authority": "none",
                           "performance_qualified": False, "scope": RANK_SCOPE, "rank": 0,
                           "ordinal": ordinal, "process_id": setup["worker_pids"][0],
                           "device_unique_id": plan["device_unique_id"]}.items():
            core.same(rank[key], value, "rank diagnostic " + key)
        core.integer(rank["process_id"], 1, (1 << 32) - 1, "diagnostic process")
        core.integer(rank["device_unique_id"], 1, U64_MAX, "diagnostic device")
        core.fields(rank["counters"], COUNTERS, "runtime counters")
        for name, value in rank["counters"].items():
            core.integer(value, 0, U64_MAX, "u64 counter " + name)
        counters.append(dict(rank["counters"]))
    before, after = counters
    core.require(all(after[key] >= before[key] for key in COUNTERS), "counter regression")
    core.same(before["dispatches"], 0, "setup dispatch count")
    for key in ("completion_polls", "dispatch_prepare_ns", "dispatch_publish_ns", "dispatch_wait_ns"):
        core.same(before[key], 0, "setup has no dispatch observations")
    delta = {key: after[key] - before[key] for key in sorted(COUNTERS)}
    core.same(delta["dispatches"], 36 * 616, "exact profiled dispatch work")
    core.require(delta["completion_polls"] >= delta["dispatches"], "each dispatch requires a fresh completion poll")
    core.same(delta["kernel_admissions"], 0, "no new kernel admissions during cached workload")
    core.same(delta["kernel_admission_ns"], 0, "no workload kernel admission timer")
    core.same(before["kernel_admissions"], 13, "closed v3 image load-time admissions")
    core.require(delta["commands"] >= delta["dispatches"] + delta["reads"] + delta["writes"] + 1,
                 "workload commands plus earlier snapshot command")
    for key in ("command_ns", "dispatch_prepare_ns", "dispatch_publish_ns", "dispatch_wait_ns",
                "operational_currentness_checks", "operational_currentness_ns"):
        core.require(delta[key] > 0, "enabled positive workload observation " + key)
    for count, timer in (("reads", "read_ns"), ("writes", "write_ns"),
                         ("full_currentness_checks", "full_currentness_ns"),
                         ("operational_currentness_checks", "operational_currentness_ns")):
        core.require((delta[count] == 0) == (delta[timer] == 0), "counter/timer consistency " + count)
    disjoint = sum(delta[key] for key in ("dispatch_prepare_ns", "dispatch_publish_ns",
                                         "dispatch_wait_ns", "read_ns", "write_ns"))
    core.require(disjoint <= delta["command_ns"], "disjoint worker phases fit command wall time")
    return before, after, delta


def validate_records(records, diagnostic_records, plan, reference):
    checked_plan(plan)
    core.require(type(records) is list and len(records) == 40, "32-token capture roster")
    setup = records[0]
    core.fields(setup, core.SETUP | {"schema", "authority", "runtime_diagnostic_status"}, "profile setup")
    core.same(setup["runtime_diagnostic_status"], DIAGNOSTIC_STATUS, "diagnostic status")
    core.same(setup["numerical_status"], NUMERICAL_STATUS, "profile numerical status")
    core.same(setup["performance_profile"], plan["performance_profile"], "profile options match plan")
    core.require(type(setup["worker_pids"]) is list and len(setup["worker_pids"]) == 1, "profile worker roster")
    before, after, delta = snapshots(diagnostic_records, setup, plan)
    # Only previously checked profiling metadata is adapted. All identities,
    # choices, page accounting, ordering, timings and closure remain untouched.
    normalized = copy.deepcopy(records)
    normalized[0].pop("runtime_diagnostic_status")
    normalized[0]["numerical_status"] = COMMON_NUMERICAL_STATUS
    normalized[0]["performance_profile"]["runtime_profiling"] = False
    common = core.validate_records(normalized, common_plan(plan), reference)
    configuration = copy.deepcopy(common["configuration"])
    configuration["performance_profile"]["runtime_profiling"] = True
    result = {key: copy.deepcopy(common[key]) for key in (
        "authority", "model", "revision", "precision", "tensor_parallel", "concurrent_requests",
        "measured_requests", "warmup_requests", "reference_sha256", "reference_tokens_and_bytes_match",
        "generated_tokens", "generated_utf8_bytes", "processed_kv_tokens", "dispatches", "identities", "timing", "cleanup")}
    result.update(schema="FerricTargetBatchRuntimeProfileDiagnosticV1", passed=True,
                  benchmark_qualified=False, performance_qualified=False, runtime_profiling=True,
                  gpu_timestamps=False, overlap_measured=False, completion_polls_measured=True,
                  configuration=configuration, common_comparator_sha256=CORE_SHA256,
                  runtime_counters={"before_workload": before, "after_workload": after, "workload_delta": delta},
                  derived_host_observations={
                      "completion_polls_per_dispatch": delta["completion_polls"] / delta["dispatches"],
                      "command_host_seconds": delta["command_ns"] / 1e9,
                      "dispatch_prepare_host_seconds": delta["dispatch_prepare_ns"] / 1e9,
                      "dispatch_publish_host_seconds": delta["dispatch_publish_ns"] / 1e9,
                      "dispatch_wait_host_seconds": delta["dispatch_wait_ns"] / 1e9,
                      "currentness_host_seconds": (delta["full_currentness_ns"] + delta["operational_currentness_ns"]) / 1e9,
                  }, nonclaims=[
                      "Profiled single unwarmed request, not a benchmark, speedup, or performance qualification.",
                      "Counters are worker host-wall observations, not GPU timestamps, kernel durations or overlap evidence.",
                      "Currentness/admission timers overlap other phases; do not sum all timer columns as independent contributions.",
                      "Deltas include the earlier snapshot command and the later snapshot idle/currentness fence; close is excluded.",
                      "Command timing excludes framing reads and response emission; profiling and host logging overhead are uncorrected.",
                      "Only this frozen 32-token prompt is reference-checked; receipts are not independent hardware attestation.",
                      "External pre/post idle/topology/resource accounting must be validated separately.",
                  ])
    return result


def records_from_bytes(data, count, label):
    core.require(data.endswith(b"\n") and 0 < len(data) <= 1024 * 1024, label + " extent/newline")
    lines = data.splitlines()
    core.require(len(lines) == count and all(0 < len(line) <= 65536 for line in lines), label + " line roster")
    return [core.json_value(line) for line in lines]


def diagnostic_records_from_bytes(data):
    core.require(0 < len(data) <= 1024 * 1024 and data.startswith(STDERR_PREFIX),
                 "exact baseline zero-transposed-weight status before snapshots")
    return records_from_bytes(data[len(STDERR_PREFIX):], 2, "diagnostic snapshots")


def compare(capture, diagnostic, status, workload, reference, expectation):
    core.require(status == b"0\n", "successful controller status")
    plan = checked_plan(core.json_value(expectation))
    core.same(core.json_value(workload), expected_workload(plan), "frozen diagnostic workload")
    oracle = core.load_reference(reference)
    report = validate_records(records_from_bytes(capture, 40, "capture"),
                              diagnostic_records_from_bytes(diagnostic), plan, oracle)
    report["input_sha256"] = {name: core.sha256(value) for name, value in (
        ("capture", capture), ("diagnostic", diagnostic), ("status", status),
        ("workload", workload), ("reference", reference), ("expectation", expectation))}
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("capture", "diagnostic", "status", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    os.umask(0o077)
    try:
        report = compare(core.read_bounded(args.capture, 1024 * 1024),
                         core.read_bounded(args.diagnostic, 1024 * 1024), core.read_bounded(args.status, 16),
                         core.read_bounded(args.workload, 65536), core.read_bounded(args.reference, 131072),
                         core.read_bounded(args.expect, 65536))
        report["comparator_sha256"] = core.sha256(core.read_bounded(Path(__file__), 131072))
        serialized = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        with args.output.open("x", encoding="utf-8") as output:
            output.write(serialized)
        print(json.dumps({"passed": True, "performance_qualified": False,
                          "dispatches": report["dispatches"],
                          "completion_polls": report["runtime_counters"]["workload_delta"]["completion_polls"]}))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError, UnicodeError) as error:
        parser.exit(1, f"Target runtime diagnostic rejected ({type(error).__name__}); no successful qualification.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
