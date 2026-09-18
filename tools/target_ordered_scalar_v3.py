#!/usr/bin/env python3
"""Strict serial/ordered scalar-v3 observations, with separate optional counters.

Both paths retain BF16 weights, activations and logits, target-only TP1 and the
same frozen 32-token reference. Ordered packet groups are not GPU overlap.
"""

import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
import types


CORE_SHA256 = "90cd589fe9b98660f6efb3400775cd269af236688706f37eaedee2d12e92b975"
COUNTER_SOURCE_SHA256 = "23131f9b2aed2b837677077440370e76b7d8d873716760d85628684c6480c3c6"
WORKER_SHA256 = "b4cb30788d4a32d9cae240823c26a80e9103f5698f91d95b16d2bda7e78270f9"
ARTIFACT = {
    "artifact_hsaco_id": "583a889a4ac5f03c7b004cdd52c1d062649853326e9f4b3d251e11de706573b5",
    "artifact_manifest_id": "636e1ca50839455767027616086b632168404de928560880fdb316feb6c54d35",
    "artifact_handoff_id": "3d6714946adb7e459717d3e03868a4a66ff86e47113c762f7f2d1e9edda81e33",
}
ORDERED_PROFILE = "scalar-v3-tp1-bf16"
EXPECTATION_SCHEMA = "FerricTargetOrderedScalarExpectationV1"


def load_pinned(name, digest):
    path = Path(__file__).with_name(name)
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 131072:
            raise ValueError("shared source extent")
        data = os.read(descriptor, 131073)
        after = os.fstat(descriptor)
        attrs = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if len(data) != before.st_size or any(getattr(before, key) != getattr(after, key) for key in attrs):
            raise ValueError("shared source changed")
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValueError("shared source digest")
    finally:
        os.close(descriptor)
    module = types.ModuleType("_frozen_ordered_scalar_" + path.stem)
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


core = load_pinned("target_batch_decode_v1.py", CORE_SHA256)
counter_source = load_pinned("target_batch_profile_v1.py", COUNTER_SOURCE_SHA256)
PLAN = core.PLAN | {"variant", "runtime_ordered_batches", "ordered_batch_profile",
                    "common_comparator_sha256", "counter_source_sha256"}


def checked_plan(plan):
    core.fields(plan, PLAN, "ordered scalar expectation")
    core.same(plan["schema"], EXPECTATION_SCHEMA, "ordered expectation schema")
    core.require(plan["variant"] in ("serial-control", "ordered-scalar-v3"), "ordered variant")
    ordered = plan["variant"] == "ordered-scalar-v3"
    core.same(plan["runtime_ordered_batches"], ordered, "explicit ordered mode")
    core.same(plan["ordered_batch_profile"], ORDERED_PROFILE if ordered else None, "ordered profile label")
    core.same(plan["common_comparator_sha256"], CORE_SHA256, "shared numerical checker")
    core.same(plan["counter_source_sha256"], COUNTER_SOURCE_SHA256, "shared counter grammar")
    core.same(plan["worker_sha256"], WORKER_SHA256, "frozen original worker")
    for key, value in ARTIFACT.items():
        core.same(plan[key], value, "frozen baseline " + key)
    core.same(plan["new_tokens"], 32, "full frozen token window")
    core.same(plan["kernel_profile"], "v3-wave", "closed scalar-capable v3 image")
    core.same(plan["collective"], "device-tp1-v3", "device TP1 residual")
    core.same(plan["host_timing_enabled"], False, "no separate host-timing instrumentation")
    profile = plan["performance_profile"]
    core.fields(profile, core.PROFILE, "ordered performance profile")
    core.require(type(profile["runtime_profiling"]) is bool, "explicit counter instrumentation flag")
    core.same(profile, {"runtime_cache_admission": True, "runtime_operational": True,
                       "runtime_profiling": profile["runtime_profiling"], "dispatch_sequences": False,
                       "queue_rollover": False, "projection": "baseline", "attention": "baseline"},
              "exact scalar execution profile")
    core.checked_plan(common_plan(plan))
    return plan


def common_plan(plan):
    common = {key: copy.deepcopy(plan[key]) for key in core.PLAN}
    common["schema"] = "FerricTargetBatchDecodeExpectationV1"
    common["performance_profile"]["runtime_profiling"] = False
    return common


def expected_workload(plan):
    return core.expected_workload(checked_plan(plan))


def execution_counts(plan):
    ordered = plan["runtime_ordered_batches"]
    return {"forwards": 36, "packets": 36 * 616,
            "ordered_groups": 36 * 72 if ordered else 0,
            "serial_frontiers": 36 * 4 if ordered else 36 * 616,
            "completion_frontiers": 36 * 76 if ordered else 36 * 616,
            "frontier_count_scope": "source-derived schedule; not a measured GPU event count"}


