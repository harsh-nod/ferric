#!/usr/bin/env python3
"""Bounded native projection differential diagnostics, not a model benchmark."""

import argparse
import array
import hashlib
import json
import math
import os
from pathlib import Path
import random
import stat
import struct
import sys
import types


PROBE_SHA256 = "1fbce2326565a5224d7ec2ed5326e95113324d8cb305d4c26a8802bd8bd1751c"
BASELINE_COLUMN = "ferric_qwen3_tp_batch_gemm_bf16_f32_bf16_v2"
BASELINE_PARTIAL = "ferric_qwen3_tp_batch_gemm_partial_bf16_f32_v2"
WAVE_COLUMN = "ferric_qwen3_tp_wave_gemv_bf16_v3"
WAVE_PARTIAL = "ferric_qwen3_tp_wave_gemv_partial_f32_v3"


def load_probe(path):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 128 * 1024:
            raise RuntimeError("invalid fixture helper file")
        data = os.read(fd, 128 * 1024 + 1)
        after = os.fstat(fd)
        fields = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if (any(getattr(before, field) != getattr(after, field) for field in fields)
                or len(data) != before.st_size
                or hashlib.sha256(data).hexdigest() != PROBE_SHA256):
            raise RuntimeError("fixture helper identity mismatch")
    finally:
        os.close(fd)
    module = types.ModuleType("ferric_verified_projection_fixture_helper")
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def unpack_bf16(data):
    return [struct.unpack("<f", struct.pack("<I", bits << 16))[0]
            for (bits,) in struct.iter_unpack("<H", data)]


def finite_bits(count, seed, exponent_base, exponent_span):
    generator = random.Random(seed)
    values = array.array("H")
    for _ in range(count):
        bits = generator.getrandbits(32)
        values.append(((bits >> 16) & 0x8000)
                      | ((exponent_base + ((bits >> 8) % exponent_span)) << 7)
                      | (bits & 127))
    if sys.byteorder != "little":
        values.byteswap()
    return values.tobytes()


def fixture_specs():
    for rows in (1, 3, 16):
        for pattern in ("mixed", "wide"):
            yield rows, 128, 4096, 2, False, pattern
    for k, projection in ((512, 1), (1536, 2)):
        for rows in (1, 3, 16):
            yield rows, 4096, k, projection, True, "wide"
    # These additionally cover both other TP8 column-width division paths.
    yield 3, 512, 4096, 1, False, "wide"
    yield 3, 1536, 4096, 4, False, "wide"


def make_fixture(core, spec):
    rows, n, k, projection, partial, pattern = spec
    base, span = (122, 5) if pattern == "mixed" else (117, 15)
    activation = finite_bits(rows * k, 0x12340000 + rows * 17 + k, base + 1, span)
    weights = finite_bits(n * k, 0x98760000 + n * 13 + k, base - 2, span)
    element_bytes = 4 if partial else 2
    output = b"\xA5" * (16 * n * element_bytes)
    return {
        "name": f"{'partial' if partial else 'column'}_n{n}_k{k}_rows{rows}_{pattern}",
        "buffers": [core.buffer("a", activation, 2), core.buffer("weights", weights, 2),
                    core.buffer("output", output, element_bytes, output)],
        "scalars": [rows, n, k, 8, projection], "partial": partial,
    }


