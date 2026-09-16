#!/usr/bin/env python3
"""Independent FP64 matrix reference for the small F32 decoder-layer probe.

This is numerical engineering evidence, not model or production qualification.
The device source is scalar-unrolled; this reference uses explicit matrices,
head vectors, and stable row softmax without sharing implementation code.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import random
import struct
import sys

ROWS = 256
INPUT_WIDTH = 12
WEIGHTS = 110
OUTPUT_WIDTH = 40
ATOL = 2e-5
RTOL = 2e-4
EPSILON = 1e-6
STAGES = (
    ("input_norm", 0, 4), ("rotated_q", 4, 8), ("rotated_k", 8, 10),
    ("current_v", 10, 12), ("attention", 12, 16),
    ("attention_residual", 16, 20), ("post_norm", 20, 24),
    ("gate", 24, 28), ("up", 28, 32), ("swiglu", 32, 36),
    ("final_residual", 36, 40),
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def matvec(matrix: list[list[float]], vector: list[float]) -> list[float]:
    require(all(len(row) == len(vector) for row in matrix), "matrix extent")
    return [math.fsum(a * b for a, b in zip(row, vector)) for row in matrix]


def norm(vector: list[float], scale: list[float]) -> list[float]:
    require(len(vector) == len(scale) and bool(vector), "normalization extent")
    denominator = math.sqrt(math.fsum(x * x for x in vector) / len(vector) + EPSILON)
    return [x / denominator * g for x, g in zip(vector, scale)]


def rotate(vector: list[float], cosine: float, sine: float) -> list[float]:
    require(len(vector) == 2, "RoPE head dimension")
    return [vector[0] * cosine - vector[1] * sine,
            vector[1] * cosine + vector[0] * sine]


def softmax(logits: list[float]) -> list[float]:
    maximum = max(logits)
    exponentials = [math.exp(x - maximum) for x in logits]
    denominator = math.fsum(exponentials)
    return [x / denominator for x in exponentials]


def silu(value: float) -> float:
    # Independent stable logistic evaluation in FP64, including large negatives.
    log_denominator = max(0.0, -value) + math.log1p(math.exp(-abs(value)))
    return value * math.exp(-log_denominator)


def decoder_layer(record: list[float], weights: list[float]) -> list[float]:
    require(len(record) == INPUT_WIDTH and len(weights) == WEIGHTS, "layer extent")
    require(all(math.isfinite(x) for x in record + weights), "nonfinite layer input")

    def matrix(offset: int, rows: int) -> list[list[float]]:
        return [weights[offset + 4 * r:offset + 4 * (r + 1)] for r in range(rows)]

    hidden = record[:4]
    normalized = norm(hidden, weights[:4])
    queries = matvec(matrix(4, 4), normalized)
    key = matvec(matrix(20, 2), normalized)
    value = matvec(matrix(28, 2), normalized)
    cosine, sine = weights[108:110]
    query_heads = [rotate(norm(queries[i:i + 2], weights[36:38]), cosine, sine)
                   for i in (0, 2)]
    key = rotate(norm(key, weights[38:40]), cosine, sine)
    keys = [record[4:6], record[6:8], key]
    values = [record[8:10], record[10:12], value]
    attended = []
    for query in query_heads:
        probabilities = softmax([math.fsum(q * k for q, k in zip(query, past))
                                 / math.sqrt(2.0) for past in keys])
        attended.extend(math.fsum(p * v[column] for p, v in zip(probabilities, values))
                        for column in range(2))
    projected = matvec(matrix(40, 4), attended)
    residual = [x + y for x, y in zip(hidden, projected)]
    post = norm(residual, weights[56:60])
    gate = matvec(matrix(60, 4), post)
    up = matvec(matrix(76, 4), post)
    activated = [silu(g) * u for g, u in zip(gate, up)]
    down = matvec(matrix(92, 4), activated)
    final = [x + y for x, y in zip(residual, down)]
    result = (normalized + sum(query_heads, []) + key + value + attended
              + residual + post + gate + up + activated + final)
    require(len(result) == OUTPUT_WIDTH and all(math.isfinite(x) for x in result),
            "nonfinite or malformed reference output")
    return result


def encode(values: list[float], kind: str) -> bytes:
    return struct.pack("<" + kind * len(values), *values)


def decode(raw: bytes, count: int, kind: str) -> list[float]:
    require(len(raw) == count * struct.calcsize(kind), "binary extent")
    return list(struct.unpack("<" + kind * count, raw))


def sha(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def save_json(path: Path, document: dict) -> None:
    with path.open("x", encoding="ascii") as output:
        json.dump(document, output, sort_keys=True, indent=2, allow_nan=False)
        output.write("\n")


def generate(path: Path, case: str) -> dict:
    require(case in {"mixed", "zero", "saturation", "skewed-attention"}, "case")
    rng = random.Random(420950)
    weights = [rng.uniform(-0.6, 0.6) for _ in range(WEIGHTS)]
    for start, length in ((0, 4), (36, 2), (38, 2), (56, 4)):
        weights[start:start + length] = [rng.uniform(0.65, 1.35) for _ in range(length)]
    weights[108:110] = [math.cos(2.0), math.sin(2.0)]
    if case == "saturation":
        weights[60:76] = [30.0 * x for x in weights[60:76]]
    records = []
    for row in range(ROWS):
        record = [rng.uniform(-2.0, 2.0) for _ in range(INPUT_WIDTH)]
        if row % 16 == 0:
            record = [0.0] * INPUT_WIDTH
        elif row % 16 in (1, 2, 3, 4):
            record[:4] = [float(i == row % 16 - 1) for i in range(4)]
        elif row % 16 == 5:
            record[:4] = [1e-8, -1e-8, 2e-8, -2e-8]
        elif row % 16 == 6:
            record[:4] = [30.0, -30.0, 60.0, -60.0]
        if case == "zero":
            record = [0.0] * INPUT_WIDTH
        if case == "skewed-attention":
            record[4:8] = [40.0 * x for x in record[4:8]]
        records.extend(record)
    input_bytes, weight_bytes = encode(records, "f"), encode(weights, "f")
    records = decode(input_bytes, ROWS * INPUT_WIDTH, "f")
    weights = decode(weight_bytes, WEIGHTS, "f")
    expected = []
    for row in range(ROWS):
        expected.extend(decoder_layer(records[row * INPUT_WIDTH:(row + 1) * INPUT_WIDTH], weights))
    expected_bytes = encode(expected, "d")
    manifest = {
        "format": "ferric.gfx950-decoder-layer-reference.v1", "case": case,
        "authority": "independent-reference-only", "rows": ROWS,
        "input_width": INPUT_WIDTH, "weights": WEIGHTS, "output_width": OUTPUT_WIDTH,
        "atol": ATOL, "rtol": RTOL,
        "inputs_sha256": sha(input_bytes), "weights_sha256": sha(weight_bytes),
        "expected_sha256": sha(expected_bytes), "reference_sha256": sha(Path(__file__).read_bytes()),
    }
    path.mkdir(parents=True, exist_ok=False)
    for name, raw in (("inputs.f32le", input_bytes), ("weights.f32le", weight_bytes),
                      ("expected.f64le", expected_bytes)):
        with (path / name).open("xb") as output:
            output.write(raw)
    save_json(path / "case.json", manifest)
    return manifest


def compare(expected: list[float], actual: list[float]) -> dict:
    require(len(expected) == len(actual) == ROWS * OUTPUT_WIDTH, "comparison extent")
    require(all(math.isfinite(x) for x in expected), "nonfinite reference")
    failures = []
    stages = []
    for name, start, end in STAGES:
        maximum = 0.0
        failed = 0
        squared = []
        reference_squared = []
        for row in range(ROWS):
            for column in range(start, end):
                index = row * OUTPUT_WIDTH + column
                ref, got = expected[index], actual[index]
                allowed = ATOL + RTOL * abs(ref)
                finite = math.isfinite(got)
                error = abs(ref - got) if finite else math.inf
                if finite:
                    maximum = max(maximum, error)
                    squared.append(error * error)
                    reference_squared.append(ref * ref)
                if not finite or error > allowed:
                    failed += 1
                    if len(failures) < 20:
                        failures.append({"row": row, "column": column, "stage": name,
                                         "expected": ref, "actual": got if finite else repr(got),
                                         "allowed_absolute_error": allowed})
        stages.append({"stage": name, "values": ROWS * (end - start), "failed": failed,
                       "max_finite_absolute_error": maximum,
                       "finite_nrmse": math.sqrt(math.fsum(squared)
                                                 / max(math.fsum(reference_squared), 1e-30))})
    return {"format": "ferric.gfx950-decoder-layer-comparison.v1",
            "authority": "engineering-numerical-check-only", "production_qualified": False,
            "full_model": False, "passed": not failures, "atol": ATOL, "rtol": RTOL,
            "compared_values": len(expected), "stages": stages, "first_failures": failures}


def check(case_dir: Path, actual_path: Path) -> dict:
    manifest = json.loads((case_dir / "case.json").read_text(encoding="ascii"))
    require(manifest["format"] == "ferric.gfx950-decoder-layer-reference.v1", "case format")
    require(manifest["atol"] == ATOL and manifest["rtol"] == RTOL, "changed tolerance")
    for name, key in (("inputs.f32le", "inputs_sha256"), ("weights.f32le", "weights_sha256"),
                      ("expected.f64le", "expected_sha256")):
        require(sha((case_dir / name).read_bytes()) == manifest[key], "case digest: " + name)
    require(sha(Path(__file__).read_bytes()) == manifest["reference_sha256"], "reference changed")
    expected = decode((case_dir / "expected.f64le").read_bytes(), ROWS * OUTPUT_WIDTH, "d")
    raw = actual_path.read_bytes()
    result = compare(expected, decode(raw, ROWS * OUTPUT_WIDTH, "f"))
    result.update({"case": manifest["case"], "case_sha256": sha((case_dir / "case.json").read_bytes()),
                   "inputs_sha256": manifest["inputs_sha256"], "weights_sha256": manifest["weights_sha256"],
                   "actual_sha256": sha(raw), "reference_sha256": manifest["reference_sha256"]})
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    generate_parser = commands.add_parser("generate")
    generate_parser.add_argument("directory", type=Path)
    generate_parser.add_argument("--case", required=True,
                                 choices=("mixed", "zero", "saturation", "skewed-attention"))
    check_parser = commands.add_parser("check")
    check_parser.add_argument("directory", type=Path)
    check_parser.add_argument("actual", type=Path)
    check_parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    if args.command == "generate":
        generate(args.directory, args.case)
        print("generated independent reference:", args.directory)
        return 0
    result = check(args.directory, args.actual)
    save_json(args.report, result)
    print(json.dumps({"passed": result["passed"], "compared_values": result["compared_values"],
                      "case": result["case"]}, sort_keys=True))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, TypeError) as error:
        print("reference rejected:", error, file=sys.stderr)
        sys.exit(2)