def validate_counters(records, setup, plan):
    # Ordered groups retain all packets but share publication/completion
    # frontiers. Never replace observed counts to satisfy the serial checker.
    core.require(type(records) is list and len(records) == 2, "exact snapshot roster")
    snapshots = []
    for ordinal, (record, phase) in enumerate(zip(records, ("before_workload", "after_workload"))):
        core.fields(record, counter_source.ENVELOPE, "counter envelope")
        for key, value in {"schema": "FerricQwen3TpRuntimeDiagnosticV1", "authority": "none",
                           "phase": phase, "measurement": counter_source.MEASUREMENT,
                           "performance_qualified": False}.items():
            core.same(record[key], value, "counter envelope " + key)
        core.require(type(record["ranks"]) is list and len(record["ranks"]) == 1, "one counter rank")
        rank = record["ranks"][0]
        core.fields(rank, counter_source.RANK, "counter rank")
        for key, value in {"schema": "FerricRuntimeDiagnosticSnapshotV1", "authority": "none",
                           "performance_qualified": False, "scope": counter_source.RANK_SCOPE,
                           "rank": 0, "ordinal": ordinal, "process_id": setup["worker_pids"][0],
                           "device_unique_id": plan["device_unique_id"]}.items():
            core.same(rank[key], value, "counter identity " + key)
        core.integer(rank["process_id"], 1, 2**32 - 1, "counter process identity")
        core.integer(rank["device_unique_id"], 1, 2**64 - 1, "counter device identity")
        core.fields(rank["counters"], counter_source.COUNTERS, "counter roster")
        for key, value in rank["counters"].items():
            core.integer(value, 0, 2**64 - 1, "u64 counter " + key)
        snapshots.append(dict(rank["counters"]))
    before, after = snapshots
    core.require(all(after[key] >= before[key] for key in before), "counter regression")
    for key in ("dispatches", "completion_polls", "dispatch_prepare_ns", "dispatch_publish_ns", "dispatch_wait_ns"):
        core.same(before[key], 0, "setup contains no dispatch observations")
    core.same(before["kernel_admissions"], 13, "closed image load-time admissions")
    delta = {key: after[key] - before[key] for key in sorted(before)}
    counts = execution_counts(plan)
    core.same(delta["dispatches"], counts["packets"], "unchanged completed packet count")
    core.require(delta["completion_polls"] >= counts["completion_frontiers"], "fresh completion frontier minimum")
    core.require(delta["commands"] >= counts["completion_frontiers"] + delta["reads"] + delta["writes"] + 1,
                 "command frontier minimum plus IO and earlier snapshot")
    core.same(delta["kernel_admissions"], 0, "cached workload kernel admissions")
    core.same(delta["kernel_admission_ns"], 0, "cached workload admission timer")
    for key in ("command_ns", "dispatch_prepare_ns", "dispatch_publish_ns", "dispatch_wait_ns",
                "operational_currentness_checks", "operational_currentness_ns"):
        core.require(delta[key] > 0, "positive enabled workload counter " + key)
    for count, timer in (("reads", "read_ns"), ("writes", "write_ns"),
                         ("full_currentness_checks", "full_currentness_ns"),
                         ("operational_currentness_checks", "operational_currentness_ns")):
        core.require((delta[count] == 0) == (delta[timer] == 0), "counter/timer consistency " + count)
    disjoint = sum(delta[key] for key in ("dispatch_prepare_ns", "dispatch_publish_ns",
                                         "dispatch_wait_ns", "read_ns", "write_ns"))
    core.require(disjoint <= delta["command_ns"], "disjoint phases fit command wall time")
    return {"before_workload": before, "after_workload": after, "workload_delta": delta}


