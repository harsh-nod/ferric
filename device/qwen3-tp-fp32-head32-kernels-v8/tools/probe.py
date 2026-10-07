#!/usr/bin/env python3
"""Fifteen bounded full-vocabulary FP32-head32 fixtures. No model or rate qualification."""
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

HELPER_SHA256 = "a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b"
N, K, CAPACITY = 151936, 4096, 32
POSITIONS = (0, 1, 15, 16, 4095)
ROOTS = ("ferric_qwen3_tp_batch32_head_bf16_f32_v8", "ferric_qwen3_tp_batch32_mfma_head_f32_v8", "ferric_qwen3_tp_batch32_argmax_f32_v8")


def require(value, message):
    if not value:
        raise RuntimeError(message)


def load_helper(path):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 128 * 1024, "helper extent")
        with os.fdopen(os.dup(fd), "rb") as source:
            data = source.read(128 * 1024 + 1)
        after = os.fstat(fd)
        require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
                == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)
                and len(data) == before.st_size and hashlib.sha256(data).hexdigest() == HELPER_SHA256,
                "helper identity drift")
    finally:
        os.close(fd)
    module = types.ModuleType("ferric_fp32_head_verified_fixture_helper")
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def bf16(value):
    bits = struct.unpack("<I", struct.pack("<f", value))[0]
    require(math.isfinite(value) and bits & 0xffff == 0, "fixture value is not exact BF16")
    return bits >> 16


def activation(row, inner):
    if inner == 0:
        return 1.0
    if inner == 1:
        return (row + 1) / 16.0
    return ((inner * 11 + row * 7) % 31 - 15) / 64.0


