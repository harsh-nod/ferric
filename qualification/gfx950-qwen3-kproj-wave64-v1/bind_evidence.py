#!/usr/bin/env python3
"""Bind fixed wave64 dispatches to the independently frozen gamma22 policy."""

import argparse
import json
import os
from pathlib import Path
import struct

import reference as wave

require = wave.require
digest = wave.file_hash
CASES = ("zero", "basis", "mixed", "cancellation")
SYMBOL = "ferric_gfx950_qwen3_kproj_wave64_v1"


def load(path):
    return json.loads(path.read_text())


def write_new(path, value):
    with path.open("x") as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write("\n")


def fixture(weights, case, policy):
    record, expected, bounds = wave.calculate(weights, case)
    require(load(policy / "policy.json") == record
            and (policy / "bounds.f64le").read_bytes() == struct.pack("<1024d", *bounds),
            "frozen tree-reduction policy differs from independent calculation")
    return record, expected, bounds


def preflight(weights, cases, policies, output):
    records = [fixture(weights, cases / name, policies / name)[0] for name in CASES]
    require([record["case"] for record in records] == list(CASES), "fixed case order mismatch")
    manifest = {
        "schema": "ferric-qwen3-kproj-wave64-preflight-v1", "cases": records,
        "binder_sha256": digest(Path(__file__)),
        "scope": "CPU revalidation; verify.sh establishes sequencing before its own dispatches",
    }
    write_new(output, manifest)
    return manifest


def validate_dispatch(report, artifact, record, hashes):
    require(report["schema"] == "ferric-qwen3-kproj-probe-v1" and report["authority"] == "none"
            and artifact["schema"] == "ferric-qwen3-kproj-artifact-v1"
            and report["artifact"] == artifact, "artifact or authority mismatch")
    require(artifact["metadata"]["symbol"] == SYMBOL
            and artifact["source_declared_abi"]["grid_work_items"] == [65536, 1, 1]
            and artifact["source_declared_abi"]["max_workgroups"] == [512, 1, 1]
            and artifact["metadata"]["wavefront_size"] == 64,
            "artifact is not the fixed wave64 projection")
    require(report["workgroup"] == [128, 1, 1] and report["grid_work_items"] == [65536, 1, 1]
            and report["completed_dispatches"] == 1
            and report["input_immutability_and_all_allocation_guards_passed"] is True
            and report["free_close_and_worker_exit_passed"] is True,
            "dispatch geometry, completion, guards or cleanup mismatch")
    require(report["input_sha256"] == record["input_sha256"]
            and report["weights_sha256"] == record["checkpoint"]["tensor_sha256"],
            "dispatch does not use frozen checkpoint fixture")
    for key in ("probe_sha256", "worker_sha256", "output_sha256"):
        require(report[key] == hashes[key], "dispatch executable or output identity mismatch")
    for key in ("source_sha256", "object_sha256"):
        require(artifact[key] == hashes[key], "source or object changed")


def bind(args):
    record, expected, bounds = fixture(args.weights_dir, args.case_dir, args.policy_dir)
    manifest = load(args.preflight)
    require(manifest["schema"] == "ferric-qwen3-kproj-wave64-preflight-v1"
            and manifest["binder_sha256"] == digest(Path(__file__))
            and [item["case"] for item in manifest["cases"]] == list(CASES)
            and manifest["cases"][CASES.index(record["case"])] == record,
            "fixture differs from pre-dispatch manifest")
    artifact = load(args.artifact)
    report_path = args.run_dir / "report.json"
    report = load(report_path)
    output_path = args.run_dir / "output.f32le"
    hashes = {name + "_sha256": digest(path) for name, path in [
        ("probe", args.probe), ("worker", args.worker), ("output", output_path),
        ("source", args.source), ("object", args.object),
    ]}
    validate_dispatch(report, artifact, record, hashes)
    output = output_path.read_bytes()
    require(len(output) == 4096, "wave output extent mismatch")
    errors = wave.serial.compare(struct.unpack("<1024f", output), expected, bounds,
                                 record["exact_output_rows"])
    receipt = {
        "schema": "ferric-qwen3-kproj-wave64-dispatch-numerical-v1", "status": "pass",
        "authority": "none", "case": record["case"], "policy": wave.POLICY,
        "checkpoint": record["checkpoint"], "input_sha256": record["input_sha256"],
        "weights_sha256": record["checkpoint"]["tensor_sha256"], **hashes,
        "report_sha256": digest(report_path), "artifact_record_sha256": digest(args.artifact),
        "preflight_sha256": digest(args.preflight), "binder_sha256": digest(Path(__file__)),
        "policy_record_sha256": digest(args.policy_dir / "policy.json"),
        "reference_sha256": record["reference_sha256"],
        "serial_reference_sha256": record["serial_reference_sha256"],
        "extractor_sha256": record["extractor_sha256"],
        "values_checked": 1024, "exact_rows_checked": len(record["exact_output_rows"]), **errors,
        "scope": "wave64 real-weight GEMV engineering observation; no full-model, performance or production claim",
    }
    write_new(args.run_dir / "dispatch-numerical.json", receipt)
    return receipt


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    before = commands.add_parser("preflight")
    for name in ("weights-dir", "cases-dir", "policies-dir", "output"):
        before.add_argument("--" + name, required=True, type=Path)
    after = commands.add_parser("bind")
    for name in ("weights-dir", "case-dir", "policy-dir", "run-dir", "artifact", "probe", "worker",
                 "source", "object", "preflight"):
        after.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    result = preflight(args.weights_dir, args.cases_dir, args.policies_dir, args.output) \
        if args.command == "preflight" else bind(args)
    print(json.dumps(result, sort_keys=True, allow_nan=False))


if __name__ == "__main__":
    main()
