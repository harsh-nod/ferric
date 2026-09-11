#!/usr/bin/env python3
"""Bounded scalar/MFMA projection diagnostics; never a model qualification."""

import argparse
import array
import hashlib
import json
import math
import os
from pathlib import Path
import stat
import struct
import sys
import types


DIFFERENTIAL_SHA256 = "4e1cf2d71411e5865d83af9cf4731b7a49d4321f5b3ef6e99257093a4ac7139f"
MFMA_COLUMN = "ferric_qwen3_tp_mfma_gemm_bf16_v3"
MFMA_PARTIAL = "ferric_qwen3_tp_mfma_gemm_partial_f32_v3"
PATTERNS = ("layout", "mixed", "cancellation")
MAX_WEIGHT_BYTES = 4096 * 12288 * 2


def load_differential(path):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 128 * 1024:
            raise RuntimeError("invalid differential helper file")
        data = os.read(fd, 128 * 1024 + 1)
        after = os.fstat(fd)
        fields = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if (any(getattr(before, field) != getattr(after, field) for field in fields)
                or len(data) != before.st_size
                or hashlib.sha256(data).hexdigest() != DIFFERENTIAL_SHA256):
            raise RuntimeError("differential helper identity mismatch")
    finally:
        os.close(fd)
    module = types.ModuleType("ferric_verified_mfma_differential_helper")
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def words(data):
    result = array.array("H")
    result.frombytes(data)
    if sys.byteorder != "little":
        result.byteswap()
    return result


def word_bytes(values):
    copied = array.array("H", values)
    if sys.byteorder != "little":
        copied.byteswap()
    return copied.tobytes()


def values(data):
    return [struct.unpack("<f", struct.pack("<I", bits << 16))[0]
            for (bits,) in struct.iter_unpack("<H", data)]


def deterministic_bf16(count, label):
    # Vectorized byte translation avoids a Python loop over production weights.
    data = bytearray(hashlib.shake_256(label.encode("ascii")).digest(count * 2))
    high_bytes = bytes((value & 0x80) | (0x3A + (value & 7)) for value in range(256))
    data[1::2] = data[1::2].translate(high_bytes)
    return bytes(data)


def transpose_nk(data, n, k):
    if len(data) != n * k * 2 or not 0 < len(data) <= MAX_WEIGHT_BYTES:
        raise ValueError("transpose byte extent")
    source = words(data)
    output = array.array("H", [0]) * (n * k)
    for column in range(n):
        output[column::n] = source[column * k:(column + 1) * k]
    # Independently check every source row against the completed strided output.
    for column in range(n):
        if output[column::n] != source[column * k:(column + 1) * k]:
            raise RuntimeError("full transpose byte parity failed")
    return word_bytes(output)


