#!/usr/bin/env python3
"""Predeclared 16-FMA plus six-add wave-tree policy, independent FP64 reference."""

import argparse
import importlib.util
import json
import math
import os
from pathlib import Path
import struct
import sys

BASELINE = Path(__file__).resolve().parent.parent / "gfx950-qwen3-kproj-v1"
original_path = sys.path.copy()
sys.path.insert(0, str(BASELINE))
try:
    SPEC = importlib.util.spec_from_file_location("kproj_serial_reference", BASELINE / "reference.py")
    serial = importlib.util.module_from_spec(SPEC)
    SPEC.loader.exec_module(serial)
    from extract_checkpoint import require
finally:
    # Loading the independent baseline must not redirect subsequent test/tool imports.
    sys.path[:] = original_path

DEPTH = 16 + 6
GAMMA22 = math.nextafter((DEPTH * 2.0**-24) / (1.0 - DEPTH * 2.0**-24), math.inf)
POLICY = {
    "id": "bf16-exact-fp32-fma16-tree6-gamma22-v1", "terms_per_lane": 16,
    "wave_lanes": 64, "tree_add_depth": 6, "rounding_depth_bound": DEPTH,
    "fp32_unit_roundoff": 2.0**-24, "gamma32_upper": GAMMA22,
    "gamma64_upper": serial.GAMMA64,
    "absolute_bound": "upward((gamma22_upper+gamma64_upper)*upward(sum_abs/(1-gamma64_upper)))",
    "derivation": "each product traverses at most16 FMA roundings and6 tree additions; (1+gamma16)*(1+gamma6)-1 <= gamma22",
    "domain": serial.POLICY["domain"], "reference": serial.POLICY["reference"],
    "exact_cases": serial.POLICY["exact_cases"],
}


def file_hash(path):
    return serial.digest(path.read_bytes())


def calculate(weights_directory, case_directory):
    _, weights, checkpoint = serial.read_weights(weights_directory)
    require(checkpoint["extractor_sha256"] == file_hash(BASELINE / "extract_checkpoint.py"),
            "checkpoint extractor changed")
    case = json.loads((case_directory / "expected.json").read_text())
    require(case["reference_sha256"] == file_hash(BASELINE / "reference.py")
            and case["checkpoint"] == checkpoint and case["policy"] == serial.POLICY,
            "frozen baseline fixture identity changed")
    inputs, exact = serial.case_inputs(case["case"], weights)
    input_bytes = struct.pack("<1024H", *inputs)
    require((case_directory / "inputs.bf16le").read_bytes() == input_bytes
            and serial.digest(input_bytes) == case["input_sha256"], "input fixture changed")
    expected, _ = serial.evaluate(weights, inputs)
    expected_bytes = struct.pack("<1024d", *expected)
    require((case_directory / "expected.f64le").read_bytes() == expected_bytes
            and serial.digest(expected_bytes) == case["expected_sha256"]
            and case["exact_output_rows"] == exact, "independent reference changed")
    values = [serial.bf16(word) for word in inputs]
    gamma = math.nextafter(GAMMA22 + serial.GAMMA64, math.inf)
    bounds = []
    for row in range(1024):
        sum_abs = math.fsum(abs(serial.bf16(weights[row * 1024 + col]) * values[col])
                            for col in range(1024))
        upper_sum = math.nextafter(sum_abs / (1.0 - serial.GAMMA64), math.inf) if sum_abs else 0.0
        bounds.append(math.nextafter(gamma * upper_sum, math.inf) if sum_abs else 0.0)
    record = {
        "schema": "ferric-qwen3-kproj-wave64-policy-v1", "case": case["case"], "policy": POLICY,
        "checkpoint": checkpoint, "input_sha256": serial.digest(input_bytes),
        "expected_sha256": serial.digest(expected_bytes),
        "bounds_sha256": serial.digest(struct.pack("<1024d", *bounds)),
        "reference_sha256": file_hash(Path(__file__)),
        "serial_reference_sha256": file_hash(BASELINE / "reference.py"),
        "extractor_sha256": file_hash(BASELINE / "extract_checkpoint.py"),
        "exact_output_rows": exact,
    }
    return record, expected, bounds


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("generate", "check"))
    for name in ("weights-dir", "case-dir", "policy-dir"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--result", type=Path)
    args = parser.parse_args()
    record, expected, bounds = calculate(args.weights_dir, args.case_dir)
    bounds_bytes = struct.pack("<1024d", *bounds)
    if args.command == "generate":
        args.policy_dir.mkdir(mode=0o700, parents=False, exist_ok=False)
        (args.policy_dir / "bounds.f64le").write_bytes(bounds_bytes)
        (args.policy_dir / "policy.json").write_text(json.dumps(record, indent=2, allow_nan=False) + "\n")
        return
    require(args.output is not None and args.result is not None, "check needs output and result")
    require(json.loads((args.policy_dir / "policy.json").read_text()) == record
            and (args.policy_dir / "bounds.f64le").read_bytes() == bounds_bytes,
            "predeclared wave reduction policy changed")
    output = args.output.read_bytes()
    require(len(output) == 4096, "output extent mismatch")
    errors = serial.compare(struct.unpack("<1024f", output), expected, bounds, record["exact_output_rows"])
    result = {
        "schema": "ferric-qwen3-kproj-wave64-file-comparison-v1", "status": "pass",
        "authority": "none", "case": record["case"], "policy": POLICY,
        "policy_record_sha256": file_hash(args.policy_dir / "policy.json"),
        "input_sha256": record["input_sha256"], "weights_sha256": record["checkpoint"]["tensor_sha256"],
        "output_sha256": serial.digest(output), "values_checked": 1024,
        "exact_rows_checked": len(record["exact_output_rows"]), **errors,
        "scope": "independent file comparison only; caller must bind output to actual wave64 dispatch",
    }
    with args.result.open("x") as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
        stream.write("\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
