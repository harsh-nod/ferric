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
PREFIX = "ferric_qwen3_tp_batch_"
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


def baseline_cases(core):
    bf16, buffer = core.bf16, core.buffer
    column_weight = (bf16(0x3F80, 1) + bf16(0, 4095)) * 512
    partial_weight = (bf16(0x3F80, 1) + bf16(0x3B80, 1) + bf16(0, 510)) * 4096
    for rows in (1, 3, 16):
        values = [float(row + 1) for row in range(rows)]
        column_input = b"".join(pack_bf16([value] * 4096) for value in values)
        column_expected = b"".join(pack_bf16([value] * 512) for value in values)
        column_tail = bf16(SENTINEL, (16 - rows) * 512)
        yield {
            "name": f"column_rows_{rows}", "symbol": PREFIX + "gemm_bf16_f32_bf16_v2",
            "buffers": [buffer("a", column_input, 2), buffer("weights", column_weight, 2),
                        buffer("output", bf16(SENTINEL, 16 * 512), 2, column_expected + column_tail)],
            "scalars": [rows, 512, 4096, 8, 1], "groups": 32,
        }
        partial_input = b"".join(pack_bf16([value] * 512) for value in values)
        partial_expected = b"".join(struct.pack("<f", value * 1.00390625) * 4096 for value in values)
        partial_tail = b"\xA5" * ((16 - rows) * 4096 * 4)
        yield {
            "name": f"partial_rows_{rows}", "symbol": PREFIX + "gemm_partial_bf16_f32_v2",
            "buffers": [buffer("a", partial_input, 2), buffer("weights", partial_weight, 2),
                        buffer("output", b"\xA5" * (16 * 4096 * 4), 4, partial_expected + partial_tail)],
            "scalars": [rows, 4096, 512, 8, 1], "groups": 256,
        }
    yield {
        "name": "swiglu_rows_3", "symbol": PREFIX + "swiglu_bf16_f32_v2",
        "buffers": [buffer("gate", bf16(0, 3 * 1536), 2),
                    buffer("up", pack_bf16([1.0, 2.0, 3.0]) * 1536, 2),
                    buffer("output", bf16(SENTINEL, 16 * 1536), 2,
                           bf16(0, 3 * 1536) + bf16(SENTINEL, 13 * 1536))],
        "scalars": [3, 8], "groups": 72,
    }
    query_rows, key_rows, rotated_queries, rotated_keys = [], [], [], []
    for row, (cosine, sine) in enumerate(((1.0, 0.0), (0.0, 1.0), (-1.0, 0.0))):
        first, second = float(row + 1), float(row + 4)
        query_rows.append(pack_bf16([first] * 64 + [second] * 64) * 4)
        key_rows.append(pack_bf16([-first] * 64 + [second] * 64))
        rotated_queries.append(pack_bf16([first * cosine - second * sine] * 64
                                       + [second * cosine + first * sine] * 64) * 4)
        rotated_keys.append(pack_bf16([-first * cosine - second * sine] * 64
                                    + [second * cosine - first * sine] * 64))
    yield {
        "name": "rope_rows_distinct_positions", "symbol": PREFIX + "rope_v2",
        "buffers": [buffer("query", b"".join(query_rows), 2), buffer("key", b"".join(key_rows), 2),
                    buffer("cos", b"".join(struct.pack("<f", x) * 64 for x in (1.0, 0.0, -1.0)), 4),
                    buffer("sin", b"".join(struct.pack("<f", x) * 64 for x in (0.0, 1.0, 0.0)), 4),
                    buffer("positions", pack_u32([0, 15, 16]), 4),
                    buffer("rotated_query", bf16(SENTINEL, 16 * 512), 2,
                           b"".join(rotated_queries) + bf16(SENTINEL, 13 * 512)),
                    buffer("rotated_key", bf16(SENTINEL, 16 * 128), 2,
                           b"".join(rotated_keys) + bf16(SENTINEL, 13 * 128))],
        "scalars": [3, 8], "groups": 3,
    }
    key = b"".join(pack_bf16([float(row + 1)] * 128) for row in range(3))
    value = b"".join(pack_bf16([float(row + 4)] * 128) for row in range(3))
    initial = bf16(SENTINEL, 4 * 16 * 128)
    expected_keys, expected_values = bytearray(initial), bytearray(initial)
    for row, slot in enumerate((47, 16, 0)):
        start = slot * 128 * 2
        expected_keys[start:start + 256] = key[row * 256:(row + 1) * 256]
        expected_values[start:start + 256] = value[row * 256:(row + 1) * 256]
    yield {
        "name": "append_page_boundary_preserves_pool", "symbol": PREFIX + "paged_kv_append_v2",
        "buffers": [buffer("key", key, 2), buffer("value", value, 2),
                    buffer("positions", pack_u32([15, 16, 0]), 4),
                    buffer("page_table", pack_u32([2, 0xFFFFFFFF, 2, 1, 0, 0xFFFFFFFF]), 4),
                    buffer("key_cache", initial, 2, expected_keys),
                    buffer("value_cache", initial, 2, expected_values)],
        "scalars": [3, 8, 2, 4], "groups": 1,
    }
    query = bytearray(bf16(0, 2 * 512))
    keys = bytearray(bf16(0x7FC0, 4 * 16 * 128))
    values = bytearray(keys)
    table = [2, 0xFFFFFFFF, 1, 3]
    outputs = []
    scale = struct.unpack("<f", struct.pack("<I", 0x3DB504F3))[0]
    for row, position in ((0, 1), (1, 16)):
        q = 1.0 if row == 0 else -1.5
        for head in range(4):
            struct.pack_into("<H", query, (row * 512 + head * 128) * 2, bf16_bits(q))
        scores, dense_values = [], []
        for token in range(position + 1):
            base = (table[row * 2 + token // 16] * 16 + token % 16) * 128 * 2
            k = token * 8.0 if row == 0 else (token - 8) * 0.25
            v = 1.0 + token * 2.0 if row == 0 else token * 0.0625
            keys[base:base + 256] = bf16(0, 128)
            struct.pack_into("<H", keys, base, bf16_bits(k))
            values[base:base + 256] = pack_bf16([v] * 128)
            scores.append(q * k * scale)
            dense_values.append(v)
        maximum = max(scores)
        weights = [math.exp(score - maximum) for score in scores]
        expected = sum(w * v for w, v in zip(weights, dense_values, strict=True)) / sum(weights)
        outputs.append(pack_bf16([expected] * 512))
    yield {
        "name": "attention_nonzero_qk_mixed_positions", "symbol": PREFIX + "paged_gqa_bf16_f32_v2",
        "buffers": [buffer("query", query, 2), buffer("key_cache", keys, 2),
                    buffer("value_cache", values, 2), buffer("positions", pack_u32([1, 16]), 4),
                    buffer("page_table", pack_u32(table), 4),
                    buffer("output", bf16(SENTINEL, 16 * 512), 2,
                           b"".join(outputs) + bf16(SENTINEL, 14 * 512))],
        "scalars": [2, 8, 2, 4, 17], "groups": 8,
    }
    logits = bytearray(bf16(0xBF80, 3 * 151936))
    choices = [0, 151935, 17]
    for row, choice in enumerate(choices):
        struct.pack_into("<H", logits, (row * 151936 + choice) * 2, 0x4000)
    struct.pack_into("<H", logits, (2 * 151936 + 18) * 2, 0x4000)
    yield {
        "name": "argmax_rows_distinct_and_lowest_tie", "symbol": PREFIX + "argmax_bf16_v2",
        "buffers": [buffer("logits", logits, 2),
                    buffer("choices", pack_u32([0xFFFFFFFF] * 16), 4,
                           pack_u32(choices + [0xFFFFFFFF] * 13))],
        "scalars": [3], "groups": 3,
    }



def transpose_bf16(data, rows, columns):
    source = memoryview(data)
    result = bytearray(len(data))
    for row in range(rows):
        for column in range(columns):
            result[(column * rows + row) * 2:(column * rows + row + 1) * 2] = source[
                (row * columns + column) * 2:(row * columns + column + 1) * 2]
    return bytes(result)


def cases(core):
    for original in baseline_cases(core):
        yield original
        if original["name"].startswith(("column_rows_", "partial_rows_")):
            partial = original["name"].startswith("partial_")
            for mode in ("wave", "mfma"):
                case = dict(original)
                case["buffers"] = [dict(record) for record in original["buffers"]]
                rows, n, k, _, _ = case["scalars"]
                case["name"] = mode + "_" + original["name"]
                case["symbol"] = ("ferric_qwen3_tp_" + mode
                                  + ("_gemv" if mode == "wave" else "_gemm")
                                  + ("_partial_f32_v3" if partial else "_bf16_v3"))
                case["groups"] = rows * n if mode == "wave" else n // 16
                if mode == "mfma":
                    transposed = transpose_bf16(case["buffers"][1]["data"], n, k)
                    case["buffers"][1]["data"] = transposed
                    case["buffers"][1]["expected"] = transposed
                yield case
        elif original["name"].startswith("attention_"):
            for dense in (False, True):
                case = dict(original)
                case["buffers"] = [dict(record) for record in original["buffers"]]
                case["name"] = "wave_" + original["name"] + ("_dense" if dense else "")
                case["symbol"] = "ferric_qwen3_tp_wave_paged_gqa_bf16_v3"
                if dense:
                    query = bytearray(case["buffers"][0]["data"])
                    keys = bytearray(case["buffers"][1]["data"])
                    table = [2, 0xFFFFFFFF, 1, 3]
                    for row, position in ((0, 1), (1, 16)):
                        q = 1.0 if row == 0 else -1.5
                        for head in range(4):
                            offset = (row * 512 + head * 128) * 2
                            query[offset:offset + 256] = pack_bf16([q / 128.0] * 128)
                        for token in range(position + 1):
                            offset = (table[row * 2 + token // 16] * 16 + token % 16) * 256
                            k = token * 8.0 if row == 0 else (token - 8) * 0.25
                            keys[offset:offset + 256] = pack_bf16([k] * 128)
                    for index, data in ((0, query), (1, keys)):
                        case["buffers"][index]["data"] = bytes(data)
                        case["buffers"][index]["expected"] = bytes(data)
                yield case
    for rows in (1, 3, 16):
        partial = b"".join(struct.pack("<f", (row + 1) * 1.00390625) * 4096
                           for row in range(rows))
        residual = pack_bf16([0.5] * (rows * 4096))
        output = b"".join(pack_bf16([(row + 1) * 1.00390625 + 0.5] * 4096)
                          for row in range(rows))
        yield {
            "name": f"residual_rows_{rows}",
            "symbol": "ferric_qwen3_tp_batch_residual_bf16_v3",
            "buffers": [core.buffer("partial", partial, 4), core.buffer("residual", residual, 2),
                        core.buffer("output", core.bf16(SENTINEL, rows * 4096), 2, output)],
            "scalars": [rows], "groups": rows * 64,
        }
    for partial, n, k, projection in ((False, 512, 4096, 1), (True, 4096, 512, 1)):
        weights = b"".join(pack_bf16([((column % 7) - 3) * (d - 7) / 64.0
                                      for d in range(16)]) * (k // 16)
                           for column in range(n))
        transposed = transpose_bf16(weights, n, k)
        for rows in (3, 16):
            activations = b"".join(pack_bf16([(row + 1) * (d - 5) / 64.0
                                             for d in range(16)]) * (k // 16)
                                  for row in range(rows))
            values = [(row + 1) * ((column % 7) - 3) * (k // 16)
                      * sum((d - 5) * (d - 7) for d in range(16)) / 4096.0
                      for row in range(rows) for column in range(n)]
            expected = (b"".join(struct.pack("<f", value) for value in values)
                        if partial else pack_bf16(values))
            element_bytes = 4 if partial else 2
            tail = b"\xA5" * ((16 - rows) * n * element_bytes)
            for mode in ("wave", "mfma"):
                yield {
                    "name": f"{mode}_dense_{'partial' if partial else 'column'}_rows_{rows}",
                    "symbol": ("ferric_qwen3_tp_" + mode
                               + ("_gemv" if mode == "wave" else "_gemm")
                               + ("_partial_f32_v3" if partial else "_bf16_v3")),
                    "buffers": [core.buffer("a", activations, 2),
                                core.buffer("weights", weights if mode == "wave" else transposed, 2),
                                core.buffer("output", b"\xA5" * (16 * n * element_bytes),
                                            element_bytes, expected + tail)],
                    "scalars": [rows, n, k, 8, projection],
                    "groups": rows * n if mode == "wave" else n // 16,
                }


def self_test(core):
    fixtures = list(cases(core))
    core.require(len(fixtures) == 36 and len({case["symbol"] for case in fixtures}) == 13, "probe roster")
    for case in fixtures:
        for record in case["buffers"]:
            core.require(0 < len(record["data"]) <= core.LIMIT
                         and len(record["data"]) == len(record["expected"]), "fixture extent")
            core.require(len(record["data"]) % record["element_bytes"] == 0, "fixture units")
    partial = next(case for case in fixtures if case["name"] == "partial_rows_1")
    core.require(struct.unpack("<f", partial["buffers"][2]["expected"][:4])[0] == 1.00390625,
                 "partial fixture lost FP32 precision")
    return fixtures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--run", action="store_true")
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
    if args.self_test:
        core.require(not args.run, "self-test cannot launch GPU work")
        print("PASS: 36 bounded host fixtures; no GPU worker launched")
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
        report = {"schema": "FerricTpPerfSyntheticKernelProbeV3", "authority": "none",
                  "model_inference": False, "benchmark": False, "source_sha256": source_identity,
                  "helper_sha256": HELPER_SHA256, "worker_sha256": args.worker_sha256,
                  "artifact_sha256": args.artifact_sha256, "device_unique_id": args.device_unique_id,
                  "worker_pid": worker.process.pid, "worker_start_ticks": worker.start,
                  "timing_scope": "worker synchronous dispatch wall-clock nanoseconds",
                  "unexecuted_roots": [PREFIX + "embedding_bf16_v2", "qwen3_rmsnorm_v1"],
                  "clean_teardown": True, "results": results}
        with (args.output / "result.json").open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2)
            output.write("\n")
        print("PASS: 36 GPU probes across 13 roots; no model/benchmark/qualification claim")
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