def coefficients(column):
    if column in (7, 101):
        return (24.25, 0.125 if column == 7 else 0.15625, 0.0, 0.0, 0.0)
    return ((column % 127 - 63) / 128.0, 1 / 64.0,
            (column // 127 % 7 - 3) / 64.0, -1 / 128.0,
            (column // 889 % 11 - 5) / 128.0)


def reference(row, column):
    # Every product/sum is exactly representable in FP32 for this fixture.
    return sum(activation(row, inner) * weight
               for inner, weight in zip(POSITIONS, coefficients(column), strict=True))


def words_bytes(values):
    words = array.array("H", values)
    if sys.byteorder != "little":
        words.byteswap()
    return words.tobytes()


def weights(layout, n=N):
    require(layout in ("nk", "kn") and type(n) is int and 102 <= n <= N, "weight fixture bounds")
    data = bytearray(n * K * 2)
    if layout == "nk":
        for column in range(n):
            for inner, value in zip(POSITIONS, coefficients(column), strict=True):
                struct.pack_into("<H", data, (column * K + inner) * 2, bf16(value))
    else:
        for index, inner in enumerate(POSITIONS):
            data[inner * n * 2:(inner + 1) * n * 2] = words_bytes(
                bf16(coefficients(column)[index]) for column in range(n))
    return bytes(data)


def head_case(core, rows, mfma):
    require(type(rows) is int and 1 <= rows <= CAPACITY and type(mfma) is bool, "head shape")
    inputs = words_bytes(bf16(activation(row, inner)) for row in range(rows) for inner in range(K))
    output = b"\xa5" * (CAPACITY * N * 4)
    expected = b"".join(struct.pack("<f", reference(row, column))
                        for row in range(rows) for column in range(N)) + output[rows * N * 4:]
    return {"name": f"{'mfma' if mfma else 'scalar'}_head_rows{rows}", "symbol": ROOTS[int(mfma)],
            "groups": ((rows + 15) // 16) * (N // 16), "scalars": [rows, N, K, 1, 6],
            "buffers": [core.buffer("a", inputs, 2), core.buffer("weights_kn" if mfma else "weights_nk", weights("kn" if mfma else "nk"), 2),
                        core.buffer("logits", output, 4, expected)]}


def argmax_case(core, rows):
    require(type(rows) is int and 1 <= rows <= CAPACITY, "argmax shape")
    logits = bytearray(struct.pack("<f", float("nan")) * (CAPACITY * N))
    winners = []
    for row in range(rows):
        values = bytearray(struct.pack("<f", -1.0) * N)
        if row % 3 == 0:
            struct.pack_into("<f", values, 7 * 4, 24.375)
            struct.pack_into("<f", values, 101 * 4, 24.40625)
            winner = 101
        elif row % 3 == 1:
            struct.pack_into("<f", values, 17 * 4, -0.0)
            struct.pack_into("<f", values, (N - 1) * 4, 0.0)
            winner = 17
        else:
            struct.pack_into("<f", values, (N - 1) * 4, 2.0)
            winner = N - 1
        logits[row * N * 4:(row + 1) * N * 4] = values
        winners.append(winner)
    choices = b"\xa5" * (CAPACITY * 4)
    expected = b"".join(struct.pack("<I", winner) for winner in winners) + choices[rows * 4:]
    return {"name": f"fp32_argmax_rows{rows}", "symbol": ROOTS[2], "groups": rows, "scalars": [rows],
            "buffers": [core.buffer("logits", logits, 4), core.buffer("choices", choices, 4, expected)]}


def self_test(core):
    n = 128
    nk, kn = weights("nk", n), weights("kn", n)
    for column in range(n):
        for inner in range(K):
            left = struct.unpack_from("<H", nk, (column * K + inner) * 2)[0]
            right = struct.unpack_from("<H", kn, (inner * n + column) * 2)[0]
            require(left == right, "complete small-n real-K transpose parity")
    for rows in (1, 16, 17, 31, 32):
        for row in range(rows):
            for column in range(n):
                total = 0.0
                for inner in POSITIONS:
                    bits = struct.unpack_from("<H", nk, (column * K + inner) * 2)[0]
                    weight = struct.unpack("<f", struct.pack("<I", bits << 16))[0]
                    total = struct.unpack("<f", struct.pack("<f", total + activation(row, inner) * weight))[0]
                require(total == reference(row, column), "independent serial FP32 fixture parity")
            require(max(range(N), key=lambda column: reference(row, column)) == 101, "full-vocab head winner")
        case = argmax_case(core, rows)
        logits, output = case["buffers"]
        for row in range(rows):
            values = struct.unpack_from(f"<{N}f", logits["data"], row * N * 4)
            winner = max(range(N), key=values.__getitem__)
            require(winner == struct.unpack_from("<I", output["expected"], row * 4)[0], "full-vocab argmax reference")
    print("PASS: fifteen native cases specified; full-vocabulary reference winners, 128x4096 full transpose; no GPU")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--run", action="store_true")
    parser.add_argument("--helper", type=Path, required=True)
    parser.add_argument("--worker", type=Path)
    parser.add_argument("--worker-sha256")
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--artifact-sha256")
    parser.add_argument("--device-unique-id", type=int)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--operational", action="store_true")
    args = parser.parse_args()
    core = load_helper(args.helper)
    if args.self_test:
        require(not args.run, "self-test cannot launch GPU work")
        self_test(core)
        return
    require(args.run and all(value is not None for value in (args.worker, args.worker_sha256,
            args.artifact, args.artifact_sha256, args.device_unique_id, args.output)), "explicit run and pins required")
    require(0 < args.device_unique_id < 1 << 64 and args.output.is_absolute(), "run bounds")
    available = next(int(line.split()[1]) * 1024 for line in Path("/proc/meminfo").read_text().splitlines() if line.startswith("MemAvailable:"))
    require(available >= 12 * 1024**3, "reserve at least 12 GiB host headroom for full-weight custody checks")
    args.output.mkdir(mode=0o700)
    source_hash = core.digest(Path(__file__).read_bytes())
    worker_fd, worker_stat, _ = core.held_file(args.worker, args.worker_sha256, 512 * 1024**2, 62)
    artifact_fd, worker = None, None
    try:
        artifact_fd, artifact_stat, artifact = core.held_file(args.artifact, args.artifact_sha256, 64 * 1024**2, 224)
        worker = core.Worker(worker_fd, worker_stat, args.device_unique_id, args.output)
        if args.operational:
            _, payload = worker.command({"op": "configure_performance", "cache_kernel_admission": False,
                                         "operational_currentness": True, "profile": False}, expected="performance_configured")
            require(not payload, "configuration payload")
        results = []
        for rows in (1, 16, 17, 31, 32):
            for mfma in (False, True):
                case = head_case(core, rows, mfma)
                result = core.probe(worker, case, artifact, args.artifact_sha256)
                result["full_output_exact"] = True
                result["active_rows"] = rows
                results.append(result)
                del case
                print(result["name"] + ": PASS", flush=True)
            case = argmax_case(core, rows)
            results.append(core.probe(worker, case, artifact, args.artifact_sha256))
            print(case["name"] + ": PASS", flush=True)
            del case
        worker.finish()
        require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                and core.identity(os.fstat(artifact_fd)) == core.identity(artifact_stat)
                and core.digest(Path(__file__).read_bytes()) == source_hash, "retained identity drift")
        report = {"schema": "FerricTpFp32Head32ProbeV8", "authority": "none", "benchmark": False,
                  "model_inference": False, "model_parity_qualified": False, "checks_pass": True,
                  "source_sha256": source_hash, "helper_sha256": HELPER_SHA256,
                  "worker_sha256": args.worker_sha256, "artifact_sha256": args.artifact_sha256,
                  "runtime_operational": args.operational, "device_unique_id": args.device_unique_id,
                  "worker_pid": worker.process.pid, "worker_start_ticks": worker.start, "clean_teardown": True,
                  "timing_scope": "diagnostic dispatch only, not accepted model performance", "results": results}
        with (args.output / "result.json").open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2, allow_nan=False)
            output.write("\n")
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
