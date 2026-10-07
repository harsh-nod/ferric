#!/usr/bin/env python3
"""CPU-only ranking analysis of retained M=5 logits, not a new reference run."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import struct
import sys


specification = importlib.util.spec_from_file_location(
    "ferric_m5_rank_reference", Path(__file__).with_name("engineering_m5_reference.py")
)
if specification is None or specification.loader is None:
    raise ImportError("cannot load sibling M=5 reference")
reference = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = reference
specification.loader.exec_module(reference)
core = reference.core
Failure = reference.Failure


def decode_row(data: bytes) -> list[float]:
    core.bf16_argmax(data)  # Enforce the full vocabulary extent and finite values.
    return [struct.unpack("<f", struct.pack("<I", bits << 16))[0]
            for (bits,) in struct.iter_unpack("<H", data)]


def ranking(values: list[float]) -> dict:
    ordered = sorted(range(len(values)), key=lambda token: (-values[token], token))
    best, second = ordered[:2]
    return {
        "argmax_token": best,
        "top5": [{"token": token, "logit": values[token]} for token in ordered[:5]],
        "top2_gap": values[best] - values[second],
        "maximum_tie_count": sum(value == values[best] for value in values),
    }


def row_diagnostic(actual: bytes, expected: bytes) -> dict:
    ferric, baseline = decode_row(actual), decode_row(expected)
    ferric_rank, reference_rank = ranking(ferric), ranking(baseline)
    chosen, wanted = ferric_rank["argmax_token"], reference_rank["argmax_token"]
    winners = []
    for token in sorted({chosen, wanted}):
        def rank(values):
            return 1 + sum(value > values[token] or (value == values[token] and index < token)
                           for index, value in enumerate(values))
        winners.append({
            "token": token, "ferric_logit": ferric[token],
            "reference_logit": baseline[token],
            "signed_error": ferric[token] - baseline[token],
            "ferric_rank": rank(ferric), "reference_rank": rank(baseline),
        })
    return {
        "ferric": ferric_rank, "reference": reference_rank,
        "token_mismatch": chosen != wanted, "winner_comparison": winners,
        "ferric_winner_minus_reference_winner": ferric[chosen] - ferric[wanted],
        "reference_winner_minus_ferric_winner": baseline[wanted] - baseline[chosen],
        "strict_winner_reversal": (chosen != wanted and ferric[chosen] > ferric[wanted]
                                   and baseline[wanted] > baseline[chosen]),
    }


def analyze(capture: dict, actual: bytes, comparison: dict, expected: bytes) -> dict:
    sequence = reference.validate_capture(capture, actual)
    required = {
        "format": "FERRIC-ENGINEERING-S1-K4-DIFFERENTIAL-V1",
        "authority": "none", "qualification": False, "benchmark_comparable": False,
        "tolerance_reviewed": False, "numerical_pass_claimed": False,
        "reference_execution": "independent-full-133-token-sequence-without-ferric-kv-two-byte-identical-runs",
        "reference_model": {"repository": core.PINNED_REPOSITORY, "revision": core.PINNED_REVISION},
        "input_token_ids": sequence,
        "ferric_logits_sha256": reference.digest(actual),
        "reference_logits_sha256": reference.digest(expected),
    }
    for key, value in required.items():
        if type(comparison.get(key)) is not type(value) or comparison[key] != value:
            raise Failure(f"retained comparison field {key} differs")
    if len(expected) != reference.ROWS * reference.ROW_BYTES:
        raise Failure("reference logits have an invalid five-row extent")
    rows, metrics = [], []
    for index in range(reference.ROWS):
        start, end = index * reference.ROW_BYTES, (index + 1) * reference.ROW_BYTES
        left, right = actual[start:end], expected[start:end]
        rows.append(dict(position=128 + index, **row_diagnostic(left, right)))
        metrics.append(dict(position=128 + index, **reference.row_metrics(left, right)))
    if (core.canonical_bytes(comparison.get("rows")) != core.canonical_bytes(metrics)
            or type(comparison.get("token_mismatch_count")) is not int
            or comparison["token_mismatch_count"] != sum(row["token_mismatch"] for row in rows)
            or type(comparison.get("max_bf16_ulp")) is not int
            or comparison["max_bf16_ulp"] != max(row["max_bf16_ulp"] for row in metrics)):
        raise Failure("retained metrics do not reproduce from the exact logits")
    return {
        "format": "FERRIC-ENGINEERING-M5-RANK-DIAGNOSTIC-V1", "authority": "none",
        "qualification": False, "benchmark_comparable": False,
        "new_model_execution": False, "numerical_pass_claimed": False,
        "cause_established": False, "rows": rows,
        "scope": "Posthoc CPU analysis of retained BF16 logits with lowest-token-ID tie breaking.",
        "nonclaim": "No pre-narrowing FP32 logits or layer activations are available here. "
                    "Ranking margins do not isolate RMSNorm, GEMM, or accumulated model error.",
    }


def implementation() -> dict:
    result = reference.implementation()
    with core.SecureDirectory.open(Path(__file__).parent, "rank diagnostic source") as source:
        name = Path(__file__).name
        result[name] = reference.digest(source.read(name, "rank diagnostic implementation"))
    return result


def run(arguments: list[str]) -> None:
    core.require_isolated_python()
    if len(arguments) != 4:
        raise Failure("usage: engineering_m5_rank_diagnostic.py CAPTURE-DIRECTORY "
                      "REFERENCE-DIRECTORY COMPARISON-SHA256 NEW-OUTPUT-DIRECTORY")
    capture_path, reference_path, expected_sha, output_path = arguments
    core.require_sha256(expected_sha, "retained comparison SHA256")
    sources = implementation()
    with core.SecureDirectory.open(Path(capture_path), "capture") as captured:
        with core.SecureDirectory.open(Path(reference_path), "reference") as baseline:
            capture_bytes = captured.read("capture.json", "capture transcript", maximum=1024 * 1024)
            comparison_bytes = baseline.read("comparison.json", "comparison", maximum=1024 * 1024)
            if reference.digest(comparison_bytes) != expected_sha:
                raise Failure("retained comparison hash mismatch")
            capture = json.loads(capture_bytes, object_pairs_hook=core._unique_object)
            comparison = json.loads(comparison_bytes, object_pairs_hook=core._unique_object)
            if comparison.get("capture_sha256") != reference.digest(capture_bytes):
                raise Failure("retained comparison belongs to another capture")
            actual = captured.read("target-logits.bf16", "Ferric logits", maximum=reference.ROWS * reference.ROW_BYTES)
            expected = baseline.read("reference-logits.bf16", "reference logits", maximum=reference.ROWS * reference.ROW_BYTES)
            result = analyze(capture, actual, comparison, expected)
            for directory, name, data in (
                (captured, "capture.json", capture_bytes),
                (captured, "target-logits.bf16", actual),
                (baseline, "comparison.json", comparison_bytes),
                (baseline, "reference-logits.bf16", expected),
            ):
                if directory.read(name, "input aftercheck", maximum=len(data)) != data:
                    raise Failure(f"diagnostic input changed: {name}")
    if implementation() != sources:
        raise Failure("diagnostic implementation changed")
    result.update({"comparison_sha256": expected_sha,
                   "capture_sha256": reference.digest(capture_bytes),
                   "ferric_logits_sha256": reference.digest(actual),
                   "reference_logits_sha256": reference.digest(expected),
                   "implementation_sha256": sources})
    parent, name = core.open_parent(Path(output_path), "rank diagnostic output")
    with parent:
        os.mkdir(name, mode=0o700, dir_fd=parent.fd)
        with parent.child(name, "new rank output") as output:
            core.write_new(output.fd, "rank-diagnostic.json", core.canonical_bytes(result), "rank diagnostic")
    print(f"output={output_path} new_model_execution=false cause_established=false")


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (Failure, OSError, ValueError, KeyError, TypeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
