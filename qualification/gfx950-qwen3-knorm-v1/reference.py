#!/usr/bin/env python3
"""Independent staged Qwen K projection / K-RMSNorm engineering reference."""

import argparse
from fractions import Fraction
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import stat
import struct
import sys

HERE = Path(__file__).resolve().parent
WAVE = HERE.parent / "gfx950-qwen3-kproj-wave64-v1"
spec = importlib.util.spec_from_file_location("knorm_wave_reference", WAVE / "reference.py")
wave = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wave)
serial = wave.serial
extractor = sys.modules["extract_checkpoint"]
require = extractor.require
digest = serial.digest
bf16 = serial.bf16
K, HEAD, LANES = 1024, 128, 64
GAMMA_TENSOR = "model.layers.0.self_attn.k_norm.weight"
GAMMA_HASH = "65ba32dce94cb1fde9037d627efd8b734a0e2db04df21ea33c026393371b2c88"
WEIGHTS_HASH = "68dd761a149b0ff707ad0c1ac6d61dfed75d6fedfbbdbfedde4889f5ca0706f4"
CASES = (*serial.CASES, "epsilon")
EPSILON = struct.unpack("<f", struct.pack("<f", 1e-6))[0]
POLICY = {
    "id": "qwen3-k-projection-bf16-staged-knorm-sqrt-div-v1",
    "projection": wave.POLICY,
    "shape": [8, 128], "epsilon_f32": EPSILON,
    "pairing": "lane and lane+64; two separate F32 squares then F32 add",
    "xor_reduction_distances": [1, 2, 4, 8, 16, 32],
    "staging": "BF16(K); F32 square/tree/mean/epsilon; sqrt; reciprocal; F32 multiply; BF16; F32 gamma multiply; BF16",
    "sqrt_ulp_acceptance": 0, "reciprocal_ulp_acceptance": 0,
    "math_scope": "strict correctly-rounded IEEE F32 sqrt/div source/compiler-TCB contract; no rsqrt equivalence or all-input machine proof",
    "basic_arithmetic": "round-to-nearest ties-even F32, no FMA contraction/reassociation",
    "conditional_domain": "zero or 2^-50 <= abs(BF16 K) <= 2^20; real gamma in [2^-10,2^8]; no nonfinite values",
    "composed_model_center": "diagnostic FP64 projection -> BF16 -> FP64 sum/rsqrt -> BF16 normalized -> BF16 gamma product; not pinned-framework bitwise parity",
    "composed_interval": "outward BF16 image of independent gamma22 projection bounds, propagated through exact staged sqrt/div consumer operations; not a framework rsqrt enclosure",
    "fixed_before_gpu": True,
}
GRAPH = {
    "basis": "fixed engineering graph; not source-to-object or runtime publication authority",
    "allocation_bytes": [2048, 2097152, 4096, 256, 2048, 2048],
    "producer_buffers": [0, 1, 2], "consumer_buffers": [2, 3, 4, 5],
    "producer_grid": [65536, 1, 1], "consumer_grid": [512, 1, 1],
    "workgroup": [128, 1, 1], "consumer_lengths": [1024, 128, 1024, 1024],
    "completion_ordered": True, "intermediate_reupload": False,
}
PRIMARY_REFERENCE = {
    "transformers_commit": "0720e206c6ba28887e4d60ef60a6a089f6c1cc76",
    "pytorch_source_audit_commit": "134179474539648ba7dee1317959529fbd0e7f89",
    "modeling_qwen3_sha256": "704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2",
    "framework_executed": False,
}


def file_hash(path):
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            checksum.update(chunk)
    return checksum.hexdigest()


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()


def load_json(path):
    return json.loads(path.read_bytes(), object_pairs_hook=extractor.unique_object)


