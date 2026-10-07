#!/usr/bin/env python3
"""Explicit engineering-only multi-row TP synthetic GPU probes, not a benchmark."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import stat
import struct
import types


HELPER_SHA256 = "a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b"
PREFIX = "ferric_qwen3_tp_batch32_"
SENTINEL = 0x3555


def load_helper(path):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 128 * 1024:
            raise RuntimeError("invalid helper file")
        chunks, size = [], 0
        while chunk := os.read(fd, 128 * 1024 - size + 1):
            size += len(chunk)
            if size > 128 * 1024:
                raise RuntimeError("probe helper grew beyond byte limit")
            chunks.append(chunk)
        data = b"".join(chunks)
        after = os.fstat(fd)
        fields = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if (any(getattr(before, field) != getattr(after, field) for field in fields)
                or hashlib.sha256(data).hexdigest() != HELPER_SHA256):
            raise RuntimeError("probe helper identity mismatch")
    finally:
        os.close(fd)
    module = types.ModuleType("ferric_tp_probe_verified_helper")
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def bf16_bits(value):
    bits = struct.unpack("<I", struct.pack("<f", value))[0]
    if not math.isfinite(value):
        raise ValueError("finite fixture value required")
    return ((bits + 0x7FFF + ((bits >> 16) & 1)) >> 16) & 0xFFFF


def pack_bf16(values):
    return b"".join(struct.pack("<H", bf16_bits(value)) for value in values)


def pack_u32(values):
    return b"".join(struct.pack("<I", value) for value in values)


def transpose_bf16(data, rows, columns):
    source = memoryview(data)
    result = bytearray(len(data))
    for row in range(rows):
        for column in range(columns):
            result[(column * rows + row) * 2:(column * rows + row + 1) * 2] = source[
                (row * columns + column) * 2:(row * columns + column + 1) * 2]
    return bytes(result)


def cases(core):
    buffer = core.buffer
    for rows in (17, 31, 32):
        for partial, n, k, projection in ((False, 512, 4096, 1), (True, 4096, 512, 1)):
            row_values = [float(row + 1) for row in range(rows)]
            a = b"".join(pack_bf16([value] * k) for value in row_values)
            weights = ((pack_bf16([1.0, 1.0 / 256.0]) + core.bf16(0, k - 2)) * n
                       if partial else (pack_bf16([1.0]) + core.bf16(0, k - 1)) * n)
            element_bytes = 4 if partial else 2
            expected = (b"".join(struct.pack("<f", value * 1.00390625) * n for value in row_values)
                        if partial else b"".join(pack_bf16([value] * n) for value in row_values))
            tail = b"\xA5" * ((32 - rows) * n * element_bytes)
            for mode in ("baseline", "wave", "mfma"):
                suffix = (("gemm_partial_bf16_f32_v5" if partial else "gemm_bf16_f32_bf16_v5")
                          if mode == "baseline" else mode
                          + ("_gemv" if mode == "wave" else "_gemm")
                          + ("_partial_f32_v5" if partial else "_bf16_v5"))
                rhs = transpose_bf16(weights, n, k) if mode == "mfma" else weights
                yield {"name": f"{mode}_{'partial' if partial else 'column'}_rows_{rows}",
                       "symbol": PREFIX + suffix,
                       "buffers": [buffer("a", a, 2), buffer("weights", rhs, 2),
                                   buffer("output", b"\xA5" * (32 * n * element_bytes),
                                          element_bytes, expected + tail)],
                       "scalars": [rows, n, k, 8, projection],
                       "groups": rows * n if mode == "wave" else ((rows + 15) // 16) * (n // 16)}
        pages = 64
        cache = bytearray(core.bf16(SENTINEL, pages * 16 * 128))
        tables, expected = [], []
        for row in range(rows):
            first_page = 2 * (31 - row)
            tables.extend([first_page, first_page + 1])
            values = pack_bf16([(row % 8 + 1) / 2.0 + dimension / 256.0 for dimension in range(128)])
            for page in (first_page, first_page + 1):
                cache[page * 16 * 256:(page + 1) * 16 * 256] = values * 16
            expected.append(values * 4)
        query = core.bf16(0, rows * 512)
        output = b"".join(expected) + core.bf16(SENTINEL, (32 - rows) * 512)
        for mode in ("baseline", "wave"):
            yield {"name": f"{mode}_attention_rows_{rows}_causal_pages",
                   "symbol": PREFIX + ("paged_gqa_bf16_f32_v5" if mode == "baseline"
                                       else "wave_paged_gqa_bf16_v5"),
                   "buffers": [buffer("query", query, 2),
                               buffer("keys", core.bf16(0, pages * 16 * 128), 2),
                               buffer("values", bytes(cache), 2),
                               buffer("positions", pack_u32([row % 17 for row in range(rows)]), 4),
                               buffer("table", pack_u32(tables), 4),
                               buffer("output", core.bf16(SENTINEL, 32 * 512), 2, output)],
                   "scalars": [rows, 8, 2, pages, 17], "groups": rows * 4}
    rows = 32
    yield {"name": "swiglu_rows_32", "symbol": PREFIX + "swiglu_bf16_f32_v5",
           "buffers": [buffer("gate", core.bf16(0, rows * 1536), 2),
                       buffer("up", pack_bf16([1.0, 2.0]) * (rows * 768), 2),
                       buffer("output", core.bf16(SENTINEL, rows * 1536), 2,
                              core.bf16(0, rows * 1536))],
           "scalars": [rows, 8], "groups": rows * 24}
    query = b"".join(pack_bf16([float(row + 1)] * 512) for row in range(rows))
    key = b"".join(pack_bf16([-float(row + 1)] * 128) for row in range(rows))
    yield {"name": "rope_rows_32_distinct_positions", "symbol": PREFIX + "rope_v5",
           "buffers": [buffer("query", query, 2), buffer("key", key, 2),
                       buffer("cos", struct.pack("<f", 1.0) * (rows * 64), 4),
                       buffer("sin", struct.pack("<f", 0.0) * (rows * 64), 4),
                       buffer("positions", pack_u32([row * 7 for row in range(rows)]), 4),
                       buffer("rotated_query", core.bf16(SENTINEL, rows * 512), 2, query),
                       buffer("rotated_key", core.bf16(SENTINEL, rows * 128), 2, key)],
           "scalars": [rows, 8], "groups": rows}
    value = b"".join(pack_bf16([float(row + 1)] * 128) for row in range(rows))
    initial = core.bf16(SENTINEL, 4 * 16 * 128)
    keys, values = bytearray(initial), bytearray(initial)
    for row in range(rows):
        slot = (3 if row < 16 else 1) * 16 + row % 16
        keys[slot * 256:(slot + 1) * 256] = key[row * 256:(row + 1) * 256]
        values[slot * 256:(slot + 1) * 256] = value[row * 256:(row + 1) * 256]
    yield {"name": "append_rows_32_distinct_slots_cross_page", "symbol": PREFIX + "paged_kv_append_v5",
           "buffers": [buffer("key", key, 2), buffer("value", value, 2),
                       buffer("positions", pack_u32(range(rows)), 4),
                       buffer("table", pack_u32([3, 1] * rows), 4),
                       buffer("key_cache", initial, 2, bytes(keys)),
                       buffer("value_cache", initial, 2, bytes(values))],
           "scalars": [rows, 8, 2, 4], "groups": 1}
    logits = bytearray(core.bf16(0, rows * 151936))
    choices = []
    for row in range(rows):
        winner = row * 53 + 7
        choices.append(winner)
        for token in (winner, winner + 2):
            struct.pack_into("<H", logits, (row * 151936 + token) * 2, bf16_bits(1.0))
    yield {"name": "argmax_rows_32_lowest_tie", "symbol": PREFIX + "argmax_bf16_v5",
           "buffers": [buffer("logits", bytes(logits), 2),
                       buffer("choices", pack_u32([0xFFFFFFFF] * rows), 4, pack_u32(choices))],
           "scalars": [rows], "groups": rows}
    partial = struct.pack("<f", 1.00390625) * (rows * 4096)
    yield {"name": "residual_rows_32_single_round", "symbol": PREFIX + "residual_bf16_v5",
           "buffers": [buffer("partial", partial, 4),
                       buffer("residual", pack_bf16([0.5]) * (rows * 4096), 2),
                       buffer("output", core.bf16(SENTINEL, rows * 4096), 2,
                              pack_bf16([1.50390625]) * (rows * 4096))],
           "scalars": [rows], "groups": rows * 64}
    ones = pack_bf16([1.0]) * (rows * 4096)
    yield {"name": "rmsnorm_rows_32", "symbol": "qwen3_rmsnorm_v1",
           "buffers": [buffer("input", ones, 2), buffer("residual", b"", 2),
                       buffer("weight", pack_bf16([1.0]) * 4096, 2),
                       buffer("fused", b"", 2, b""),
                       buffer("normalized", core.bf16(SENTINEL, rows * 4096), 2, ones)],
           "scalars": [rows, 4096, 0x358637BD, 0], "groups": rows}


def self_test(core):
    fixtures = list(cases(core))
    core.require(len(fixtures) == 30 and len({case["symbol"] for case in fixtures}) == 14,
                 "closed batch32 fixture roster")
    for case in fixtures:
        core.require(0 < case["groups"] <= 32 * 4096, "bounded fixture launch")
        for record in case["buffers"]:
            core.require(0 <= len(record["data"]) <= 16 * 1024 * 1024
                         and len(record["data"]) == len(record["expected"]), "bounded fixture extent")
            core.require(len(record["data"]) % record["element_bytes"] == 0, "fixture element units")
    core.require({case["scalars"][0] for case in fixtures} == {17, 31, 32}, "row boundary coverage")
    return fixtures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--profile", choices=("full", "wave"), default="full")
    parser.add_argument("--helper", type=Path)
    parser.add_argument("--worker", type=Path)
    parser.add_argument("--worker-sha256")
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--artifact-sha256")
    parser.add_argument("--device-unique-id", type=int)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    helper = args.helper
    if helper is None:
        parents = Path(__file__).resolve().parents
        if len(parents) < 4:
            parser.error("--helper is required outside the Ferric source tree")
        helper = parents[3] / "proofs/tensor-parallel-kernels-v1/probe.py"
    core = load_helper(helper)
    fixtures = self_test(core)
    if args.profile == "wave":
        fixtures = [case for case in fixtures if "_mfma_" not in case["symbol"]]
        core.require(len(fixtures) == 24 and len({case["symbol"] for case in fixtures}) == 12,
                     "wave-only probe roster")
    if args.self_test:
        core.require(not args.run, "self-test cannot launch GPU work")
        print(f"PASS: {len(fixtures)} bounded host fixtures ({args.profile}); no GPU worker launched")
        return
    core.require(args.run and all(value is not None for value in
                 (args.worker, args.worker_sha256, args.artifact, args.artifact_sha256,
                  args.device_unique_id, args.output)), "explicit run and all identities required")
    core.require(0 < args.device_unique_id < 1 << 64 and args.output.is_absolute(), "invalid run destination")
    args.output.mkdir(mode=0o700)
    source_identity = core.digest(Path(__file__).read_bytes())
    worker_fd, worker_stat, _ = core.held_file(args.worker, args.worker_sha256, 512 * 1024 * 1024, 62)
    artifact_fd, worker = None, None
    try:
        artifact_fd, artifact_stat, artifact = core.held_file(args.artifact, args.artifact_sha256, 64 * 1024 * 1024, 224)
        worker = core.Worker(worker_fd, worker_stat, args.device_unique_id, args.output)
        results = [core.probe(worker, case, artifact, args.artifact_sha256) for case in fixtures]
        worker.finish()
        core.require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                     and core.identity(os.fstat(artifact_fd)) == core.identity(artifact_stat), "held identity drifted")
        core.require(source_identity == core.digest(Path(__file__).read_bytes()), "probe source drifted")
        report = {"schema": "FerricTpBatch32SyntheticKernelProbeV5", "authority": "none",
                  "profile": args.profile,
                  "model_inference": False, "benchmark": False, "source_sha256": source_identity,
                  "helper_sha256": HELPER_SHA256, "worker_sha256": args.worker_sha256,
                  "artifact_sha256": args.artifact_sha256, "device_unique_id": args.device_unique_id,
                  "worker_pid": worker.process.pid, "worker_start_ticks": worker.start,
                  "timing_scope": "worker synchronous dispatch wall-clock nanoseconds",
                  "unexecuted_roots": [PREFIX + "embedding_bf16_v5"],
                  "clean_teardown": True, "results": results}
        with (args.output / "result.json").open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2)
            output.write("\n")
        print(f"PASS: {len(fixtures)} GPU probes ({args.profile}); no model/benchmark/qualification claim")
    except BaseException:
        if worker is not None:
            worker.abort()
        raise
    finally:
        if artifact_fd is not None:
            os.close(artifact_fd)
        os.close(worker_fd)


if __name__ == "__main__":
    main()
