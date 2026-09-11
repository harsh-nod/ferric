#!/usr/bin/env python3
"""Pinned v9 physical-pool canary over the unchanged fixed-reference validator."""

import argparse
import copy
import json
from pathlib import Path

import compare_competitiveness_candidate as base

CHECK = base.check
SCHEMA = "FerricLargeKvCandidateExpectationV1"
CANDIDATE = "large-kv-v9"
BASE_SHA256 = "b7e7eb09a0543cf843e29afd6956b2e80eda3f7e0872101e093da0adcfcbd6db"
EXTRA = {"kv_pool_profile", "kv_pool_max_physical_pages", "kv_pool_payload_bytes",
         "kv_pool_artifact", "physical_pages"}
BYTES_PER_PAGE = 16 * 36 * 2 * 1024 * 2


def source_pins():
    pins = {}
    for name, module, digest in (
        ("base_checker", base, BASE_SHA256),
        ("semantic_checker", CHECK, base.SEMANTIC_SHA256),
        ("metric_extractor", base.ledger, base.METRIC_EXTRACTOR_SHA256),
    ):
        actual = CHECK.sha256(CHECK.read_bounded(Path(module.__file__), 128 * 1024))
        base.exact(actual, digest, f"frozen {name} source")
        pins[name + "_sha256"] = actual
    return pins


def expectation(value):
    CHECK.require(type(value) is dict, "large KV expectation object")
    base.exact(value.get("schema"), SCHEMA, "large KV schema")
    base.exact(value.get("candidate"), CANDIDATE, "explicit large KV candidate")
    CHECK.require(EXTRA <= value.keys(), "missing large KV expectation fields")
    pages = CHECK.integer(value["physical_pages"], 4, 16384, "physical pages")
    base.exact(value["kv_pool_profile"], CANDIDATE, "explicit pool profile")
    base.exact(value["kv_pool_max_physical_pages"], 16384, "physical capacity bound")
    base.exact(value["kv_pool_payload_bytes"], pages * BYTES_PER_PAGE, "exact KV array payload")
    CHECK.peer_artifact(value["kv_pool_artifact"])
    shadow = {key: copy.deepcopy(item) for key, item in value.items() if key not in EXTRA}
    shadow.update(schema=base.SCHEMA, candidate="head32-v8")
    base.expectation(shadow)
    return shadow


def normalize(records, expected):
    shadow = expectation(expected)
    CHECK.require(type(records) is list and 1 < len(records) <= 64, "large KV record count")
    setup = records[0]
    for key in EXTRA:
        base.exact(setup.get(key), expected[key], f"externally pinned {key}")
    normalized = base.normalize(records, shadow)
    for key in EXTRA - {"physical_pages"}:
        del normalized[0][key]
    actual = expected["physical_pages"]
    virtual = min(actual, 512)
    normalized[0]["physical_pages"] = virtual
    # Only unused capacity is translated. Retained/cached pages, causal rows,
    # tokens, counters and timings remain subject to the frozen full validator.
    for item in normalized[1:]:
        if item.get("schema") == CHECK.SCHEMA_PREFIX + "CompletedV2":
            retained = CHECK.integer(item.get("retained_pages"), 0, virtual, "retained pages")
            free = CHECK.integer(item.get("free_pages"), 0, actual, "free pages")
            base.exact(free + retained, actual, "actual pool conservation")
            item["free_pages"] = free - (actual - virtual)
    return normalized


def compare(run_dir, workload, reference, expected):
    pins = source_pins()
    expectation(expected)
    data = {name: CHECK.read_bounded(run_dir / name, limit) for name, limit in (
        ("status", 16), ("gpu-before.json", 65536), ("gpu-after.json", 65536),
        ("results.jsonl", 8 * 1024 * 1024))}
    data["workload.json"] = CHECK.read_bounded(workload, 1024 * 1024)
    data["reference.json"] = CHECK.read_bounded(reference, 128 * 1024)
    CHECK.require(data["status"] == b"0\n", "large KV candidate did not exit successfully")
    CHECK.load_workload(data["workload.json"])
    CHECK.load_reference(data["reference.json"])
    for name in ("workload", "reference"):
        base.exact(CHECK.sha256(data[name + ".json"]), expected[name + "_sha256"], "external input hash")
    ids = CHECK.gpu_roster(data["gpu-before.json"])
    base.exact(ids, expected["physical_gpu_ids"], "external physical roster")
    base.exact(CHECK.gpu_roster(data["gpu-after.json"]), ids, "post-run idle physical roster")
    raw = data["results.jsonl"]
    CHECK.require(raw.endswith(b"\n"), "truncated large KV JSONL")
    lines = raw.splitlines()
    CHECK.require(1 < len(lines) <= 64 and all(lines), "large KV JSONL record count")
    normalized = normalize([CHECK.json_value(line) for line in lines], expected)
    report = CHECK.validate_records(
        normalized, ids, 1, {key: expected[key] for key in CHECK.IDENTITY_FIELDS},
        expected["prefix_cache"], expected["output_head_pruning"], expected["collective"],
        expected["performance_profile"], None, expected["wide_kernel_profile"])
    CHECK.require(report["passed"] is True and report["all_reference_tokens_and_bytes_match"] is True,
                  "large KV candidate did not match the complete fixed reference")
    report.update(
        schema="FerricLargeKvCandidateComparisonV1", qualification=False,
        candidate=CANDIDATE, expected_candidate=copy.deepcopy(expected), sources=pins,
        actual_physical_pages=expected["physical_pages"],
        actual_kv_pool_payload_bytes=expected["kv_pool_payload_bytes"],
        input_sha256={name: CHECK.sha256(value) for name, value in data.items()},
        metrics=base.ledger.extract_metrics(normalized), gpu_idle_before_and_after=True,
        normalization="in-memory only: remove pinned v8/v9 Setup extensions; translate unused capacity to min(actual,512); retain every other field")
    report["nonclaims"].extend([
        "model trace does not expose physical page addresses; high-page device access needs separate native evidence",
        "fixed small canary only; no long-context, 32-concurrent-request, serving or baseline-framework qualification"])
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("run-dir", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        source_pins()
        raw = CHECK.read_bounded(args.expect, 65536)
        report = compare(args.run_dir, args.workload, args.reference, CHECK.json_value(raw))
        report["expectation_sha256"] = CHECK.sha256(raw)
        report["comparator_sha256"] = CHECK.sha256(CHECK.read_bounded(Path(__file__), 128 * 1024))
        with args.output.open("x", encoding="utf-8") as output:
            json.dump(report, output, indent=2, sort_keys=True, allow_nan=False)
            output.write("\n")
    except (ValueError, KeyError, TypeError, IndexError, OSError) as error:
        parser.exit(1, f"Large KV candidate rejected: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
