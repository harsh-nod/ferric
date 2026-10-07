#!/usr/bin/env python3
"""Explicit TP1 v7 head-precision checks; separate from frozen BF16 ledgers."""

import argparse
import copy
import importlib.util
import json
from pathlib import Path


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("head_v7_frozen_comparator", HERE / "compare_tp_batch.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)
SEMANTIC_SHA256 = "1be4f2b3b0f8d1c96ef0b5bf21388da232d6b23e5700c7bef6295117b6093b6a"
SCHEMA = "FerricQwen3TpHeadPrecisionComparisonV1"
EXPECT_SCHEMA = "FerricQwen3TpHeadPrecisionExpectationV1"
PROFILES = {"bf16-v7-control": 0, "fp32-v7": 9723904}
EXTENSIONS = {"head_precision", "fp32_head_artifact", "fp32_head_workspace_bytes"}


def exact(actual, expected, label):
    CHECK.require(json.dumps(actual, sort_keys=True, allow_nan=False) ==
                  json.dumps(expected, sort_keys=True, allow_nan=False), label)


def expectation(value):
    CHECK.fields(value, CHECK.IDENTITY_FIELDS | {"schema", "workload_sha256", "reference_sha256",
                 "physical_gpu_ids", "prefix_cache", "output_head_pruning", "performance_profile",
                 "collective", "head_precision", "fp32_head_artifact"}, "v7 expectation")
    CHECK.require(value["schema"] == EXPECT_SCHEMA, "v7 expectation schema")
    for key in CHECK.IDENTITY_FIELDS | {"workload_sha256", "reference_sha256"}:
        CHECK.hash_value(value[key], key)
    CHECK.peer_artifact(value["fp32_head_artifact"])
    CHECK.require(value["head_precision"] in PROFILES, "unsupported explicit head profile")
    for key in ("prefix_cache", "output_head_pruning"):
        CHECK.require(type(value[key]) is bool, "v7 boolean policy")
    ids = CHECK.integer_list(value["physical_gpu_ids"], 1)
    CHECK.require(len(ids) == 8 and len(set(ids)) == 8, "externally pinned eight-GPU roster")
    profile = CHECK.performance_profile(value["performance_profile"])
    CHECK.require(profile["projection"] in ("baseline", "mfma") and profile["attention"] == "baseline"
                  and profile["dispatch_sequences"] is False, "v7 arithmetic/sequence scope")
    CHECK.require(value["collective"] in (None, "host-staged-reuse-v3", "device-tp1-v3"), "v7 TP1 collective")
    return value


def normalize(records, expected):
    CHECK.require(type(records) is list and 1 < len(records) <= 64, "v7 record count")
    setup = records[0]
    exact(setup.get("head_precision"), expected["head_precision"], "external head precision")
    CHECK.peer_artifact(setup.get("fp32_head_artifact"))
    exact(setup["fp32_head_artifact"], expected["fp32_head_artifact"], "external v7 artifact identities")
    exact(setup.get("fp32_head_workspace_bytes"), PROFILES[expected["head_precision"]], "exact v7 workspace payload")
    exact([setup.get("tensor_parallel"), setup.get("batch_tokens"), setup.get("prefill_chunk")],
          [1, 16, 16], "v7 TP1/16-row geometry")
    normalized = copy.deepcopy(records)
    for key in EXTENSIONS:
        del normalized[0][key]
    return normalized


def compare(run_dir, workload, reference, expected):
    expectation(expected)
    files = {"status": CHECK.read_bounded(run_dir / "status", 16),
             "gpu-before.json": CHECK.read_bounded(run_dir / "gpu-before.json", 65536),
             "gpu-after.json": CHECK.read_bounded(run_dir / "gpu-after.json", 65536),
             "results.jsonl": CHECK.read_bounded(run_dir / "results.jsonl", 8 * 1024 * 1024),
             "workload.json": CHECK.read_bounded(workload, 1024 * 1024),
             "reference.json": CHECK.read_bounded(reference, 128 * 1024)}
    CHECK.require(files["status"] == b"0\n", "v7 run did not exit successfully")
    CHECK.load_reference(files["reference.json"])
    CHECK.load_workload(files["workload.json"])
    for name in ("workload", "reference"):
        exact(CHECK.sha256(files[name + ".json"]), expected[name + "_sha256"], "external " + name + " hash")
    gpu_ids = CHECK.gpu_roster(files["gpu-before.json"])
    exact(CHECK.gpu_roster(files["gpu-after.json"]), gpu_ids, "post-run idle physical roster")
    exact(gpu_ids, expected["physical_gpu_ids"], "external physical roster")
    raw = files["results.jsonl"]
    CHECK.require(raw.endswith(b"\n"), "truncated v7 JSONL")
    lines = raw.splitlines()
    CHECK.require(1 < len(lines) <= 64 and all(lines), "v7 JSONL record count")
    records = [CHECK.json_value(line) for line in lines]
    normalized = normalize(records, expected)
    report = CHECK.validate_records(normalized, gpu_ids, 1,
             {key: expected[key] for key in CHECK.IDENTITY_FIELDS}, expected["prefix_cache"],
             expected["output_head_pruning"], expected["collective"], expected["performance_profile"])
    CHECK.require(report["passed"] is True and report["all_reference_tokens_and_bytes_match"] is True,
                  "v7 fixed-reference evidence is not qualified")
    report.update(schema=SCHEMA, head_precision=expected["head_precision"],
                  fp32_head_artifact=dict(expected["fp32_head_artifact"]),
                  fp32_head_workspace_bytes=PROFILES[expected["head_precision"]],
                  input_sha256={name: CHECK.sha256(data) for name, data in files.items()},
                  gpu_idle_before_and_after=True, semantic_checker_sha256=SEMANTIC_SHA256,
                  normalization="remove only the three exact validated v7 Setup fields in memory; raw trace unchanged",
                  comparison_scope="explicit v7 head-precision profile; not a legacy BF16 ledger input")
    report["identities"]["fp32_head_artifact"] = dict(expected["fp32_head_artifact"])
    report["expected_execution_profile"].update(head_precision=expected["head_precision"],
                                               fp32_head_artifact=dict(expected["fp32_head_artifact"]))
    report["nonclaims"].append("no automatic mixing with frozen BF16 performance ledgers; require an explicit matched v7 control")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("run-dir", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    exact(CHECK.sha256(CHECK.read_bounded(HERE / "compare_tp_batch.py", 128 * 1024)),
          SEMANTIC_SHA256, "frozen semantic checker source")
    expected_raw = CHECK.read_bounded(args.expect, 65536)
    report = compare(args.run_dir, args.workload, args.reference, CHECK.json_value(expected_raw))
    report["expectation_sha256"] = CHECK.sha256(expected_raw)
    report["comparator_sha256"] = CHECK.sha256(CHECK.read_bounded(Path(__file__), 128 * 1024))
    with args.output.open("x", encoding="utf-8") as target:
        json.dump(report, target, sort_keys=True, indent=2, allow_nan=False)
        target.write("\n")


if __name__ == "__main__":
    main()
