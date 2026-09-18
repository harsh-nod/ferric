#!/usr/bin/env python3
"""Reference-checked v7-head host spans, separate from uninstrumented results."""

import argparse
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import tempfile
import types


HEAD_SHA = "dc4ddd14834186ccb4a7c7468f7e8e3b6953fcbd2e002789187d6cd97d3567eb"
HELPER_SHA = {
    "host_timing_summary.py": "057f4a6dfb929db44714f8192bb1e373184f4170ce3946dcc31be811637b2402",
    "performance_ledger.py": "6da3b95acda3332bfa27a1c51685dda316d5f188615b9b939554d5eef8c9085e",
    "compare_tp_batch.py": "1be4f2b3b0f8d1c96ef0b5bf21388da232d6b23e5700c7bef6295117b6093b6a",
}
PUBLIC_SPANS = {
    "metadata", "embedding", "attention", "feed_forward", "output_head",
    "output_head_normalization", "output_head_projection", "output_head_argmax",
    "output_readback", "batch", "setup", "workload", "close", "controller",
}


def pinned_bytes(path, digest):
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 131072:
            raise ValueError("checker extent")
        raw = os.read(descriptor, 131073)
        after = os.fstat(descriptor)
        attrs = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if len(raw) != before.st_size or any(getattr(before, key) != getattr(after, key) for key in attrs):
            raise ValueError("checker changed during read")
    finally:
        os.close(descriptor)
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError("pinned checker digest")
    return raw


def load_head():
    path = Path(__file__).with_name("target_head_decode_v1.py")
    raw = pinned_bytes(path, HEAD_SHA)
    module = types.ModuleType("_target_head_host_frozen_head")
    module.__file__ = str(path)
    exec(compile(raw, str(path), "exec"), module.__dict__)
    return module


head = load_head()
batch = head.batch


