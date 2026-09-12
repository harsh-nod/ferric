#!/usr/bin/env python3
"""Eight exact finite TP1 attention checks; no model or performance qualification."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import struct
import types

HELPER_SHA256 = "a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b"
ARTIFACT_SHA256 = "98b5fdb14ac7242e324e1801e1885c98e77473d622b2e9b3f9f12f14c7d75502"
ROOTS = ("ferric_qwen3_tp_batch32_paged_gqa_bf16_f32_v5",
         "ferric_qwen3_tp_batch32_wave_paged_gqa_bf16_v5")
CAPACITY, QUERY_HEADS, KV_HEADS, DIMENSION = 32, 32, 8, 128
PAGE_TOKENS, PHYSICAL_PAGES, CONTEXT = 16, 68, 32
POSITION_CYCLE = (0, 1, 14, 15, 16, 17, 30, 31)
SENTINEL = struct.pack("<H", 0x3555)


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
    module = types.ModuleType("ferric_tp1_attention_verified_fixture_helper")
    module.__file__ = str(path)
    exec(compile(data, str(path), "exec"), module.__dict__)
    return module


def specifications():
    for family, rows in (("uniform", 1), ("uniform", 17), ("uniform", 31), ("selector", 32)):
        for mode in ("baseline", "wave"):
            yield (family, rows, mode)


def exact_bf16(values):
    output = bytearray()
    for value in values:
        require(type(value) is int and -256 <= value <= 256, "exact finite integer range")
        bits = struct.unpack("<I", struct.pack("<f", value))[0]
        require(bits & 0xffff == 0, "integer must be exactly representable in BF16")
        output.extend(struct.pack("<H", bits >> 16))
    return bytes(output)


def position(row, rows):
    return 1 if rows == 1 else POSITION_CYCLE[row % len(POSITION_CYCLE)]


def pages(row):
    return (2 * (31 - row) + 1, 2 * (31 - row))


def pattern(index, dimension):
    return -1 if (((index & 1) * ((dimension >> 6) & 1)
                   + ((index >> 1) & 1) * (dimension & 1)) & 1) else 1


def base_value(row, head, dimension):
    return 2 * row + 8 * head + dimension % 16 + 16 * (dimension // 64)


def expected_value(family, row, head, dimension, count):
    base = base_value(row, head // 4, dimension)
    selected = head % 4
    matches = 0 if count <= selected else (count - 1 - selected) // 4 + 1
    if family == "uniform" or matches == 0:
        return base + count
    return base + 2 * selected + 1 + 4 * (matches - 1)


def make_case(core, spec):
    require(type(spec) is tuple and spec in tuple(specifications())
            and type(spec[1]) is int, "closed fixture specification")
    family, rows, mode = spec
    query = bytearray(exact_bf16([-2]) * (CAPACITY * QUERY_HEADS * DIMENSION))
    cache_elements = PHYSICAL_PAGES * PAGE_TOKENS * KV_HEADS * DIMENSION
    keys = bytearray(exact_bf16([3]) * cache_elements)
    values = bytearray(exact_bf16([-64]) * cache_elements)
    output = SENTINEL * (CAPACITY * QUERY_HEADS * DIMENSION)
    expected = bytearray(output)
    for row in range(rows):
        count = position(row, rows) + 1
        for head in range(QUERY_HEADS):
            q = [1 + head % 4] * DIMENSION if family == "uniform" else [
                pattern(head % 4, dimension) for dimension in range(DIMENSION)]
            offset = (row * QUERY_HEADS + head) * DIMENSION * 2
            query[offset:offset + DIMENSION * 2] = exact_bf16(q)
            expected[offset:offset + DIMENSION * 2] = exact_bf16([
                expected_value(family, row, head, dimension, count) for dimension in range(DIMENSION)])
        for token in range(CONTEXT):
            physical = pages(row)[token // PAGE_TOKENS]
            key = [1] * DIMENSION if family == "uniform" else [
                32 * pattern(token % 4, dimension) for dimension in range(DIMENSION)]
            for head in range(KV_HEADS):
                offset = ((physical * PAGE_TOKENS + token % PAGE_TOKENS) * KV_HEADS + head) * DIMENSION * 2
                keys[offset:offset + DIMENSION * 2] = exact_bf16(key)
                value = [2 * token + 1 + base_value(row, head, dimension)
                         for dimension in range(DIMENSION)] if token < count else [224] * DIMENSION
                values[offset:offset + DIMENSION * 2] = exact_bf16(value)
    positions = struct.pack("<32I", *(position(row, rows) for row in range(CAPACITY)))
    table = struct.pack("<64I", *(page for row in range(CAPACITY) for page in pages(row)))
    return {"name": f"{mode}_{family}_rows{rows}_tp1_causal_pages", "symbol": ROOTS[mode == "wave"],
            "groups": rows * QUERY_HEADS, "scalars": [rows, 1, 2, PHYSICAL_PAGES, CONTEXT],
            "buffers": [core.buffer("query", query, 2), core.buffer("keys", keys, 2),
                        core.buffer("values", values, 2), core.buffer("positions", positions, 4),
                        core.buffer("table", table, 4), core.buffer("output", output, 2, expected)]}


def validate_case(case):
    require(set(case) == {"name", "symbol", "groups", "scalars", "buffers"}, "closed case schema")
    matches = [spec for spec in specifications()
               if case["name"] == f"{spec[2]}_{spec[0]}_rows{spec[1]}_tp1_causal_pages"]
    require(len(matches) == 1, "closed case name")
    family, rows, mode = matches[0]
    require(case["symbol"] == ROOTS[mode == "wave"] and type(case["groups"]) is int
            and case["groups"] == rows * QUERY_HEADS
            and all(type(value) is int for value in case["scalars"])
            and case["scalars"] == [rows, 1, 2, PHYSICAL_PAGES, CONTEXT], "TP1 launch geometry")
    names = ("query", "keys", "values", "positions", "table", "output")
    extents = (262144, 2228224, 2228224, 128, 256, 262144)
    require(len(case["buffers"]) == 6, "six source ABI slices")
    for index, (record, name, extent) in enumerate(zip(case["buffers"], names, extents, strict=True)):
        require(record["name"] == name and record["element_bytes"] == (4 if index in (3, 4) else 2)
                and len(record["data"]) == len(record["expected"]) == extent
                and record["access"] == ("write" if index == 5 else "read"), "exact slice contract")
        if index != 5:
            require(record["data"] == record["expected"], "immutable input")
        if index in (0, 1, 2, 5):
            for data in (record["data"], record["expected"]):
                require(all(bits & 0x7f80 != 0x7f80 for (bits,) in struct.iter_unpack("<H", data)),
                        "all active and inactive fixture values must be finite")
    output = case["buffers"][-1]
    require(output["data"] == SENTINEL * (CAPACITY * QUERY_HEADS * DIMENSION), "output sentinel")
    expected = bytearray(output["data"])
    for row in range(rows):
        for head in range(QUERY_HEADS):
            offset = (row * QUERY_HEADS + head) * DIMENSION * 2
            expected[offset:offset + DIMENSION * 2] = exact_bf16([
                expected_value(family, row, head, dimension, position(row, rows) + 1)
                for dimension in range(DIMENSION)])
    require(output["expected"] == expected, "exact analytical output and inactive tail")


def self_test(core):
    specs = list(specifications())
    require(len(specs) == len(set(specs)) == 8, "closed eight-case fixture roster")
    for spec in specs:
        validate_case(make_case(core, spec))
    print("PASS: eight exact finite TP1 attention cases; no GPU")


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
    require(args.artifact_sha256 == ARTIFACT_SHA256, "only the frozen inspected v5 image is admitted")
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
        report = {"schema": "FerricTp1AttentionProbeV1", "authority": "none", "benchmark": False,
                  "model_inference": False, "model_parity_qualified": False, "checks_pass": True,
                  "source_sha256": source_hash, "helper_sha256": HELPER_SHA256,
                  "worker_sha256": args.worker_sha256, "artifact_sha256": args.artifact_sha256,
                  "runtime_operational": args.operational, "device_unique_id": args.device_unique_id,
                  "worker_pid": worker.process.pid, "worker_start_ticks": worker.start, "clean_teardown": True,
                  "timing_scope": "worker-reported dispatch latency, not accepted model performance",
                  "closed_roots": list(ROOTS), "active_inputs_finite": True, "results": results}
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
