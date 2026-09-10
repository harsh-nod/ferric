#!/usr/bin/env python3
"""Single-device peer-kernel arithmetic probes; not a peer-mapping qualification."""

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
REDUCE = "ferric_qwen3_tp_batch32_peer_ordered_residual_bf16_v6"
COPY = "ferric_qwen3_tp_batch32_peer_copy_bf16_v6"


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
                raise RuntimeError("helper exceeds byte limit")
            chunks.append(chunk)
        data = b"".join(chunks)
        after = os.fstat(fd)
        fields = ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")
        if (any(getattr(before, key) != getattr(after, key) for key in fields)
                or hashlib.sha256(data).hexdigest() != HELPER_SHA256):
            raise RuntimeError("helper identity mismatch")
    finally:
        os.close(fd)
    module = types.ModuleType("ferric_verified_tp_probe_helper")
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def bf16(value):
    if not math.isfinite(value):
        raise ValueError("finite reference required")
    bits = struct.unpack("<I", struct.pack("<f", value))[0]
    return ((bits + 0x7fff + ((bits >> 16) & 1)) >> 16) & 0xffff


def ordered_reference(partials, residual):
    total = 0.0
    for partial in partials:
        total = f32(total + partial)
    return bf16(f32(total + residual))


def cases(core):
    for rows in (1, 16, 17, 31, 32):
        elements = rows * 4096
        for world in (2, 8):
            partials = [bytearray() for _ in range(8)]
            expected = bytearray()
            residual = bytearray()
            for index in range(elements):
                row, column = divmod(index, 4096)
                values = [f32(((column % 13) - 6) * (rank + 1) / 1024 + row / 256)
                          for rank in range(world)]
                for rank, value in enumerate(values):
                    partials[rank].extend(struct.pack("<f", value))
                base = 1.0 if column % 2 == 0 else -0.5
                residual.extend(struct.pack("<H", bf16(base)))
                expected.extend(struct.pack("<H", ordered_reference(values, base)))
            buffers = [core.buffer(f"p{rank}", data, 4) for rank, data in enumerate(partials)]
            buffers += [core.buffer("residual", residual, 2),
                        core.buffer("output", core.bf16(0x3555, elements), 2, expected)]
            yield {"name": f"ordered_tp{world}_rows{rows}", "symbol": REDUCE,
                   "groups": rows * 64, "scalars": [rows, world], "buffers": buffers}
        patterns = [0, 0x8000, 0x7f80, 0xff80, 0x7fc1, 0x7f81, 0xffff, 0x3f80]
        data = b"".join(struct.pack("<H", index & 0xffff if rows >= 16 else patterns[index % 8])
                        for index in range(elements))
        yield {"name": f"copy_all_bits_rows{rows}", "symbol": COPY,
               "groups": rows * 64, "scalars": [rows],
               "buffers": [core.buffer("source", data, 2),
                           core.buffer("output", core.bf16(0x3555, elements), 2, data)]}
    for world in (2, 8):
        for mode in ("single_round", "rank_order"):
            values = ([1 / 256] * world if mode == "single_round" else
                      ([16_777_216.0, 1.0, -16_777_216.0, 0.5, -0.25, 0.125, 0.0625, -0.03125]
                       if world == 8 else [16_777_216.0, -16_777_216.0]))
            values += [0.0] * (8 - world)
            buffers = [core.buffer(f"p{rank}", struct.pack("<f", value) * 4096 if rank < world else b"", 4)
                       for rank, value in enumerate(values)]
            expected = core.bf16(ordered_reference(values[:world], 1.0), 4096)
            buffers += [core.buffer("residual", core.bf16(0x3f80, 4096), 2),
                        core.buffer("output", core.bf16(0x3555, 4096), 2, expected)]
            yield {"name": f"tp{world}_{mode}", "symbol": REDUCE,
                   "groups": 64, "scalars": [1, world], "buffers": buffers}


def validate_case(core, case):
    rows = case["scalars"][0]
    core.require(case["groups"] == rows * 64 and 1 <= rows <= 32, "launch bounds")
    core.require(case["symbol"] in (REDUCE, COPY), "unknown root")
    if case["symbol"] == REDUCE:
        core.require(case["scalars"][1] in (2, 8), "world bound")
    for index, record in enumerate(case["buffers"]):
        disabled = case["symbol"] == REDUCE and case["scalars"][1] == 2 and 2 <= index < 8
        size = len(record["data"])
        core.require(size == (0 if disabled else rows * 4096 * record["element_bytes"]), "active slice extent")
        core.require(size <= core.LIMIT and len(record["expected"]) == size, "fixture byte limit")


def self_test(core):
    fixtures = list(cases(core))
    core.require(len(fixtures) == 19 and {case["symbol"] for case in fixtures} == {REDUCE, COPY}, "closed fixture roster")
    for case in fixtures:
        validate_case(core, case)
    for rows, world, groups in [(0, 2, 0), (33, 2, 2112), (2**32-1, 2, 64),
                                (1, 1, 64), (1, 3, 64), (1, 2, 63), (1, 2, 65)]:
        invalid = dict(fixtures[0], scalars=[rows, world], groups=groups)
        try:
            validate_case(core, invalid)
        except RuntimeError:
            pass
        else:
            raise RuntimeError("invalid boundary fixture was accepted")
    for index in [0, 2, 9]:
        buffers = [dict(record) for record in fixtures[0]["buffers"]]
        buffers[index]["data"] = buffers[index]["data"] + b"\0\0\0\0"
        try:
            validate_case(core, dict(fixtures[0], buffers=buffers))
        except RuntimeError:
            pass
        else:
            raise RuntimeError("invalid active or disabled slice was accepted")
    core.require(ordered_reference([16_777_216.0, 1.0, -16_777_216.0], 1.0) == 0x3f80, "rank order")
    core.require(ordered_reference([1 / 256, 1 / 256], 1.0) == 0x3f81, "single final rounding")
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
    helper = args.helper or Path(__file__).resolve().parents[3] / "proofs/tensor-parallel-kernels-v1/probe.py"
    core = load_helper(helper)
    fixtures = self_test(core)
    if args.self_test:
        core.require(not args.run, "self-test cannot launch a worker")
        print("PASS: 19 bounded arithmetic fixtures; no GPU worker launched")
        return
    core.require(args.run and all(value is not None for value in
                 (args.worker, args.worker_sha256, args.artifact, args.artifact_sha256,
                  args.device_unique_id, args.output)), "explicit run and exact identities required")
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
        report = {"schema": "FerricTpPeer32ArithmeticProbeV6", "authority": "none",
                  "model_inference": False, "benchmark": False, "peer_mapping_qualification": False,
                  "source_sha256": source_identity, "helper_sha256": HELPER_SHA256,
                  "worker_sha256": args.worker_sha256, "artifact_sha256": args.artifact_sha256,
                  "device_unique_id": args.device_unique_id, "worker_pid": worker.process.pid,
                  "worker_start_ticks": worker.start, "clean_teardown": True, "results": results}
        with (args.output / "result.json").open("x", encoding="utf-8") as output:
            json.dump(report, output, sort_keys=True, indent=2)
            output.write("\n")
        print("PASS: 19 GPU arithmetic probes; no peer-mapping or benchmark claim")
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