def checked_shape(rows, n, k, world, projection, partial):
    if (any(type(value) is not int for value in (rows, n, k, world, projection))
            or type(partial) is not bool or world not in (1, 2, 8) or not 1 <= rows <= 16):
        raise ValueError("projection shape scalar domain")
    if partial:
        valid = n == 4096 and ((projection == 1 and k == 4096 // world)
                              or (projection == 2 and k == 12288 // world))
    else:
        valid = k == 4096 and (
            (projection == 1 and n == 4096 // world)
            or (projection in (2, 3) and n == 1024 // world)
            or (projection in (4, 5) and n == 12288 // world))
    if not valid or n * k * 2 > MAX_WEIGHT_BYTES:
        raise ValueError("projection shape is outside the bounded diagnostic roster")


def fixture_specs(suite):
    worlds = (1, 2, 8) if suite == "shapes" else (1,)
    for world in worlds:
        shapes = ((4096 // world, 4096, 1, False),
                  (1024 // world, 4096, 2, False),
                  (12288 // world, 4096, 4, False),
                  (4096, 4096 // world, 1, True),
                  (4096, 12288 // world, 2, True))
        for n, k, projection, partial in shapes:
            for pattern in PATTERNS:
                rows = 16 if pattern == "layout" else (1 if pattern == "cancellation" else 3)
                yield rows, n, k, world, projection, partial, pattern


def layout_indices(k):
    return [0, 1, 15, 16, 31, 32, 63, 64, 127, 128,
            k // 2, k // 2 + 15, k - 65, k - 17, k - 2, k - 1]


def make_fixture(core, probe, spec):
    rows, n, k, world, projection, partial, pattern = spec
    checked_shape(rows, n, k, world, projection, partial)
    if pattern not in PATTERNS:
        raise ValueError("unknown numerical pattern")
    activation = deterministic_bf16(rows * k, f"activation:{rows}:{k}")
    weights = deterministic_bf16(n * k, f"weights:{n}:{k}")
    if pattern == "layout":
        active_words = array.array("H", [0]) * (rows * k)
        for row, inner in enumerate(layout_indices(k)[:rows]):
            active_words[row * k + inner] = 0x3F80
        activation = word_bytes(active_words)
    elif pattern == "cancellation":
        activation = struct.pack("<H", 0x3F80) * (rows * k)
        positive = [2.0 ** 24, 1.0] + [0.0] * 14 + [-(2.0 ** 24), 1.0] + [0.0] * 14
        negative = [-value for value in positive]
        pair = (probe.pack_bf16(positive) * (k // 32)
                + probe.pack_bf16(negative) * (k // 32))
        weights = pair * (n // 2)
    element_bytes = 4 if partial else 2
    output = b"\xA5" * (16 * n * element_bytes)
    name = f"tp{world}_{'partial' if partial else 'column'}_n{n}_k{k}_rows{rows}_{pattern}"
    return {"name": name, "pattern": pattern, "partial": partial,
            "scalars": [rows, n, k, world, projection],
            "buffers": [core.buffer("a", activation, 2), core.buffer("weights", weights, 2),
                        core.buffer("output", output, element_bytes, output)]}


def reference_dots(left, right):
    products = [f32(a * b) for a, b in zip(left, right, strict=True)]
    serial = 0.0
    chunk16 = 0.0
    for product in products:
        serial = f32(serial + product)
    for start in range(0, len(products), 16):
        chunk16 = f32(chunk16 + math.fsum(products[start:start + 16]))
    return {"serial_fp32": serial, "fp64_products_sum": math.fsum(products),
            "sum_abs_products": math.fsum(abs(value) for value in products),
            "chunk16_once_rounded_hypothesis": chunk16}


def sampled_columns(n):
    return sorted({0, 1, 15, 16, 31, 32, n // 2 - 1, n // 2, n - 17, n - 2, n - 1})


def encode(probe, value, partial):
    return struct.pack("<f", value) if partial else struct.pack("<H", probe.bf16_bits(value))


def layout_expected(case, probe):
    rows, n, k = case["scalars"][:3]
    source = words(case["buffers"][1]["data"])
    output = bytearray()
    for inner in layout_indices(k)[:rows]:
        for column in range(n):
            bits = source[column * k + inner]
            if case["partial"]:
                output.extend(struct.pack("<I", bits << 16))
            else:
                output.extend(struct.pack("<H", bits))
    return bytes(output)


def compare(core, probe, case, baseline, candidate):
    rows, n, k = case["scalars"][:3]
    partial = case["partial"]
    size = 4 if partial else 2
    core.require(len(baseline) == len(candidate) == rows * n * size, "active output extent")
    decode = (lambda data: [value for (value,) in struct.iter_unpack("<f", data)]) if partial else values
    left_output, right_output = decode(baseline), decode(candidate)
    core.require(all(math.isfinite(value) for value in left_output + right_output), "nonfinite projection output")
    changed = sum(baseline[index:index + size] != candidate[index:index + size]
                  for index in range(0, len(baseline), size))
    columns = sampled_columns(n)
    weight_rows = {column: values(case["buffers"][1]["data"][column * k * 2:(column + 1) * k * 2])
                   for column in columns}
    samples = []
    gamma = (k * 2.0 ** -24) / (1.0 - k * 2.0 ** -24)
    for row in range(rows):
        activation = values(case["buffers"][0]["data"][row * k * 2:(row + 1) * k * 2])
        for column in columns:
            refs = reference_dots(activation, weight_rows[column])
            index = row * n + column
            offset = index * size
            precise = refs["fp64_products_sum"]
            error = abs(right_output[index] - precise)
            # A necessary diagnostic bound, not an MFMA-order or model-parity proof.
            forward_bound = gamma * refs["sum_abs_products"]
            if not partial:
                forward_bound += max(abs(precise), abs(right_output[index])) * 2.0 ** -8
            samples.append({"row": row, "column": column, **refs,
                            "baseline_value": left_output[index], "mfma_value": right_output[index],
                            "baseline_matches_serial": baseline[offset:offset + size] == encode(probe, refs["serial_fp32"], partial),
                            "mfma_matches_serial": candidate[offset:offset + size] == encode(probe, refs["serial_fp32"], partial),
                            "mfma_matches_fp64_via_fp32": candidate[offset:offset + size] == encode(probe, precise, partial),
                            "mfma_matches_chunk16_hypothesis": candidate[offset:offset + size] == encode(probe, refs["chunk16_once_rounded_hypothesis"], partial),
                            "baseline_abs_error": abs(left_output[index] - precise),
                            "mfma_abs_error": error, "diagnostic_forward_bound": forward_bound,
                            "mfma_within_diagnostic_bound": error <= forward_bound})
    layout_pass = None
    if case["pattern"] == "layout":
        expected = layout_expected(case, probe)
        layout_pass = baseline == expected and candidate == expected
    differences = [abs(left - right) for left, right in zip(left_output, right_output, strict=True)]
    return {"name": case["name"], "pattern": case["pattern"], "scalars": case["scalars"],
            "elements": rows * n, "different_output_bits": changed,
            "max_abs_difference": max(differences),
            "rms_difference": math.sqrt(math.fsum(value * value for value in differences) / len(differences)),
            "sampled_columns": columns, "samples": samples, "full_layout_exact": layout_pass,
            "diagnostic_checks_pass": layout_pass is not False
            and all(sample["baseline_matches_serial"] and sample["mfma_within_diagnostic_bound"]
                    for sample in samples)}


def write_json(path, value):
    with path.open("x", encoding="utf-8") as output:
        json.dump(value, output, sort_keys=True, indent=2, allow_nan=False)
        output.write("\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--suite", choices=("tp1", "shapes"), default="tp1")
    parser.add_argument("--differential", type=Path, default=Path(__file__).with_name("projection_differential.py"))
    parser.add_argument("--probe", type=Path, default=Path(__file__).with_name("probe.py"))
    parser.add_argument("--helper", type=Path)
    parser.add_argument("--worker", type=Path)
    parser.add_argument("--worker-sha256")
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--artifact-sha256")
    parser.add_argument("--device-unique-id", type=int)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    diff = load_differential(args.differential)
    probe = diff.load_probe(args.probe)
    helper = args.helper or Path(__file__).resolve().parents[3] / "proofs/tensor-parallel-kernels-v1/probe.py"
    core = probe.load_helper(helper)
    specs = list(fixture_specs(args.suite))
    for spec in specs:
        checked_shape(*spec[:6])
    core.require(len(specs) == (15 if args.suite == "tp1" else 45), "closed shape roster")
    if args.self_test:
        core.require(not args.run, "self-test cannot execute GPU work")
        print(f"PASS: {len(specs)} closed MFMA diagnostic shapes, pinned helpers; no GPU launched")
        return
    core.require(args.run and all(value is not None for value in
                 (args.worker, args.worker_sha256, args.artifact, args.artifact_sha256,
                  args.device_unique_id, args.output)), "explicit run and all identities required")
    core.require(0 < args.device_unique_id < 1 << 64 and args.output.is_absolute(), "run destination")
    args.output.mkdir(mode=0o700)
    source_identity = core.digest(Path(__file__).read_bytes())
    worker_fd, worker_stat, _ = core.held_file(args.worker, args.worker_sha256, 512 * 1024 * 1024, 62)
    artifact_fd, worker = None, None
    try:
        artifact_fd, artifact_stat, artifact = core.held_file(args.artifact, args.artifact_sha256, 64 * 1024 * 1024, 224)
        worker = core.Worker(worker_fd, worker_stat, args.device_unique_id, args.output)
        results = []
        for spec in specs:
            case = make_fixture(core, probe, spec)
            _, n, k = case["scalars"][:3]
            baseline_symbol = diff.BASELINE_PARTIAL if case["partial"] else diff.BASELINE_COLUMN
            candidate_symbol = MFMA_PARTIAL if case["partial"] else MFMA_COLUMN
            baseline, baseline_receipt = diff.capture(core, worker, case, baseline_symbol, n // 16,
                                                      artifact, args.artifact_sha256)
            transpose = transpose_nk(case["buffers"][1]["data"], n, k)
            candidate_case = dict(case)
            candidate_case["buffers"] = [case["buffers"][0], core.buffer("weights_kn", transpose, 2), case["buffers"][2]]
            candidate, candidate_receipt = diff.capture(core, worker, candidate_case, candidate_symbol, n // 16,
                                                        artifact, args.artifact_sha256)
            result = compare(core, probe, case, baseline, candidate)
            result["dispatches"] = [baseline_receipt, candidate_receipt]
            result["activation_sha256"] = core.digest(case["buffers"][0]["data"])
            result["weights_nk_sha256"] = core.digest(case["buffers"][1]["data"])
            result["weights_kn_sha256"] = core.digest(transpose)
            result["full_transpose_exact"] = True
            for name, data in (("baseline", baseline), ("mfma", candidate)):
                with (args.output / f"{case['name']}.{name}.bin").open("xb") as output:
                    output.write(data)
            write_json(args.output / f"{case['name']}.json", result)
            results.append(result)
            print(f"{case['name']}: changed={result['different_output_bits']}/{result['elements']} "
                  f"layout={result['full_layout_exact']} diagnostic={result['diagnostic_checks_pass']}", flush=True)
        worker.finish()
        core.require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                     and core.identity(os.fstat(artifact_fd)) == core.identity(artifact_stat), "held identity drifted")
        core.require(source_identity == core.digest(Path(__file__).read_bytes()), "source drifted")
        passed = all(result["diagnostic_checks_pass"] for result in results)
        write_json(args.output / "result.json", {
            "schema": "FerricTpMfmaDifferentialV1", "authority": "none", "benchmark": False,
            "model_inference": False, "model_parity_qualified": False, "suite": args.suite,
            "source_sha256": source_identity, "differential_sha256": DIFFERENTIAL_SHA256,
            "probe_sha256": diff.PROBE_SHA256, "helper_sha256": probe.HELPER_SHA256,
            "worker_sha256": args.worker_sha256, "artifact_sha256": args.artifact_sha256,
            "device_unique_id": args.device_unique_id, "worker_pid": worker.process.pid,
            "worker_start_ticks": worker.start, "clean_teardown": True,
            "diagnostic_checks_pass": passed,
            "reference_scope": "all-output exact layout sentinels; sampled dense/cancellation FP32 and FP64 references",
            "mfma_order_scope": "chunk16 is an unproven hypothesis, never an acceptance requirement",
            "error_bound_scope": "necessary coarse diagnostic bound, not indexing correctness or token parity",
            "timing_scope": "single synchronous diagnostic dispatch only; no accepted performance claim",
            "results": results})
        core.require(passed, "MFMA projection diagnostic failed; raw outputs retained")
    except BaseException:
        if worker is not None and worker.process.poll() is None:
            worker.abort()
        raise
    finally:
        if artifact_fd is not None:
            os.close(artifact_fd)
        os.close(worker_fd)


if __name__ == "__main__":
    main()
