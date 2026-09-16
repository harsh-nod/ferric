#!/usr/bin/env python3
"""Bind frozen numerical fixtures and comparisons to engineering dispatch reports."""

import argparse
import json
import os
from pathlib import Path
import struct

import extract_checkpoint
import reference
from extract_checkpoint import require


def load(path):
    return json.loads(path.read_text())


def file_hash(path):
    return reference.digest(path.read_bytes())


def fixture_identity(weights_directory, case_directory):
    _, weights, checkpoint = reference.read_weights(weights_directory)
    require(checkpoint["extractor_sha256"] == file_hash(Path(extract_checkpoint.__file__)),
            "checkpoint extractor source changed")
    record = load(case_directory / "expected.json")
    require(record["policy"] == reference.POLICY and record["checkpoint"] == checkpoint
            and record["reference_sha256"] == file_hash(Path(reference.__file__)),
            "fixture policy, checkpoint or reference source changed")
    inputs, exact_rows = reference.case_inputs(record["case"], weights)
    expected, bounds = reference.evaluate(weights, inputs)
    serialized = {
        "inputs.bf16le": (struct.pack("<1024H", *inputs), "input_sha256"),
        "expected.f64le": (struct.pack("<1024d", *expected), "expected_sha256"),
        "bounds.f64le": (struct.pack("<1024d", *bounds), "bounds_sha256"),
    }
    for name, (wanted, hash_key) in serialized.items():
        actual = (case_directory / name).read_bytes()
        require(actual == wanted and reference.digest(actual) == record[hash_key],
                "fixture bytes differ from predeclared independent calculation")
    require(record["exact_output_rows"] == exact_rows, "fixture exact-output contract changed")
    return {
        "case": record["case"], "expected_record_sha256": file_hash(case_directory / "expected.json"),
        "input_sha256": record["input_sha256"], "expected_sha256": record["expected_sha256"],
        "bounds_sha256": record["bounds_sha256"], "weights_sha256": checkpoint["tensor_sha256"],
    }


def preflight(weights_directory, cases_directory, manifest_path):
    cases = [fixture_identity(weights_directory, cases_directory / name) for name in reference.CASES]
    require([case["case"] for case in cases] == list(reference.CASES), "fixture case order mismatch")
    manifest = {
        "schema": "ferric-qwen3-kproj-preflight-v1", "policy": reference.POLICY,
        "reference_sha256": file_hash(Path(reference.__file__)), "cases": cases,
        "extractor_sha256": file_hash(Path(extract_checkpoint.__file__)),
        "binder_sha256": file_hash(Path(__file__)),
        "scope": "CPU fixture revalidation; only verify.sh sequencing establishes that this step precedes its dispatches",
    }
    with manifest_path.open("x") as stream:
        json.dump(manifest, stream, indent=2)
        stream.write("\n")
    return manifest


def bind(weights_directory, case_directory, run_directory, artifact_path,
         probe_path, worker_path, preflight_path):
    identity = fixture_identity(weights_directory, case_directory)
    predeclared = load(preflight_path)
    names = [case["case"] for case in predeclared["cases"]]
    require(predeclared["schema"] == "ferric-qwen3-kproj-preflight-v1"
            and predeclared["policy"] == reference.POLICY
            and predeclared["reference_sha256"] == file_hash(Path(reference.__file__))
            and predeclared["extractor_sha256"] == file_hash(Path(extract_checkpoint.__file__))
            and predeclared["binder_sha256"] == file_hash(Path(__file__))
            and names == list(reference.CASES)
            and predeclared["cases"][names.index(identity["case"])] == identity,
            "fixture identity differs from before dispatch")
    artifact = load(artifact_path)
    report_path = run_directory / "report.json"
    report = load(report_path)
    require(report["schema"] == "ferric-qwen3-kproj-probe-v1" and report["authority"] == "none"
            and artifact["schema"] == "ferric-qwen3-kproj-artifact-v1"
            and report["artifact"] == artifact, "dispatch artifact or authority mismatch")
    require(report["probe_sha256"] == file_hash(probe_path)
            and report["worker_sha256"] == file_hash(worker_path), "dispatch executable identity mismatch")
    require(report["input_sha256"] == identity["input_sha256"]
            and report["weights_sha256"] == identity["weights_sha256"]
            and report["output_sha256"] == file_hash(run_directory / "output.f32le"),
            "dispatch input, checkpoint weights or output identity mismatch")
    require(report["workgroup"] == [128, 1, 1] and report["grid_work_items"] == [1024, 1, 1]
            and report["completed_dispatches"] == 1
            and report["input_immutability_and_all_allocation_guards_passed"] is True
            and report["free_close_and_worker_exit_passed"] is True,
            "dispatch geometry, completion, guards or cleanup mismatch")
    comparison_path = run_directory / "comparison.json"
    comparison = reference.check(weights_directory, case_directory, run_directory / "output.f32le",
                                 comparison_path)
    require(comparison["input_sha256"] == report["input_sha256"]
            and comparison["output_sha256"] == report["output_sha256"], "comparison identity mismatch")
    evidence = {
        "schema": "ferric-qwen3-kproj-dispatch-numerical-v1", "status": "pass",
        "authority": "none", "case": identity["case"], "policy": reference.POLICY,
        "checkpoint": comparison["checkpoint"], "source_sha256": artifact["source_sha256"],
        "object_sha256": artifact["object_sha256"], "probe_sha256": report["probe_sha256"],
        "worker_sha256": report["worker_sha256"], "input_sha256": report["input_sha256"],
        "weights_sha256": report["weights_sha256"], "output_sha256": report["output_sha256"],
        "report_sha256": file_hash(report_path), "artifact_record_sha256": file_hash(artifact_path),
        "comparison_sha256": file_hash(comparison_path), "preflight_sha256": file_hash(preflight_path),
        "reference_sha256": comparison["reference_sha256"], "values_checked": comparison["values_checked"],
        "extractor_sha256": file_hash(Path(extract_checkpoint.__file__)),
        "binder_sha256": file_hash(Path(__file__)),
        "exact_rows_checked": comparison["exact_rows_checked"],
        "maximum_absolute_error": comparison["maximum_absolute_error"],
        "maximum_error_to_bound_ratio": comparison["maximum_error_to_bound_ratio"],
        "scope": "actual-weight scalar GEMV engineering evidence; no full-model, speedup or production claim",
    }
    with (run_directory / "dispatch-numerical.json").open("x") as stream:
        json.dump(evidence, stream, indent=2, allow_nan=False)
        stream.write("\n")
    return evidence


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    before = commands.add_parser("preflight")
    for name in ("weights-dir", "cases-dir", "manifest"):
        before.add_argument("--" + name, required=True, type=Path)
    after = commands.add_parser("bind")
    for name in ("weights-dir", "case-dir", "run-dir", "artifact", "probe", "worker", "preflight"):
        after.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    if args.command == "preflight":
        result = preflight(args.weights_dir, args.cases_dir, args.manifest)
    else:
        result = bind(args.weights_dir, args.case_dir, args.run_dir, args.artifact,
                      args.probe, args.worker, args.preflight)
    print(json.dumps(result, sort_keys=True, allow_nan=False))


if __name__ == "__main__":
    main()
