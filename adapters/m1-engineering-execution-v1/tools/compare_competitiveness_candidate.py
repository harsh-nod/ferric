#!/usr/bin/env python3
"""Explicit new-profile canaries over the frozen semantic checker, not serving claims."""

import argparse
import copy
import hashlib
import json
from pathlib import Path

import compare_tp_batch as check
import performance_ledger as ledger

SEMANTIC_SHA256 = "1be4f2b3b0f8d1c96ef0b5bf21388da232d6b23e5700c7bef6295117b6093b6a"
METRIC_EXTRACTOR_SHA256 = "6da3b95acda3332bfa27a1c51685dda316d5f188615b9b939554d5eef8c9085e"
SCHEMA = "FerricCompetitivenessCandidateExpectationV1"
COMMON = check.IDENTITY_FIELDS | {
    "schema", "candidate", "world", "workload_sha256", "reference_sha256",
    "physical_gpu_ids", "prefix_cache", "output_head_pruning", "performance_profile",
    "collective", "batch_tokens", "prefill_chunk"}
HEAD = {"head_precision", "fp32_head_artifact", "wide_kernel_profile"}
SHARED = {"peer_artifact", "peer_shared_full_currentness"}
HEAD_WORKSPACE = {"bf16-v8-control": 0, "fp32-v8": 32 * 151936 * 4}


def exact(actual, expected, label):
    check.require(json.dumps(actual, sort_keys=True, allow_nan=False) ==
                  json.dumps(expected, sort_keys=True, allow_nan=False), label)


def expectation(value):
    check.require(type(value) is dict and value.get("schema") == SCHEMA,
                  "candidate expectation schema")
    candidate = value.get("candidate")
    check.require(candidate in ("head32-v8", "shared-full-currentness-v1"), "candidate profile")
    check.fields(value, COMMON | (HEAD if candidate == "head32-v8" else SHARED), "candidate expectation")
    for key in check.IDENTITY_FIELDS | {"workload_sha256", "reference_sha256"}:
        check.hash_value(value[key], key)
    ids = check.integer_list(value["physical_gpu_ids"], 1)
    check.require(len(ids) == 8 and len(set(ids)) == 8, "eight physical GPU identities required")
    for key in ("prefix_cache", "output_head_pruning"):
        check.require(type(value[key]) is bool, "candidate boolean policy")
    profile = check.performance_profile(value["performance_profile"])
    check.require(profile["runtime_profiling"] is False, "diagnostics cannot enter performance ledger")
    world = check.integer(value["world"], 1, 8, "world")
    if candidate == "head32-v8":
        check.require(world == 1 and value["head_precision"] in HEAD_WORKSPACE,
                      "head32 is a separate TP1 profile")
        check.peer_artifact(value["fp32_head_artifact"])
        check.wide_kernel_profile(value["wide_kernel_profile"], profile)
        check.require(profile["projection"] in ("baseline", "mfma")
                      and profile["attention"] == "baseline" and not profile["dispatch_sequences"],
                      "head32 arithmetic scope")
        check.require(value["collective"] in (None, "host-staged-reuse-v3", "device-tp1-v3"),
                      "head32 collective scope")
        for key in ("batch_tokens", "prefill_chunk"):
            check.integer(value[key], 16, 32, key)
    else:
        check.require(world in (2, 8) and value["peer_shared_full_currentness"] is True,
                      "shared observation requires explicit TP2/8 opt-in")
        check.require(value["collective"] in check.PEER_COLLECTIVES, "shared peer collective scope")
        check.peer_artifact(value["peer_artifact"])
        for key in ("batch_tokens", "prefill_chunk"):
            exact(value[key], 16, "shared candidate sixteen-row scope")
    return value


