#!/usr/bin/env python3
"""Strict TP1 v8 diagnostics, never a performance comparison.

Only validated diagnostic Setup fields and pinned v8 extensions are normalized,
in memory, for the frozen full-reference checker. Raw files are never changed;
its timing report is discarded. Counter durations overlap and are not GPU time.
"""

import argparse
import copy
import json
from pathlib import Path

import compare_competitiveness_candidate as candidate

CHECK = candidate.check
CANDIDATE_SHA256 = "b7e7eb09a0543cf843e29afd6956b2e80eda3f7e0872101e093da0adcfcbd6db"
SEMANTIC_SHA256 = "1be4f2b3b0f8d1c96ef0b5bf21388da232d6b23e5700c7bef6295117b6093b6a"
EXTRACTOR_SHA256 = "6da3b95acda3332bfa27a1c51685dda316d5f188615b9b939554d5eef8c9085e"
RUNTIME_STATUS = "unqualified cumulative overlapping host-wall snapshots; deltas include the earlier snapshot command"
NUMERICAL_STATUS = "Diagnostic runtime host-wall counters enabled; timings are not performance qualified"
ORDINARY_STATUS = "Contracted; independently compare emitted token IDs; not a serving qualification"
MEASUREMENT = "cumulative overlapping host-wall counters; not GPU timestamps"
COUNTER_SCOPE = "cumulative overlapping worker host-wall counters, not GPU timestamps"
ENVELOPE_SCHEMA = "FerricQwen3TpRuntimeDiagnosticV1"
SNAPSHOT_SCHEMA = "FerricRuntimeDiagnosticSnapshotV1"
COUNTERS = set("commands command_ns full_currentness_checks full_currentness_ns "
               "operational_currentness_checks operational_currentness_ns kernel_admissions "
               "kernel_admission_ns dispatches dispatch_prepare_ns dispatch_publish_ns "
               "dispatch_wait_ns completion_polls reads read_bytes read_ns writes write_bytes write_ns".split())


def source_pins():
    pins = {}
    for name, module, digest in (
        ("candidate_checker", candidate, CANDIDATE_SHA256),
        ("semantic_checker", CHECK, SEMANTIC_SHA256),
        ("metric_extractor_unused", candidate.ledger, EXTRACTOR_SHA256),
    ):
        actual = CHECK.sha256(CHECK.read_bounded(Path(module.__file__), 128 * 1024))
        candidate.exact(actual, digest, f"frozen {name} source")
        pins[name + "_sha256"] = actual
    pins["diagnostic_checker_sha256"] = CHECK.sha256(CHECK.read_bounded(Path(__file__), 128 * 1024))
    return pins


def normalize_diagnostics(records, expected):
    candidate.expectation(expected)
    CHECK.require(expected["candidate"] == "head32-v8" and expected["world"] == 1,
                  "runtime diagnostics require the explicit TP1 v8 candidate")
    CHECK.require(type(records) is list and 1 < len(records) <= 64, "diagnostic trace record count")
    setup = records[0]
    CHECK.require(type(setup) is dict, "diagnostic Setup object")
    profile = copy.deepcopy(expected["performance_profile"])
    profile["runtime_profiling"] = True
    candidate.exact(setup.get("performance_profile"), profile, "exact runtime diagnostic profile")
    candidate.exact(setup.get("runtime_diagnostic_status"), RUNTIME_STATUS, "runtime diagnostic status")
    candidate.exact(setup.get("numerical_status"), NUMERICAL_STATUS, "diagnostic numerical status")
    normalized = copy.deepcopy(records)
    del normalized[0]["runtime_diagnostic_status"]
    normalized[0]["performance_profile"]["runtime_profiling"] = False
    normalized[0]["numerical_status"] = ORDINARY_STATUS
    return candidate.normalize(normalized, expected), profile


def snapshots(raw, setup, dispatches):
    CHECK.require(raw.endswith(b"\n"), "truncated diagnostic stderr")
    text = raw.decode("utf-8", errors="strict")
    observed = []
    for line_number, line in enumerate(text.splitlines(), 1):
        stripped = line.lstrip()
        if stripped.startswith("{") or ENVELOPE_SCHEMA in line or SNAPSHOT_SCHEMA in line:
            CHECK.require(len(line.encode("utf-8")) <= 65536, "oversized diagnostic record")
            value = CHECK.json_value(line)
            CHECK.fields(value, {"schema", "authority", "phase", "measurement",
                                 "performance_qualified", "ranks"}, "diagnostic envelope")
            candidate.exact(value.get("schema"), ENVELOPE_SCHEMA, "diagnostic envelope schema")
            observed.append((line_number, value))
    CHECK.require(len(observed) == 2, "exactly two diagnostic snapshots required")
    counters = []
    for ordinal, (_, envelope) in enumerate(observed):
        phase = ("before_workload", "after_workload")[ordinal]
        candidate.exact(envelope["phase"], phase, "ordered diagnostic phase")
        candidate.exact(envelope["authority"], "none", "diagnostic authority")
        candidate.exact(envelope["performance_qualified"], False, "diagnostic nonqualification")
        candidate.exact(envelope["measurement"], MEASUREMENT, "diagnostic measurement scope")
        ranks = envelope["ranks"]
        CHECK.require(type(ranks) is list and len(ranks) == 1, "one TP1 diagnostic rank required")
        rank = ranks[0]
        CHECK.fields(rank, {"schema", "authority", "performance_qualified", "scope", "process_id",
                            "device_unique_id", "rank", "ordinal", "counters"}, "rank snapshot")
        for key, value in {
            "schema": SNAPSHOT_SCHEMA, "authority": "none", "performance_qualified": False,
            "scope": COUNTER_SCOPE, "process_id": setup["worker_pids"][0],
            "device_unique_id": setup["device_unique_ids"][0], "rank": 0, "ordinal": ordinal,
        }.items():
            candidate.exact(rank[key], value, f"snapshot {key} binding")
        CHECK.fields(rank["counters"], COUNTERS, "runtime counter schema")
        counters.append({key: CHECK.integer(rank["counters"][key], label=key) for key in sorted(COUNTERS)})
    before, after = counters
    CHECK.require(all(after[key] >= before[key] for key in COUNTERS), "runtime counters regressed")
    delta = {key: after[key] - before[key] for key in sorted(COUNTERS)}
    CHECK.require(before["dispatches"] == 0 and delta["dispatches"] == dispatches,
                  "runtime workload dispatch count mismatch")
    CHECK.require(delta["commands"] >= dispatches + 1,
                  "command delta must include dispatches and the earlier snapshot command")
    return {
        "worker": {"process_id": setup["worker_pids"][0],
                   "device_unique_id": setup["device_unique_ids"][0], "rank": 0},
        "stderr_snapshot_lines": [line for line, _ in observed],
        "snapshot_ordinals": [0, 1],
        "cumulative_counters": {"before_workload": before, "after_workload": after},
        "delta_counters": delta,
    }


