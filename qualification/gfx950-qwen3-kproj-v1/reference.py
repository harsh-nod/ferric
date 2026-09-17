#!/usr/bin/env python3
"""Frozen BF16-input, FP32-FMA GEMV error policy and independent FP64 reference."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import struct

from extract_checkpoint import CHECKPOINT_SHA256, REVISION, SHAPE, TENSOR, require

K = 1024
CASES = ("zero", "basis", "mixed", "cancellation")
GAMMA32 = math.nextafter((K * 2.0**-24) / (1.0 - K * 2.0**-24), math.inf)
GAMMA64 = math.nextafter((K * 2.0**-53) / (1.0 - K * 2.0**-53), math.inf)
POLICY = {
    "id": "bf16-exact-fp32-fma-gamma1024-v1",
    "reduction_length": K, "fp32_unit_roundoff": 2.0**-24,
    "reference_unit_roundoff": 2.0**-53, "gamma32_upper": GAMMA32, "gamma64_upper": GAMMA64,
    "absolute_bound": "upward((gamma32_upper+gamma64_upper)*upward(sum_abs/(1-gamma64_upper)))",
    "domain": "finite BF16; nonzero product lattice at least2^-126; sum_abs below2^120",
    "reference": "FP64 math.fsum of exactly widened BF16 products; gamma64 covers reference rounding",
    "exact_cases": "zero and basis require exact FP32 outputs; cancellation requires exact row0 zero",
    "fixed_before_gpu": True,
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def bf16(word):
    return struct.unpack("<f", struct.pack("<I", word << 16))[0]


def bf16_words(data, count):
    require(len(data) == count * 2, "BF16 byte extent mismatch")
    words = struct.unpack("<" + str(count) + "H", data)
    require(all(word & 0x7F80 != 0x7F80 for word in words), "nonfinite BF16 operand")
    return words


def quantum_exponent(word):
    exponent = (word >> 7) & 255
    return exponent - 134 if exponent else -133


def read_weights(directory):
    data = (directory / "weights.bf16le").read_bytes()
    record = json.loads((directory / "checkpoint.json").read_text())
    require(record["checkpoint_sha256"] == CHECKPOINT_SHA256 and record["revision"] == REVISION
            and record["tensor_key"] == TENSOR and record["shape"] == SHAPE
            and record["dtype"] == "BF16" and record["tensor_bytes"] == K * K * 2
            and record["tensor_sha256"] == digest(data), "checkpoint tensor identity mismatch")
    return data, bf16_words(data, K * K), record


def evaluate(weight_words, input_words):
    require(len(weight_words) == K * K and len(input_words) == K, "fixed GEMV shape mismatch")
    nonzero_w = [quantum_exponent(word) for word in weight_words if word & 0x7FFF]
    nonzero_x = [quantum_exponent(word) for word in input_words if word & 0x7FFF]
    require(not nonzero_w or not nonzero_x or min(nonzero_w) + min(nonzero_x) >= -126,
            "operand lattice could permit subnormal products/intermediate sums")
    table = {word: bf16(word) for word in set(weight_words) | set(input_words)}
    require(all(math.isfinite(value) for value in table.values()), "nonfinite reference operand")
    values = [table[word] for word in input_words]
    expected, bounds = [], []
    gamma = math.nextafter(GAMMA32 + GAMMA64, math.inf)
    for row in range(K):
        products = [table[weight_words[row * K + column]] * values[column] for column in range(K)]
        reference = math.fsum(products)
        sum_abs = math.fsum(abs(value) for value in products)
        require(math.isfinite(reference) and sum_abs < 2.0**120, "FP32 overflow-risk envelope")
        upper_sum = math.nextafter(sum_abs / (1.0 - GAMMA64), math.inf) if sum_abs else 0.0
        bound = math.nextafter(gamma * upper_sum, math.inf) if sum_abs else 0.0
        expected.append(reference)
        bounds.append(bound)
    return expected, bounds


def case_inputs(name, weights):
    require(name in CASES, "unknown fixed numerical case")
    words = [0] * K
    exact_rows = []
    if name == "zero":
        exact_rows = list(range(K))
    elif name == "basis":
        words[37] = 0x3F80
        exact_rows = list(range(K))
    elif name == "mixed":
        pattern = [0x3F80, 0xBF80, 0x3F00, 0xBF00, 0x3F40, 0xBF40, 0, 0x3E80]
        words = [pattern[(column * 5 + 3) % len(pattern)] for column in range(K)]
    else:
        columns = [column for column in range(K) if weights[column] & 0x7FFF][:2]
        require(len(columns) == 2, "cancellation fixture requires two nonzero row0 weights")
        left, right = columns
        words[left], words[right] = weights[right], weights[left] ^ 0x8000
        exact_rows = [0]
    return words, exact_rows


def generate(weights_directory, name, output):
    _, weights, checkpoint = read_weights(weights_directory)
    inputs, exact_rows = case_inputs(name, weights)
    expected, bounds = evaluate(weights, inputs)
    if name == "cancellation":
        require(expected[0] == 0.0, "constructed cancellation is not exact")
    input_bytes = struct.pack("<1024H", *inputs)
    expected_bytes = struct.pack("<1024d", *expected)
    bound_bytes = struct.pack("<1024d", *bounds)
    record = {
        "schema": "ferric-qwen3-kproj-reference-v1", "case": name, "policy": POLICY,
        "checkpoint": checkpoint, "input_sha256": digest(input_bytes),
        "reference_sha256": digest(Path(__file__).read_bytes()),
        "expected_sha256": digest(expected_bytes), "bounds_sha256": digest(bound_bytes),
        "exact_output_rows": exact_rows, "output_elements": K,
        "scope": "actual-weight scalar GEMV baseline; not full model or performance evidence",
    }
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    (output / "inputs.bf16le").write_bytes(input_bytes)
    (output / "expected.f64le").write_bytes(expected_bytes)
    (output / "bounds.f64le").write_bytes(bound_bytes)
    (output / "expected.json").write_text(json.dumps(record, indent=2, allow_nan=False) + "\n")
    return record


def compare(actual, expected, bounds, exact_rows):
    require(len(actual) == len(expected) == len(bounds) == K, "output/reference extent mismatch")
    require(all(math.isfinite(value) for value in actual), "nonfinite GPU output")
    exact = set(exact_rows)
    maximum_absolute = 0.0
    maximum_bound_ratio = 0.0
    for row, (observed, reference, bound) in enumerate(zip(actual, expected, bounds)):
        require(math.isfinite(reference) and math.isfinite(bound) and bound >= 0,
                "invalid reference envelope")
        error = abs(observed - reference)
        if row in exact:
            require(observed == reference, "exact zero/basis/cancellation output mismatch")
        require(error <= bound, "GPU result exceeds predeclared FP32 FMA error bound")
        maximum_absolute = max(maximum_absolute, error)
        maximum_bound_ratio = max(maximum_bound_ratio, error / bound if bound else 0.0)
    return {"maximum_absolute_error": maximum_absolute, "maximum_error_to_bound_ratio": maximum_bound_ratio}


def check(weights_directory, case_directory, output_path, result_path):
    _, weights, checkpoint = read_weights(weights_directory)
    record = json.loads((case_directory / "expected.json").read_text())
    require(record["policy"] == POLICY and record["checkpoint"] == checkpoint
            and record["reference_sha256"] == digest(Path(__file__).read_bytes()),
            "reference policy/checkpoint/source changed after generation")
    inputs = (case_directory / "inputs.bf16le").read_bytes()
    words, exact_rows = case_inputs(record["case"], weights)
    require(inputs == struct.pack("<1024H", *words) and digest(inputs) == record["input_sha256"]
            and record["exact_output_rows"] == exact_rows, "reference input/exact-row identity mismatch")
    expected, bounds = evaluate(weights, words)
    for filename, values, hash_key in [("expected.f64le", expected, "expected_sha256"),
                                       ("bounds.f64le", bounds, "bounds_sha256")]:
        serialized = (case_directory / filename).read_bytes()
        require(serialized == struct.pack("<1024d", *values) and digest(serialized) == record[hash_key],
                "saved reference differs from independently recomputed policy")
    output = output_path.read_bytes()
    require(len(output) == K * 4, "GPU output must contain1024 FP32 values")
    metrics = compare(struct.unpack("<1024f", output), expected, bounds, exact_rows)
    result = {
        "schema": "ferric-qwen3-kproj-numerical-v1", "status": "pass", "case": record["case"],
        "policy": POLICY, "checkpoint": checkpoint, "input_sha256": digest(inputs),
        "output_sha256": digest(output), "reference_sha256": record["reference_sha256"],
        "expected_record_sha256": digest((case_directory / "expected.json").read_bytes()),
        "values_checked": K, "exact_rows_checked": len(exact_rows),
        "scope": "numerical file comparison only; caller must bind output to actual dispatch evidence",
        **metrics,
    }
    with result_path.open("x") as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
        stream.write("\n")
    return result


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    generate_parser = commands.add_parser("generate")
    generate_parser.add_argument("--weights-dir", required=True, type=Path)
    generate_parser.add_argument("--case", required=True, choices=CASES)
    generate_parser.add_argument("--case-dir", required=True, type=Path)
    check_parser = commands.add_parser("check")
    for argument in ("weights-dir", "case-dir", "output", "result"):
        check_parser.add_argument("--" + argument, required=True, type=Path)
    args = parser.parse_args()
    if args.command == "generate":
        result = generate(args.weights_dir, args.case, args.case_dir)
    else:
        result = check(args.weights_dir, args.case_dir, args.output, args.result)
    print(json.dumps(result, sort_keys=True, allow_nan=False))


if __name__ == "__main__":
    main()