def normalize(records, expected):
    check.require(type(records) is list and 1 < len(records) <= 64, "candidate record count")
    setup = records[0]
    for key in ("batch_tokens", "prefill_chunk"):
        exact(setup.get(key), expected[key], "externally pinned scheduling policy")
    normalized = copy.deepcopy(records)
    if expected["candidate"] == "head32-v8":
        exact(setup.get("head_precision"), expected["head_precision"], "external head32 precision")
        exact(setup.get("fp32_head_artifact"), expected["fp32_head_artifact"], "external head32 image")
        exact(setup.get("fp32_head_workspace_bytes"), HEAD_WORKSPACE[expected["head_precision"]],
              "exact head32 workspace bytes")
        for key in ("head_precision", "fp32_head_artifact", "fp32_head_workspace_bytes"):
            del normalized[0][key]
    else:
        exact(setup.get("peer_shared_full_currentness"), True, "explicit shared observation receipt")
        del normalized[0]["peer_shared_full_currentness"]
    return normalized


def compare(run_dir, workload, reference, expected):
    expectation(expected)
    data = {name: check.read_bounded(run_dir / name, limit) for name, limit in (
        ("status", 16), ("gpu-before.json", 65536), ("gpu-after.json", 65536),
        ("results.jsonl", 8 * 1024 * 1024))}
    data["workload.json"] = check.read_bounded(workload, 1024 * 1024)
    data["reference.json"] = check.read_bounded(reference, 128 * 1024)
    check.require(data["status"] == b"0\n", "candidate did not exit successfully")
    check.load_workload(data["workload.json"])
    check.load_reference(data["reference.json"])
    for name in ("workload", "reference"):
        exact(check.sha256(data[name + ".json"]), expected[name + "_sha256"], "external input hash")
    ids = check.gpu_roster(data["gpu-before.json"])
    exact(ids, expected["physical_gpu_ids"], "external physical roster")
    exact(check.gpu_roster(data["gpu-after.json"]), ids, "post-run idle physical roster")
    raw = data["results.jsonl"]
    check.require(raw.endswith(b"\n"), "truncated candidate JSONL")
    lines = raw.splitlines()
    check.require(1 < len(lines) <= 64 and all(lines), "candidate JSONL record count")
    records = [check.json_value(line) for line in lines]
    normalized = normalize(records, expected)
    report = check.validate_records(
        normalized, ids, expected["world"], {key: expected[key] for key in check.IDENTITY_FIELDS},
        expected["prefix_cache"], expected["output_head_pruning"], expected["collective"],
        expected["performance_profile"], expected.get("peer_artifact"), expected.get("wide_kernel_profile"))
    check.require(report["passed"] is True and report["all_reference_tokens_and_bytes_match"] is True,
                  "candidate did not match the complete fixed reference")
    report.update(schema="FerricCompetitivenessCandidateComparisonV1", qualification=False,
                  candidate=expected["candidate"], expected_candidate=copy.deepcopy(expected),
                  input_sha256={name: check.sha256(value) for name, value in data.items()},
                  semantic_checker_sha256=SEMANTIC_SHA256,
                  metrics=ledger.extract_metrics(normalized),
                  normalization="remove only exact externally pinned candidate Setup extensions in memory",
                  gpu_idle_before_and_after=True)
    report["nonclaims"].append("fixed canary only; no serving, baseline-framework, or confidence-interval qualification")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("run-dir", "workload", "reference", "expect", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    exact(hashlib.sha256(check.read_bounded(Path(check.__file__), 128 * 1024)).hexdigest(),
          SEMANTIC_SHA256, "frozen semantic checker source")
    exact(check.sha256(check.read_bounded(Path(ledger.__file__), 128 * 1024)),
          METRIC_EXTRACTOR_SHA256, "frozen metric extractor source")
    expected_raw = check.read_bounded(args.expect, 65536)
    report = compare(args.run_dir, args.workload, args.reference, check.json_value(expected_raw))
    report["expectation_sha256"] = check.sha256(expected_raw)
    report["comparator_sha256"] = check.sha256(check.read_bounded(Path(__file__), 128 * 1024))
    report["metric_extractor_sha256"] = check.sha256(check.read_bounded(Path(ledger.__file__), 128 * 1024))
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(report, output, indent=2, sort_keys=True, allow_nan=False)
        output.write("\n")


if __name__ == "__main__":
    main()