def validate_records(records, plan, reference, diagnostics=None):
    checked_plan(plan)
    profiled = plan["performance_profile"]["runtime_profiling"]
    core.same(diagnostics is not None, profiled, "counter data is explicit and paired")
    core.require(type(records) is list and len(records) == 40, "complete 32-token capture")
    setup = records[0]
    ordered_keys = {"runtime_ordered_batches", "ordered_batch_profile"} if plan["runtime_ordered_batches"] else set()
    profile_keys = {"runtime_diagnostic_status"} if profiled else set()
    core.fields(setup, core.SETUP | {"schema", "authority"} | ordered_keys | profile_keys, "ordered setup")
    core.same(setup["performance_profile"], plan["performance_profile"], "pinned execution profile")
    for key in ordered_keys:
        core.same(setup[key], plan[key], "explicit ordered setup " + key)
    if profiled:
        core.same(setup["runtime_diagnostic_status"], counter_source.DIAGNOSTIC_STATUS, "counter scope")
        core.same(setup["numerical_status"], counter_source.NUMERICAL_STATUS, "profiled numerical status")
    normalized = copy.deepcopy(records)
    for key in ordered_keys | profile_keys:
        del normalized[0][key]
    if profiled:
        normalized[0]["performance_profile"]["runtime_profiling"] = False
        normalized[0]["numerical_status"] = counter_source.COMMON_NUMERICAL_STATUS
    # No choices, bytes, dispatch counts, identities, scheduling or timestamps
    # are adapted. Only the validated opt-in metadata differs from the core.
    result = core.validate_records(normalized, common_plan(plan), reference)
    result.update(schema="FerricTargetOrderedScalarRuntimeDiagnosticV1" if profiled else
                         "FerricTargetOrderedScalarObservationV1",
                  variant=plan["variant"], performance_qualified=False, runtime_profiling=profiled,
                  gpu_timestamps=False, overlap_measured=False, completion_polls_measured=profiled,
                  common_comparator_sha256=CORE_SHA256, counter_source_sha256=COUNTER_SOURCE_SHA256,
                  execution_counts=execution_counts(plan))
    result["configuration"].update(runtime_ordered_batches=plan["runtime_ordered_batches"],
                                   ordered_batch_profile=plan["ordered_batch_profile"],
                                   performance_profile=copy.deepcopy(plan["performance_profile"]),
                                   target_only=True, speculation=False, head_precision="BF16")
    result["nonclaims"].append("Ordered groups change submission/completion frontiers, not packet count, arithmetic or GPU overlap. "
                               "This new profile does not widen the separate v5/v8 ordered path or older checkers.")
    if profiled:
        counters = validate_counters(diagnostics, setup, plan)
        result["runtime_counters"] = counters
        delta = counters["workload_delta"]
        result["derived_host_observations"] = {
            "completion_polls_per_packet": delta["completion_polls"] / result["dispatches"],
            "completion_polls_per_frontier": delta["completion_polls"] / result["execution_counts"]["completion_frontiers"],
            "command_host_seconds": delta["command_ns"] / 1e9,
            "dispatch_prepare_host_seconds": delta["dispatch_prepare_ns"] / 1e9,
            "dispatch_publish_host_seconds": delta["dispatch_publish_ns"] / 1e9,
            "dispatch_wait_host_seconds": delta["dispatch_wait_ns"] / 1e9,
            "currentness_host_seconds": (delta["full_currentness_ns"] + delta["operational_currentness_ns"]) / 1e9,
        }
        result["nonclaims"].extend([
            "Instrumented host-wall counters are not GPU durations or CPU seconds; currentness timers overlap other phases.",
            "Preparation is per packet; ordered publication/wait are per group. Never sum overlapping timer columns.",
            "Deltas include the earlier snapshot command and later snapshot idle/currentness fence, excluding close.",
            "Command timing excludes framing reads/response emission; profiling overhead is uncorrected.",
        ])
    return result


def compare(capture, status, workload, reference, expectation, diagnostic=None):
    core.require(status == b"0\n", "successful controller status")
    plan = checked_plan(core.json_value(expectation))
    core.same(core.json_value(workload), expected_workload(plan), "frozen workload")
    oracle = core.load_reference(reference)
    records = counter_source.records_from_bytes(capture, 40, "capture")
    diagnostics = None if diagnostic is None else counter_source.diagnostic_records_from_bytes(diagnostic)
    result = validate_records(records, plan, oracle, diagnostics)
    result["input_sha256"] = {name: core.sha256(value) for name, value in (
        ("capture", capture), ("status", status), ("workload", workload), ("reference", reference),
        ("expectation", expectation))}
    if diagnostic is not None:
        result["input_sha256"]["diagnostic"] = core.sha256(diagnostic)
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("capture", "status", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--diagnostic", type=Path)
    args = parser.parse_args(argv)
    os.umask(0o077)
    try:
        report = compare(core.read_bounded(args.capture, 1024 * 1024), core.read_bounded(args.status, 16),
                         core.read_bounded(args.workload, 65536), core.read_bounded(args.reference, 131072),
                         core.read_bounded(args.expect, 65536),
                         None if args.diagnostic is None else core.read_bounded(args.diagnostic, 1024 * 1024))
        report["comparator_sha256"] = core.sha256(core.read_bounded(Path(__file__), 131072))
        encoded = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        with args.output.open("x", encoding="utf-8") as stream:
            stream.write(encoded)
        print(json.dumps({"passed": True, "variant": report["variant"], "runtime_profiling": report["runtime_profiling"],
                          "packets": report["dispatches"], "source_frontiers": report["execution_counts"]["completion_frontiers"]}))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError) as error:
        parser.exit(1, f"Target ordered scalar observation rejected ({type(error).__name__}); no successful qualification.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