def gamma_extract(checkpoint, output):
    with checkpoint.open("rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode), "checkpoint must be a regular file")
        raw_length = stream.read(8)
        require(len(raw_length) == 8, "truncated safetensors header")
        length = struct.unpack("<Q", raw_length)[0]
        require(2 <= length <= 16 * 1024 * 1024 and length + 8 <= before.st_size,
                "safetensors header exceeds cap")
        raw_header = stream.read(length)
        header = json.loads(raw_header, object_pairs_hook=extractor.unique_object)
        extractor.inspect_layout(header, before.st_size - 8 - length)
        tensor = header.get(GAMMA_TENSOR)
        require(tensor == {"dtype": "BF16", "shape": [128],
                           "data_offsets": [641208320, 641208576]}, "gamma tensor layout mismatch")
        stream.seek(0)
        checksum = hashlib.sha256()
        while chunk := stream.read(1024 * 1024):
            checksum.update(chunk)
        require(checksum.hexdigest() == extractor.CHECKPOINT_SHA256, "checkpoint hash mismatch")
        offset = 8 + length + tensor["data_offsets"][0]
        stream.seek(offset)
        data = stream.read(256)
        require(len(data) == 256 and digest(data) == GAMMA_HASH, "gamma tensor hash mismatch")
        require(extractor.stat_identity(before) == extractor.stat_identity(os.fstat(stream.fileno())),
                "checkpoint changed during extraction")
    record = {
        "schema": "ferric-qwen3-knorm-gamma-v1", "model": extractor.MODEL,
        "revision": extractor.REVISION, "checkpoint_sha256": extractor.CHECKPOINT_SHA256,
        "checkpoint_bytes": before.st_size, "header_bytes": length,
        "header_sha256": digest(raw_header), "tensor_key": GAMMA_TENSOR,
        **tensor, "absolute_file_offset": offset, "tensor_bytes": len(data),
        "tensor_sha256": digest(data), "reference_sha256": file_hash(Path(__file__)),
        "layout": "unchanged contiguous checkpoint bytes, shared across eight heads",
    }
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    (output / "gamma.bf16le").write_bytes(data)
    (output / "gamma.json").write_bytes(json_bytes(record))
    return record


def read_gamma(directory):
    data = (directory / "gamma.bf16le").read_bytes()
    record = load_json(directory / "gamma.json")
    require(record["schema"] == "ferric-qwen3-knorm-gamma-v1"
            and record["model"] == extractor.MODEL and record["revision"] == extractor.REVISION
            and record["checkpoint_sha256"] == extractor.CHECKPOINT_SHA256
            and record["checkpoint_bytes"] == 1503300328 and record["header_bytes"] == 35552
            and record["header_sha256"] == "399d16f500e925c7e923fe05966c6df6862ab64da60916843119e802f1801bca"
            and record["tensor_key"] == GAMMA_TENSOR and record["dtype"] == "BF16"
            and record["shape"] == [128] and record["data_offsets"] == [641208320, 641208576]
            and record["absolute_file_offset"] == 641243880 and record["tensor_bytes"] == 256
            and record["tensor_sha256"] == GAMMA_HASH == digest(data), "gamma provenance mismatch")
    words = serial.bf16_words(data, HEAD)
    require(all(2**-10 <= bf16(word) <= 2**8 for word in words), "gamma domain mismatch")
    return words, record


def float_bits(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]


def bits_float(bits):
    return struct.unpack("<f", struct.pack("<I", bits))[0]


def adjacent(value, direction, bf=False):
    if value == 0:
        return math.copysign(bf16(1) if bf else bits_float(1), direction)
    bits = float_bits(value)
    delta = (65536 if bf else 1) * (1 if (value > 0) == (direction > 0) else -1)
    return bits_float(bits + delta)


def rounded(value, bf=False):
    """Select nearest representable value with exact rational midpoint comparisons."""
    if not isinstance(value, Fraction) and value == 0:
        return float(value)
    rational = value if isinstance(value, Fraction) else Fraction(value)
    approximate = bits_float(float_bits(float(rational)))
    require(math.isfinite(approximate), "rounding domain overflow")
    if bf:
        approximate = bits_float(float_bits(approximate) & 0xFFFF0000)
    candidates = [adjacent(approximate, -1, bf), approximate, adjacent(approximate, 1, bf)]
    candidates = [candidate for candidate in candidates if math.isfinite(candidate)]
    return min(candidates, key=lambda candidate: (
        abs(Fraction(candidate) - rational), (float_bits(candidate) >> (16 if bf else 0)) & 1))


def bf16_word(value):
    return float_bits(rounded(value, True)) >> 16


def add(left, right):
    if left == 0 and right == 0:
        return -0.0 if math.copysign(1, left) < 0 and math.copysign(1, right) < 0 else 0.0
    return rounded(Fraction(left) + Fraction(right))


def multiply(left, right):
    if left == 0 or right == 0:
        return math.copysign(0.0, left * right)
    return rounded(Fraction(left) * Fraction(right))


def divide(left, right):
    if left == 0:
        return math.copysign(0.0, left * right)
    return rounded(Fraction(left) / Fraction(right))


def sqrt_rn(value):
    require(value > 0 and math.isfinite(value), "sqrt domain mismatch")
    lower = rounded(math.sqrt(value))
    exact = Fraction(value)
    while Fraction(lower) ** 2 > exact:
        lower = adjacent(lower, -1)
    upper = adjacent(lower, 1)
    while Fraction(upper) ** 2 <= exact:
        lower, upper = upper, adjacent(upper, 1)
    midpoint_squared = ((Fraction(lower) + Fraction(upper)) / 2) ** 2
    return lower if exact < midpoint_squared or (exact == midpoint_squared and not float_bits(lower) & 1) else upper


def widen_ulp(value, direction, count):
    for _ in range(count):
        value = adjacent(value, direction)
    return value


def product_interval(left, right):
    values = [multiply(a, b) for a in left for b in right]
    return min(values), max(values)


def norm_intervals(keys, gamma):
    require(len(keys) == K and len(gamma) == HEAD, "norm shape mismatch")
    result = []
    for head in range(8):
        group = keys[head * HEAD:(head + 1) * HEAD]
        squares = []
        for lower, upper in group:
            require(math.isfinite(lower) and math.isfinite(upper) and -2**20 <= lower <= upper <= 2**20,
                    "key interval domain mismatch")
            ends = [multiply(lower, lower), multiply(upper, upper)]
            squares.append((0.0 if lower <= 0 <= upper else min(ends), max(ends)))
        sums = [(add(squares[lane][0], squares[lane + LANES][0]),
                 add(squares[lane][1], squares[lane + LANES][1])) for lane in range(LANES)]
        for distance in POLICY["xor_reduction_distances"]:
            sums = [(add(sums[lane][0], sums[lane ^ distance][0]),
                     add(sums[lane][1], sums[lane ^ distance][1])) for lane in range(LANES)]
        for column, key in enumerate(group):
            total = sums[column % LANES]
            epsilon_sum = tuple(add(multiply(value, 1 / HEAD), EPSILON) for value in total)
            roots = (widen_ulp(sqrt_rn(epsilon_sum[0]), -1, POLICY["sqrt_ulp_acceptance"]),
                     widen_ulp(sqrt_rn(epsilon_sum[1]), 1, POLICY["sqrt_ulp_acceptance"]))
            reciprocal = (widen_ulp(divide(1.0, roots[1]), -1, POLICY["reciprocal_ulp_acceptance"]),
                          widen_ulp(divide(1.0, roots[0]), 1, POLICY["reciprocal_ulp_acceptance"]))
            normalized = tuple(rounded(value, True) for value in product_interval(key, reciprocal))
            weighted = product_interval(normalized, (bf16(gamma[column]),) * 2)
            result.append(tuple(rounded(value, True) for value in weighted))
    return result


def model_center(keys, gamma, wrong_cast=False):
    result = []
    for head in range(8):
        group = [bf16(word) for word in keys[head * HEAD:(head + 1) * HEAD]]
        reciprocal = 1.0 / math.sqrt(math.fsum(value * value for value in group) / HEAD + EPSILON)
        for column, value in enumerate(group):
            normalized = value * reciprocal
            if not wrong_cast:
                normalized = rounded(normalized, True)
            result.append(bf16_word(normalized * bf16(gamma[column])))
    return result


def projection_reference(weights, name):
    if name == "epsilon":
        inputs, exact = [0] * K, list(range(K))
        inputs[37] = 0x3980  # Exact 2^-12 basis activation.
    else:
        inputs, exact = serial.case_inputs(name, weights)
    expected, _ = serial.evaluate(weights, inputs)
    values = [bf16(word) for word in inputs]
    gamma = math.nextafter(wave.GAMMA22 + serial.GAMMA64, math.inf)
    bounds = []
    for row in range(K):
        total = math.fsum(abs(bf16(weights[row * K + column]) * values[column]) for column in range(K))
        upper = math.nextafter(total / (1 - serial.GAMMA64), math.inf) if total else 0.0
        bounds.append(math.nextafter(gamma * upper, math.inf) if total else 0.0)
    intervals = []
    for row, (value, error) in enumerate(zip(expected, bounds)):
        endpoints = (value, value) if row in exact else (math.nextafter(value - error, -math.inf),
                                                        math.nextafter(value + error, math.inf))
        intervals.append(tuple(rounded(endpoint, True) for endpoint in endpoints))
    return inputs, exact, expected, bounds, intervals


def pack_intervals(values):
    return struct.pack("<" + str(len(values) * 2) + "d", *(item for pair in values for item in pair))


def build_case(weights_dir, gamma_dir, name):
    require(name in CASES, "unknown case")
    weight_bytes, weights, checkpoint = serial.read_weights(weights_dir)
    require(digest(weight_bytes) == WEIGHTS_HASH, "pinned projection tensor hash mismatch")
    gamma, gamma_record = read_gamma(gamma_dir)
    inputs, exact, expected, bounds, key_intervals = projection_reference(weights, name)
    composed = norm_intervals(key_intervals, gamma)
    center = model_center([bf16_word(value) for value in expected], gamma)
    files = {
        "inputs.bf16le": struct.pack("<1024H", *inputs),
        "projection.f64le": struct.pack("<1024d", *expected),
        "projection-bounds.f64le": struct.pack("<1024d", *bounds),
        "quantized-intervals.f64le": pack_intervals(key_intervals),
        "composed-model.bf16le": struct.pack("<1024H", *center),
        "composed-intervals.f64le": pack_intervals(composed),
    }
    record = {
        "schema": "ferric-qwen3-knorm-reference-v1", "case": name, "policy": POLICY,
        "primary_source_reference": PRIMARY_REFERENCE,
        "checkpoint": checkpoint, "gamma": gamma_record, "exact_projection_rows": exact,
        "reference_sha256": file_hash(Path(__file__)), "wave_reference_sha256": file_hash(WAVE / "reference.py"),
        "serial_reference_sha256": file_hash(wave.BASELINE / "reference.py"),
        "checkpoint_extractor_sha256": file_hash(wave.BASELINE / "extract_checkpoint.py"),
        "files": {key: digest(data) for key, data in files.items()},
        "scope": "real checkpoint weights and synthetic activations; two dispatches; no model, timing, publication or protected-runtime authority",
    }
    return record, files, (expected, bounds, key_intervals, gamma, composed, center)


def compare_outputs(projection_bytes, quantized_bytes, output_bytes, values, exact):
    expected, bounds, key_intervals, gamma, composed, center = values
    require(len(projection_bytes) == 4096, "projection extent mismatch")
    projection = struct.unpack("<1024f", projection_bytes)
    metrics = serial.compare(projection, expected, bounds, exact)
    keys = serial.bf16_words(quantized_bytes, K)
    outputs = serial.bf16_words(output_bytes, K)
    for value, word, interval in zip(projection, keys, key_intervals):
        require(word == bf16_word(value), "quantized K differs from exact BF16 cast of validated projection")
        require(interval[0] <= bf16(word) <= interval[1], "quantized K outside projection envelope")
        require(word & 0x7FFF == 0 or 2**-50 <= abs(bf16(word)) <= 2**20, "conditional K domain mismatch")
    conditional = norm_intervals([(bf16(word), bf16(word)) for word in keys], gamma)
    for word, local, joint in zip(outputs, conditional, composed):
        require(local[0] == local[1] and word == bf16_word(local[0]),
                "conditional consumer exact BF16 mismatch")
        require(joint[0] <= bf16(word) <= joint[1], "composed interval mismatch")
    return {
        "projection": metrics, "projection_values_checked": K, "quantized_values_checked": K,
        "conditional_norm_values_checked": K, "composed_interval_values_checked": K,
        "conditional_singleton_intervals": sum(left == right for left, right in conditional),
        "composed_model_center_different_values": sum(a != b for a, b in zip(outputs, center)),
        "composed_center_is_diagnostic_not_exact_parity": True,
    }


def identities(args):
    artifact = load_json(args.artifact)
    require(artifact.get("schema") == "ferric-qwen3-knorm-chain-artifact-v1", "artifact schema mismatch")
    require(json_bytes(artifact.get("graph")) == json_bytes(GRAPH), "artifact fixed graph mismatch")
    record = {"artifact_sha256": file_hash(args.artifact)}
    for role in ("producer", "consumer"):
        for suffix in ("source", "object"):
            actual = file_hash(getattr(args, role + "_" + suffix))
            require(artifact[role][suffix + "_sha256"] == actual, "artifact " + role + " " + suffix + " mismatch")
            record[role + "_" + suffix + "_sha256"] = actual
    for name in ("probe", "worker"):
        record[name + "_sha256"] = file_hash(getattr(args, name))
    return record, artifact


def check_report(report, frozen, artifact, record, raw):
    require(report.get("schema") == "ferric-qwen3-knorm-chain-probe-v1"
            and report.get("authority") == "none" and report.get("artifact") == artifact,
            "dispatch artifact/authority mismatch")
    require(type(report.get("completed_dispatches")) is int and report["completed_dispatches"] == 2,
            "dispatch count mismatch")
    for field in ("input_immutability_and_all_allocation_guards_passed", "projection_unchanged_after_consumer",
                  "producer_completion_before_consumer", "intermediate_allocation_reused_without_host_write",
                  "free_close_and_worker_exit_passed"):
        require(report.get(field) is True, "dispatch lifecycle mismatch: " + field)
    required = {"probe_sha256": frozen["probe_sha256"], "worker_sha256": frozen["worker_sha256"],
                "input_sha256": record["files"]["inputs.bf16le"], "weights_sha256": WEIGHTS_HASH,
                "norm_weights_sha256": GAMMA_HASH,
                **{name + "_sha256": digest(data) for name, data in raw.items()}}
    require(all(report.get(key) == value for key, value in required.items()), "dispatch file hash mismatch")


def check_frozen(saved, record, files, directory):
    require(saved == record, "frozen reference or artifact identity changed")
    require(all((directory / name).read_bytes() == data for name, data in files.items()),
            "saved reference bytes changed")


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    extract = sub.add_parser("extract-gamma")
    extract.add_argument("--checkpoint", type=Path, required=True)
    extract.add_argument("--out-dir", type=Path, required=True)
    for command in ("generate", "check"):
        action = sub.add_parser(command)
        for name in ("weights-dir", "gamma-dir", "case-dir", "artifact", "producer-source",
                     "producer-object", "consumer-source", "consumer-object", "probe", "worker"):
            action.add_argument("--" + name, type=Path, required=True)
        if command == "generate":
            action.add_argument("--case", choices=CASES, required=True)
        else:
            action.add_argument("--run-dir", type=Path, required=True)
            action.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "extract-gamma":
        print(json.dumps(gamma_extract(args.checkpoint, args.out_dir), sort_keys=True))
        return
    binding, artifact = identities(args)
    saved = None if args.command == "generate" else load_json(args.case_dir / "case.json")
    record, files, values = build_case(args.weights_dir, args.gamma_dir, args.case if saved is None else saved["case"])
    record["binding"] = binding
    if args.command == "generate":
        args.case_dir.mkdir(mode=0o700, parents=False, exist_ok=False)
        for name, data in files.items():
            (args.case_dir / name).write_bytes(data)
        (args.case_dir / "case.json").write_bytes(json_bytes(record))
        print(json.dumps(record, sort_keys=True))
        return
    check_frozen(saved, record, files, args.case_dir)
    raw = {"projection": (args.run_dir / "projection.f32le").read_bytes(),
           "quantized": (args.run_dir / "quantized.bf16le").read_bytes(),
           "output": (args.run_dir / "output.bf16le").read_bytes()}
    report = load_json(args.run_dir / "report.json")
    check_report(report, binding, artifact, record, raw)
    metrics = compare_outputs(raw["projection"], raw["quantized"], raw["output"], values, record["exact_projection_rows"])
    result = {"schema": "ferric-qwen3-knorm-numerical-v1", "status": "pass", "authority": "none",
              "case": record["case"], "policy": POLICY, "binding": binding,
              "case_record_sha256": file_hash(args.case_dir / "case.json"),
              "report_sha256": file_hash(args.run_dir / "report.json"),
              "output_hashes": {key: digest(data) for key, data in raw.items()}, **metrics,
              "scope": record["scope"]}
    with args.result.open("xb") as stream:
        stream.write(json_bytes(result))
    print(json.dumps(result, sort_keys=True, allow_nan=False))


if __name__ == "__main__":
    main()
