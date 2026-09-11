#!/usr/bin/env python3
"""Exact-reference ordered-batch TP1 canary; no serving qualification."""

import argparse
import copy
import json
from pathlib import Path

import compare_competitiveness_candidate as base

CHECK = base.check
SCHEMA = "FerricOrderedBatchCandidateExpectationV1"
CANDIDATE = "ordered-batch-head32-v8"
BASE_SHA256 = "b7e7eb09a0543cf843e29afd6956b2e80eda3f7e0872101e093da0adcfcbd6db"


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
    CHECK.require(type(value) is dict, "ordered candidate expectation object")
    base.exact(value.get("schema"), SCHEMA, "ordered expectation schema")
    base.exact(value.get("candidate"), CANDIDATE, "explicit ordered candidate")
    base.exact(value.get("runtime_ordered_batches"), True, "explicit ordered batch policy")
    for key, required in (
        ("world", 1), ("wide_kernel_profile", "v5-mfma32"),
        ("head_precision", "fp32-v8"), ("collective", "device-tp1-v3"),
        ("output_head_pruning", True),
    ):
        base.exact(value.get(key), required, "ordered candidate " + key)
    profile = CHECK.performance_profile(value.get("performance_profile"))
    base.exact(profile["projection"], "mfma", "ordered MFMA projection")
    base.exact(profile["attention"], "baseline", "ordered baseline attention")
    base.exact(profile["dispatch_sequences"], False, "ordered is not serial sequences")
    # The pinned v8 contract rejects all unrelated policy and identity extensions.
    shadow = copy.deepcopy(value)
    del shadow["runtime_ordered_batches"]
    shadow.update(schema=base.SCHEMA, candidate="head32-v8")
    base.expectation(shadow)
    return shadow


def normalize(records, shadow):
    CHECK.require(type(records) is list and 1 < len(records) <= 64,
                  "ordered candidate record count")
    CHECK.require(type(records[0]) is dict, "ordered candidate Setup object")
    base.exact(records[0].get("runtime_ordered_batches"), True,
               "explicit ordered batch Setup receipt")
    normalized = copy.deepcopy(records)
    del normalized[0]["runtime_ordered_batches"]
    return base.normalize(normalized, shadow)


def compare(run_dir, workload, reference, expected):
    pins = source_pins()
    shadow = expectation(expected)
    data = {name: CHECK.read_bounded(run_dir / name, limit) for name, limit in (
        ("status", 16), ("gpu-before.json", 65536), ("gpu-after.json", 65536),
        ("results.jsonl", 8 * 1024 * 1024))}
    data["workload.json"] = CHECK.read_bounded(workload, 1024 * 1024)
    data["reference.json"] = CHECK.read_bounded(reference, 128 * 1024)
    CHECK.require(data["status"] == b"0\n", "ordered candidate did not exit successfully")
    CHECK.load_workload(data["workload.json"])
    CHECK.load_reference(data["reference.json"])
    for name in ("workload", "reference"):
        base.exact(CHECK.sha256(data[name + ".json"]), expected[name + "_sha256"],
                   "external input hash")
    ids = CHECK.gpu_roster(data["gpu-before.json"])
    base.exact(ids, expected["physical_gpu_ids"], "external physical roster")
    base.exact(CHECK.gpu_roster(data["gpu-after.json"]), ids, "post-run idle physical roster")
    raw = data["results.jsonl"]
    CHECK.require(raw.endswith(b"\n"), "truncated ordered candidate JSONL")
    lines = raw.splitlines()
    CHECK.require(1 < len(lines) <= 64 and all(lines), "ordered candidate JSONL record count")
    records = [CHECK.json_value(line) for line in lines]
    normalized = normalize(records, shadow)
    # The actual seven-key performance profile is never rewritten or broadened.
    report = CHECK.validate_records(
        normalized, ids, 1, {key: expected[key] for key in CHECK.IDENTITY_FIELDS},
        expected["prefix_cache"], expected["output_head_pruning"], expected["collective"],
        expected["performance_profile"], None, expected["wide_kernel_profile"])
    CHECK.require(report["passed"] is True and report["all_reference_tokens_and_bytes_match"] is True,
                  "ordered candidate did not match the complete fixed reference")
    report.update(
        schema="FerricOrderedBatchCandidateComparisonV1", qualification=False,
        candidate=CANDIDATE, expected_candidate=copy.deepcopy(expected), sources=pins,
        input_sha256={name: CHECK.sha256(value) for name, value in data.items()},
        metrics=base.ledger.extract_metrics(normalized), gpu_idle_before_and_after=True,
        normalization="remove only exact externally pinned ordered-batch and v8 head Setup extensions in memory; preserve actual performance_profile")
    report["nonclaims"].append(
        "fixed canary only; no serving, baseline-framework, per-kernel timing, or confidence-interval qualification")
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
        parser.exit(1, f"Ordered batch candidate rejected: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
