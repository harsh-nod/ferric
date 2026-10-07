#!/usr/bin/env python3
"""Fourteen bounded finite-active FP32 argmax fixtures; no model/rate qualification."""
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
ROOT = "ferric_qwen3_tp_batch32_wave_argmax_f32_v11"
N, CAPACITY = 151936, 32
SENTINEL = b"\xa5" * 4
MIN_SUBNORMAL = struct.unpack("<f", struct.pack("<I", 1))[0]
MAX_FINITE = struct.unpack("<f", struct.pack("<I", 0x7f7fffff))[0]


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
    module = types.ModuleType("ferric_argmax_v11_verified_fixture_helper")
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def specifications():
    for rows in (1, 2, 16, 17, 31, 32):
        yield ("mixed", rows, CAPACITY, 0, 0)
    for step in (0, 1187, 2373):
        for first_lane in (0, 32):
            yield ("lanes", 32, CAPACITY, step, first_lane)
    for rows in (1, 32):
        yield ("random", rows, rows, 0, 0)


def scalar_argmax(data):
    require(len(data) == N * 4, "full vocabulary reference extent")
    winner, maximum = 0, -math.inf
    for token, (value,) in enumerate(struct.iter_unpack("<f", data)):
        require(math.isfinite(value), "active nonfinite fixture prohibited")
        if value > maximum:
            winner, maximum = token, value
    return winner


def row_values(kind, row, step, first_lane):
    pattern = row % 10
    baseline = -1.0
    if kind == "mixed" and pattern == 3:
        baseline = -MAX_FINITE
    elif kind == "mixed" and pattern == 5:
        baseline = -MIN_SUBNORMAL
    elif kind == "mixed" and pattern == 6:
        baseline = MIN_SUBNORMAL
    elif kind == "mixed" and pattern == 7:
        baseline = MAX_FINITE
    data = bytearray(struct.pack("<f", baseline) * N)
    if kind == "random":
        state = (0x9e3779b9 + row) & 0xffffffff
        for token in range(N):
            state ^= state << 13 & 0xffffffff
            state ^= state >> 17
            state ^= state << 5 & 0xffffffff
            bits = state ^ 0x00800000 if state & 0x7f800000 == 0x7f800000 else state
            struct.pack_into("<I", data, token * 4, bits)
        return data, scalar_argmax(data)
    if kind == "lanes":
        winner = step * 64 + first_lane + row
        updates = ((winner, 2.0),)
    elif pattern == 0:
        winner, updates = 101, ((7, 24.375), (101, 24.40625))
    elif pattern == 1:
        winner, updates = 63, ((63, -0.0), (64, 0.0))
    elif pattern == 2:
        winner, updates = 63, ((63, 0.0), (64, -0.0))
    elif pattern in (3, 6, 7):
        winner, updates = 0, ()
    elif pattern == 4:
        winner, updates = N - 1, ((N - 1, MAX_FINITE),)
    elif pattern == 5:
        winner, updates = 129, ((64, 0.0), (129, MIN_SUBNORMAL))
    elif pattern == 8:
        winner, updates = 64, ((64, 2.0), (128, 2.0))
    else:
        winner, updates = 127, ((127, 2.0), (128, 2.0))
    for token, value in updates:
        struct.pack_into("<f", data, token * 4, value)
    return data, winner


def make_case(core, spec):
    kind, rows, capacity, step, first_lane = spec
    require(kind in ("mixed", "lanes", "random"), "fixture kind")
    require(all(type(value) is int for value in (rows, capacity, step, first_lane))
            and 1 <= rows <= capacity <= CAPACITY, "fixture shape")
    require((kind == "lanes" and rows == 32 and step in (0, 1187, 2373) and first_lane in (0, 32))
            or (kind != "lanes" and step == 0 and first_lane == 0), "lane fixture bounds")
    # Nonfinite capacity tails are outside the active view, never deliberate fault cases.
    logits = bytearray(struct.pack("<I", 0x7fc01234) * (capacity * N))
    winners = []
    for row in range(rows):
        data, winner = row_values(kind, row, step, first_lane)
        logits[row * N * 4:(row + 1) * N * 4] = data
        winners.append(winner)
    choices = SENTINEL * capacity
    expected = b"".join(struct.pack("<I", winner) for winner in winners) + choices[rows * 4:]
    return {"name": f"{kind}_rows{rows}_capacity{capacity}_step{step}_lane{first_lane}",
            "symbol": ROOT, "groups": rows, "scalars": [rows],
            "buffers": [core.buffer("logits", logits, 4), core.buffer("choices", choices, 4, expected)]}


def validate_case(case):
    require(case["symbol"] == ROOT and case["scalars"] == [case["groups"]], "one-root launch")
    logits, choices = case["buffers"]
    rows = case["groups"]
    require(logits["data"] == logits["expected"], "input must be immutable")
    for row in range(rows):
        reference = scalar_argmax(logits["data"][row * N * 4:(row + 1) * N * 4])
        require(reference == struct.unpack_from("<I", choices["expected"], row * 4)[0],
                "independent full-vocabulary winner mismatch")
    require(choices["data"][rows * 4:] == choices["expected"][rows * 4:], "choice tail preservation")


def self_test(core):
    specs = list(specifications())
    require(len(specs) == 14 and len(set(specs)) == 14, "closed fourteen-case fixture roster")
    for spec in specs:
        validate_case(make_case(core, spec))
    print("PASS: fourteen finite-active full-vocabulary argmax cases; no GPU")


def main(argv=None):
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
    args = parser.parse_args(argv)
    require(args.self_test != args.run, "choose exactly one of self-test or explicit run")
    core = load_helper(args.helper)
    if args.self_test:
        self_test(core)
        return
    require(all(value is not None for value in (args.worker, args.worker_sha256, args.artifact,
            args.artifact_sha256, args.device_unique_id, args.output)), "explicit run and pins required")
    require(0 < args.device_unique_id < 1 << 64 and args.output.is_absolute(), "run bounds")
    available = next(int(line.split()[1]) * 1024 for line in Path("/proc/meminfo").read_text().splitlines()
                     if line.startswith("MemAvailable:"))
    require(available >= 1024**3, "reserve at least 1 GiB host headroom")
    self_test(core)
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
        for spec in specifications():
            case = make_case(core, spec)
            validate_case(case)
            results.append(core.probe(worker, case, artifact, args.artifact_sha256))
            print(case["name"] + ": PASS", flush=True)
        worker.finish()
        require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                and core.identity(os.fstat(artifact_fd)) == core.identity(artifact_stat)
                and core.digest(Path(__file__).read_bytes()) == source_hash, "retained identity drift")
        report = {"schema": "FerricTpFp32ArgmaxProbeV11", "authority": "none", "benchmark": False,
                  "model_inference": False, "model_parity_qualified": False, "checks_pass": True,
                  "source_sha256": source_hash, "helper_sha256": HELPER_SHA256,
                  "worker_sha256": args.worker_sha256, "artifact_sha256": args.artifact_sha256,
                  "runtime_operational": args.operational, "device_unique_id": args.device_unique_id,
                  "worker_pid": worker.process.pid, "worker_start_ticks": worker.start, "clean_teardown": True,
                  "timing_scope": "worker-reported dispatch latency, not accepted model performance",
                  "closed_roots": [ROOT], "active_inputs_finite": True, "results": results}
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