def capture(core, worker, case, symbol, groups, artifact, artifact_hash):
    loaded, payload = worker.command({"op": "load_kernel", "payload_bytes": len(artifact),
                                     "object_sha256": list(bytes.fromhex(artifact_hash)),
                                     "symbol": symbol}, artifact, "loaded_kernel")
    core.require(not payload, "load response payload")
    metadata = loaded["metadata"]
    core.require(metadata["symbol"] == symbol
                 and metadata["object_sha256"] == list(bytes.fromhex(artifact_hash)), "artifact identity")
    core.require(metadata["wavefront_size"] == 64 and metadata["private_segment_bytes"] == 0
                 and metadata["group_segment_bytes"] == 0 and metadata["kernarg_alignment"] == 8,
                 "unexpected kernel resource requirements")
    args = metadata["explicit_arguments"]
    core.require(len(args) == 11, "projection ABI argument count")
    core.require(metadata["kernarg_bytes"] == 328 and metadata["implicit_argument_offset"] == 72
                 and metadata["implicit_argument_bytes"] == 256, "projection ABI byte layout")
    encoded = bytearray(328)
    pointers, allocated = [], []
    for index, record in enumerate(case["buffers"]):
        offset = index * 16
        pointer, count = args[index * 2:index * 2 + 2]
        core.require(core.pointer_matches_source(pointer, offset, record), "pointer metadata")
        core.require(count["offset"] == offset + 8 and count["bytes"] == 8
                     and not count["global_buffer"], "slice metadata")
        identifier = worker.allocate(core.GUARD + record["data"] + core.GUARD)
        core.require(identifier not in allocated, "allocation ID reused")
        allocated.append(identifier)
        struct.pack_into("<Q", encoded, offset + 8, len(record["data"]) // record["element_bytes"])
        pointers.append({"kernarg_offset": offset, "buffer": identifier,
                         "buffer_offset": len(core.GUARD), "extent_bytes": len(record["data"]),
                         "access": record["access"]})
    for index, (argument, value) in enumerate(zip(args[6:], case["scalars"], strict=True)):
        offset = 48 + index * 4
        core.require(argument["offset"] == offset and argument["bytes"] == 4
                     and not argument["global_buffer"], "scalar metadata")
        struct.pack_into("<I", encoded, offset, value)
    result, payload = worker.command({"op": "dispatch", "kernel": loaded["kernel"],
                                     "payload_bytes": len(encoded), "workgroup": [64, 1, 1],
                                     "grid": [groups * 64, 1, 1], "pointers": pointers,
                                     "timeout_ms": 30000}, bytes(encoded), "dispatched")
    core.require(not payload and result["elapsed_ns"] > 0, "dispatch completion")
    output = None
    for identifier, record in zip(allocated, case["buffers"], strict=True):
        guarded = worker.read(identifier, len(record["data"]) + 2 * len(core.GUARD))
        core.require(guarded[:len(core.GUARD)] == core.GUARD
                     and guarded[-len(core.GUARD):] == core.GUARD, "slice guard changed")
        actual = guarded[len(core.GUARD):-len(core.GUARD)]
        if record["access"] == "read":
            core.require(actual == record["data"], "immutable projection input changed")
        else:
            rows, n = case["scalars"][:2]
            active = rows * n * record["element_bytes"]
            core.require(actual[active:] == record["data"][active:], "inactive output tail changed")
            output = actual[:active]
        worker.command({"op": "free", "buffer": identifier}, expected="freed")
    core.require(output is not None, "missing projection output")
    return output, {"symbol": symbol, "elapsed_ns": result["elapsed_ns"], "metadata": metadata,
                    "output_sha256": core.digest(output), "guards_unchanged": True,
                    "inputs_unchanged": True, "inactive_tail_unchanged": True}


def reference_dots(left, right):
    serial, partials = 0.0, [0.0] * 64
    products = [f32(a * b) for a, b in zip(left, right, strict=True)]
    for index, product in enumerate(products):
        serial = f32(serial + product)
        partials[index % 64] = f32(partials[index % 64] + product)
    # Exact fe2o3 LLVM ReduceF32 lowering: XOR distances 1,2,4,8,16,32.
    for distance in (1, 2, 4, 8, 16, 32):
        previous = partials
        partials = [f32(previous[lane] + previous[lane ^ distance]) for lane in range(64)]
    return serial, partials[0], math.fsum(products), math.fsum(abs(value) for value in products)


def output_values(data, partial):
    if partial:
        return [value for (value,) in struct.iter_unpack("<f", data)]
    return unpack_bf16(data)


def compare(core, fixture, probe, baseline, wave):
    rows, n, k = fixture["scalars"][:3]
    partial = fixture["partial"]
    baseline_values, wave_values = output_values(baseline, partial), output_values(wave, partial)
    core.require(all(math.isfinite(value) for value in baseline_values + wave_values), "nonfinite output")
    element_bytes = 4 if partial else 2
    differences = [abs(a - b) for a, b in zip(baseline_values, wave_values, strict=True)]
    changed = sum(baseline[index:index + element_bytes] != wave[index:index + element_bytes]
                  for index in range(0, len(baseline), element_bytes))
    columns = sorted({0, 1, 15, 16, n // 2, n - 17, n - 2, n - 1})
    weights = {column: unpack_bf16(fixture["buffers"][1]["data"][column * k * 2:(column + 1) * k * 2])
               for column in columns}
    samples = []
    for row in range(rows):
        left = unpack_bf16(fixture["buffers"][0]["data"][row * k * 2:(row + 1) * k * 2])
        for column in columns:
            serial, cooperative, precise, sum_abs = reference_dots(left, weights[column])
            encode = (lambda value: struct.pack("<f", value)) if partial else (
                lambda value: struct.pack("<H", probe.bf16_bits(value)))
            offset = (row * n + column) * element_bytes
            actual_baseline = baseline[offset:offset + element_bytes]
            actual_wave = wave[offset:offset + element_bytes]
            samples.append({"row": row, "column": column, "fp64_reference": precise,
                            "sum_abs_products": sum_abs,
                            "baseline_fp32_reference": serial, "wave_fp32_reference": cooperative,
                            "baseline_value": baseline_values[row * n + column],
                            "wave_value": wave_values[row * n + column],
                            "baseline_matches_own_order": actual_baseline == encode(serial),
                            "wave_matches_own_order": actual_wave == encode(cooperative),
                            "baseline_abs_error": abs(baseline_values[row * n + column] - precise),
                            "wave_abs_error": abs(wave_values[row * n + column] - precise)})
    return {"name": fixture["name"], "scalars": fixture["scalars"], "elements": rows * n,
            "different_output_bits": changed, "max_abs_difference": max(differences),
            "rms_difference": math.sqrt(math.fsum(value * value for value in differences) / len(differences)),
            "relative_l2_difference": math.sqrt(math.fsum(value * value for value in differences)
                                                / max(math.fsum(value * value for value in baseline_values), 1e-300)),
            "sampled_reference_columns": columns, "samples": samples,
            "sampled_own_order_pass": all(sample["baseline_matches_own_order"]
                                          and sample["wave_matches_own_order"] for sample in samples)}


def self_test(core, probe):
    core.require(len(list(fixture_specs())) == 14, "differential shape roster")
    core.require(finite_bits(64, 7, 117, 15) == finite_bits(64, 7, 117, 15), "nondeterministic fixtures")
    values = unpack_bf16(finite_bits(256, 11, 117, 15))
    core.require(all(math.isfinite(value) and value != 0 for value in values), "invalid finite fixture")
    serial, wave, precise, _ = reference_dots([1.0] * 128, [1.0] * 128)
    core.require((serial, wave, precise) == (128.0, 128.0, 128.0), "reference accumulation")
    left = [1.0] * 128
    right = [0.0] * 128
    right[0], right[1], right[64], right[65] = 2.0 ** 24, 1.0, -(2.0 ** 24), 1.0
    serial, wave, precise, _ = reference_dots(left, right)
    core.require((serial, wave, precise) == (1.0, 2.0, 2.0), "cancellation/order diagnostic lost")
    core.require(probe.bf16_bits(1.00390625) == 0x3F80, "BF16 ties-to-even reference")
    print("PASS: 14 differential shapes and arithmetic references; no GPU launched")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--probe", type=Path, default=Path(__file__).with_name("probe.py"))
    parser.add_argument("--helper", type=Path)
    parser.add_argument("--worker", type=Path)
    parser.add_argument("--worker-sha256")
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--artifact-sha256")
    parser.add_argument("--device-unique-id", type=int)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    probe = load_probe(args.probe)
    helper = args.helper or Path(__file__).resolve().parents[3] / "proofs/tensor-parallel-kernels-v1/probe.py"
    core = probe.load_helper(helper)
    self_test(core, probe)
    if args.self_test:
        core.require(not args.run, "self-test cannot run hardware")
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
        for spec in fixture_specs():
            fixture = make_fixture(core, spec)
            rows, n = fixture["scalars"][:2]
            baseline_symbol = BASELINE_PARTIAL if fixture["partial"] else BASELINE_COLUMN
            wave_symbol = WAVE_PARTIAL if fixture["partial"] else WAVE_COLUMN
            baseline, baseline_receipt = capture(core, worker, fixture, baseline_symbol, n // 16,
                                                  artifact, args.artifact_sha256)
            wave, wave_receipt = capture(core, worker, fixture, wave_symbol, rows * n,
                                         artifact, args.artifact_sha256)
            result = compare(core, fixture, probe, baseline, wave)
            result["dispatches"] = [baseline_receipt, wave_receipt]
            result["input_sha256"] = [core.digest(record["data"]) for record in fixture["buffers"][:2]]
            results.append(result)
            with (args.output / (fixture["name"] + ".json")).open("x", encoding="utf-8") as output:
                json.dump(result, output, sort_keys=True, indent=2, allow_nan=False)
                output.write("\n")
            print(f"{fixture['name']}: changed={result['different_output_bits']}/{rows * n} "
                  f"own-order-pass={result['sampled_own_order_pass']}", flush=True)
        worker.finish()
        core.require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                     and core.identity(os.fstat(artifact_fd)) == core.identity(artifact_stat), "held identity drifted")
        core.require(source_identity == core.digest(Path(__file__).read_bytes()), "source drifted")
        passed = all(result["sampled_own_order_pass"] for result in results)
        report = {"schema": "FerricTpProjectionDifferentialV3", "authority": "none",
                  "model_inference": False, "benchmark": False, "model_parity_qualified": False,
                  "source_sha256": source_identity, "probe_sha256": PROBE_SHA256,
                  "helper_sha256": probe.HELPER_SHA256, "worker_sha256": args.worker_sha256,
                  "artifact_sha256": args.artifact_sha256, "device_unique_id": args.device_unique_id,
                  "worker_pid": worker.process.pid, "worker_start_ticks": worker.start,
                  "timing_scope": "single synchronous worker dispatch, diagnostic only",
                  "reference_scope": "all outputs compared; eight reference columns per active row",
                  "clean_teardown": True, "sampled_own_order_pass": passed, "results": results}
        with (args.output / "result.json").open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2, allow_nan=False)
            output.write("\n")
        core.require(passed, "GPU outputs differ from their sampled exact accumulation-order references")
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