def load_timing():
    directory = Path(__file__).resolve().parents[1] / "adapters/m1-engineering-execution-v1/tools"
    sources = {name: pinned_bytes(directory / name, digest) for name, digest in HELPER_SHA.items()}
    # The old helper uses relative importlib loads. A fresh private snapshot
    # ensures those loads see the verified bytes, never a pre-existing .pyc.
    with tempfile.TemporaryDirectory(prefix="ferric-head-host-checked-") as temporary:
        snapshot = Path(temporary)
        for name, raw in sources.items():
            with (snapshot / name).open("xb") as output:
                output.write(raw)
        spec = importlib.util.spec_from_file_location("_target_head_host_summary", snapshot / "host_timing_summary.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
    return module


def pinned_base():
    pinned_bytes(Path(head.__file__), HEAD_SHA)
    head.pinned_base()
    load_timing()


def checked_plan(plan):
    batch.fields(plan, head.PLAN, "host-span expectation")
    batch.same(plan["schema"], "FerricTargetHeadHostTimingExpectationV1", "host-span schema")
    batch.same(plan["host_timing_enabled"], True, "explicit host instrumentation")
    ordinary = copy.deepcopy(plan)
    ordinary["schema"] = "FerricTargetHeadDecodeExpectationV1"
    ordinary["host_timing_enabled"] = False
    head.checked_plan(ordinary)
    return ordinary


def validate_records(records, plan, reference, sidecar, workload_sha256):
    pinned_base()
    ordinary = checked_plan(plan)
    # There is no host-timing field in the raw setup record. All raw numerical,
    # schedule, timestamp and closure records pass through entirely unchanged.
    report = head.validate_records(records, ordinary, reference)
    summary = load_timing()
    rows = summary.validate_sidecar(sidecar, records, {
        "world": 1, "collective": plan["collective"],
        "performance_profile": plan["performance_profile"],
        "workload_sha256": workload_sha256,
    })
    aggregates = {}
    globals_ = {"controller", "setup", "workload", "close"}
    batches = {record["pool_batch_id"] for record in records
               if record["schema"] == "FerricQwen3TpBatchCompletedV2"}
    span_batches = {label: set() for label in PUBLIC_SPANS - globals_}
    for row in rows:
        if row["category"] == "span" and row["label"] in PUBLIC_SPANS:
            label = row["label"]
            batch.require(row["rank"] is None and row["phase"] == label, "named host span scope")
            if label in globals_:
                batch.require(row["batch"] is None and row["count"] == 1, "global host span scope")
            else:
                batch.require(row["batch"] in batches and row["batch"] not in span_batches[label],
                              "unique per-batch host span")
                span_batches[label].add(row["batch"])
                batch.same(row["count"], 36 if label in {"attention", "feed_forward"} else 1,
                           "per-batch named span count")
            item = aggregates.setdefault(row["label"], {"count": 0, "elapsed_ns": 0, "max_ns": 0})
            item["count"] += row["count"]
            item["elapsed_ns"] += row["elapsed_ns"]
            item["max_ns"] = max(item["max_ns"], row["max_ns"])
    batch.require(set(aggregates) == PUBLIC_SPANS, "complete named host spans")
    batch.require(all(seen == batches for seen in span_batches.values()), "complete per-batch named span roster")
    for label, item in aggregates.items():
        count = 1 if label in {"controller", "setup", "workload", "close"} else 36
        if label in {"attention", "feed_forward"}:
            count *= 36
        batch.same(item["count"], count, "host-span multiplicity " + label)
    report["schema"] = "FerricTargetHeadHostTimingObservationV1"
    report["configuration"]["host_timing_enabled"] = True
    report["head_checker_sha256"] = HEAD_SHA
    report["timing_helper_sha256"] = copy.deepcopy(HELPER_SHA)
    report["host_spans"] = aggregates
    report["nonclaims"].extend([
        "Instrumented capture, not an uninstrumented throughput sample; recording cost is not removed.",
        "Host spans overlap and nest. Do not sum them as disjoint components, GPU durations or GPU overlap.",
        "Named spans include runtime checks, IPC, scheduling and completion waits; they are not kernel execution times.",
    ])
    return report


def compare(capture, status, workload, reference, expectation, timing):
    batch.require(status == b"0\n", "successful controller")
    plan = batch.json_value(expectation)
    checked_plan(plan)
    batch.same(batch.json_value(workload), batch.expected_workload(plan), "frozen workload")
    oracle = batch.load_reference(reference)
    batch.require(capture.endswith(b"\n") and 0 < len(capture) <= 1024 * 1024, "capture extent")
    lines = capture.splitlines()
    batch.require(len(lines) == 40 and all(lines) and all(len(line) <= 65536 for line in lines), "complete capture")
    report = validate_records([batch.json_value(line) for line in lines], plan, oracle,
                              batch.json_value(timing), batch.sha256(workload))
    report["input_sha256"] = {key: batch.sha256(value) for key, value in {
        "capture": capture, "status": status, "workload": workload, "reference": reference,
        "expectation": expectation, "host_timing": timing,
    }.items()}
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("capture", "status", "workload", "reference", "expect", "timing", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        report = compare(batch.read_bounded(args.capture, 1024 * 1024), batch.read_bounded(args.status, 16),
                         batch.read_bounded(args.workload, 65536), batch.read_bounded(args.reference, 131072),
                         batch.read_bounded(args.expect, 65536), batch.read_bounded(args.timing, 64 * 1024 * 1024))
        report["comparator_sha256"] = batch.sha256(batch.read_bounded(Path(__file__), 131072))
        encoded = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
        with args.output.open("x", encoding="utf-8") as output:
            output.write(encoded)
        print(json.dumps({"passed": True, "variant": report["variant"], "tokens": 32,
                          "measurement": "overlapping host spans, not GPU durations"}))
    except (OSError, ValueError, KeyError, TypeError, IndexError, OverflowError):
        parser.exit(1, "Target head host-span diagnostic rejected; no successful observation.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