def diagnose(run_dir, workload, reference, expected):
    pins = source_pins()
    candidate.expectation(expected)
    data = {name: CHECK.read_bounded(run_dir / name, limit) for name, limit in (
        ("status", 16), ("gpu-before.json", 65536), ("gpu-after.json", 65536),
        ("results.jsonl", 8 * 1024 * 1024), ("progress.log", 8 * 1024 * 1024))}
    data["workload.json"] = CHECK.read_bounded(workload, 1024 * 1024)
    data["reference.json"] = CHECK.read_bounded(reference, 128 * 1024)
    CHECK.require(data["status"] == b"0\n", "diagnostic run did not exit successfully")
    CHECK.load_workload(data["workload.json"])
    CHECK.load_reference(data["reference.json"])
    for name in ("workload", "reference"):
        candidate.exact(CHECK.sha256(data[name + ".json"]), expected[name + "_sha256"], "external input hash")
    ids = CHECK.gpu_roster(data["gpu-before.json"])
    candidate.exact(ids, expected["physical_gpu_ids"], "external physical roster")
    candidate.exact(CHECK.gpu_roster(data["gpu-after.json"]), ids, "post-run idle physical roster")
    raw = data["results.jsonl"]
    CHECK.require(raw.endswith(b"\n"), "truncated diagnostic JSONL")
    lines = raw.splitlines()
    CHECK.require(1 < len(lines) <= 64 and all(lines), "diagnostic JSONL record count")
    records = [CHECK.json_value(line) for line in lines]
    normalized, diagnostic_profile = normalize_diagnostics(records, expected)
    checked = CHECK.validate_records(
        normalized, ids, 1, {key: expected[key] for key in CHECK.IDENTITY_FIELDS},
        expected["prefix_cache"], expected["output_head_pruning"], expected["collective"],
        expected["performance_profile"], None, expected["wide_kernel_profile"])
    CHECK.require(checked["passed"] is True and checked["all_reference_tokens_and_bytes_match"] is True,
                  "diagnostic run failed the complete fixed reference")
    report = {
        "schema": "FerricRuntimeDiagnosticReportV1", "authority": "none", "passed": True,
        "qualification": False, "performance_qualified": False, "fixed_reference_passed": True,
        "gpu_idle_before_and_after": True, "clean_teardown_recorded": True,
        "expected_candidate": copy.deepcopy(expected), "diagnostic_profile": diagnostic_profile,
        "identities": checked["identities"], "input_sha256": {key: CHECK.sha256(value) for key, value in data.items()},
        "sources": pins,
        "counts": {key: checked[key] for key in ("batch_count", "physical_token_rows", "generated_tokens",
                   "rank_dispatch_counts", "kernel_row_capacity", "maximum_batch_rows_observed")},
        "normalization": [
            "in memory only: exact diagnostic runtime_profiling true becomes externally expected false",
            "in memory only: remove exact runtime_diagnostic_status and restore frozen ordinary numerical_status",
            "in memory only: remove externally pinned v8 precision/image/workspace Setup extensions",
            "raw files unchanged; frozen semantic timing report discarded; metric extractor never called",
        ],
        "measurement": MEASUREMENT,
        "counter_semantics": [
            "counter durations overlap and must not be summed as exclusive time or GPU execution time",
            "before_workload includes profiled setup; after_workload is cumulative, not a workload-only counter",
            "after-minus-before commands and command_ns include the earlier snapshot command",
            "phase ordering follows source-bound controller labels and ordered stderr, not authenticated cross-stream timestamps",
            "diagnostic snapshots and profiling overhead invalidate ordinary performance comparison",
        ],
    }
    report.update(snapshots(data["progress.log"], records[0], checked["rank_dispatch_counts"][0]))
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("run-dir", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        source_pins()
        raw = CHECK.read_bounded(args.expect, 65536)
        report = diagnose(args.run_dir, args.workload, args.reference, CHECK.json_value(raw))
        report["expectation_sha256"] = CHECK.sha256(raw)
        with args.output.open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2, allow_nan=False)
            output.write("\n")
    except (ValueError, KeyError, TypeError, IndexError, OSError) as error:
        parser.exit(1, f"Runtime diagnostic report rejected: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
