#!/usr/bin/env python3
"""Nine explicit TP1 large-KV native fixtures; no model or timing qualification."""
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
APPEND = "ferric_qwen3_tp_batch32_large_kv_append_v9"
ATTENTION = "ferric_qwen3_tp_batch32_large_kv_paged_gqa_bf16_f32_v9"
CAPACITY, COLUMNS, PAGE_TOKENS, STRIDE_MAX = 32, 1024, 16, 512
SENTINEL = 0x3555
SPECS = (
    ("append", 1, 513, 8192), ("append", 16, 16384, 8192),
    ("append", 17, 8192, 8192), ("append", 32, 16384, 8192),
    ("attention", 1, 513, 17), ("attention", 16, 16384, 17),
    ("attention", 17, 8192, 17), ("attention", 32, 16384, 17),
    ("attention", 1, 16384, 8192),
)


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


def pack_words(words):
    return b"".join(struct.pack("<H", word) for word in words)


def pack_u32(words):
    return b"".join(struct.pack("<I", word) for word in words)


def bf16(value):
    require(math.isfinite(value), "finite reference")
    bits = struct.unpack("<I", struct.pack("<f", value))[0]
    return ((bits + 0x7fff + ((bits >> 16) & 1)) >> 16) & 0xffff


def validate_spec(kind, rows, pages, context):
    require(kind in ("append", "attention") and type(rows) is int and 1 <= rows <= 32,
            "kind/rows bounds")
    require(type(pages) is int and 1 <= pages <= 16384 and type(context) is int
            and 1 <= context <= 8192, "independent physical/logical bounds")
    require(kind != "attention" or pages >= rows * ((context + 15) // 16),
            "fixture uses disjoint per-row pages")


def fixture_name(spec):
    kind, rows, pages, context = spec
    return f"{kind}_rows{rows}_pages{pages}_context{context}"


def append_case(core, rows, pages):
    validate_spec("append", rows, pages, 8192)
    positions = [row for row in range(rows)]
    positions[-1] = 8191
    tables = [0xffffffff] * (CAPACITY * STRIDE_MAX)
    slots = []
    for row, position in enumerate(positions):
        page = pages - 1 if row == rows - 1 else (511 if row == 0 and pages > 511 else row)
        tables[row * STRIDE_MAX + position // 16] = page
        slots.append(page * PAGE_TOKENS + position % PAGE_TOKENS)
    require(len(set(slots)) == rows and max(slots) < pages * PAGE_TOKENS, "exclusive append slots")
    key = pack_words(0x3e00 + (row * 31 + column) % 256
                     for row in range(CAPACITY) for column in range(COLUMNS))
    value = pack_words(0xbe00 + (row * 17 + column) % 256
                       for row in range(CAPACITY) for column in range(COLUMNS))
    initial = struct.pack("<H", SENTINEL) * (pages * PAGE_TOKENS * COLUMNS)
    keys, values = bytearray(initial), bytearray(initial)
    for row, slot in enumerate(slots):
        destination, source = slot * COLUMNS * 2, row * COLUMNS * 2
        keys[destination:destination + COLUMNS * 2] = key[source:source + COLUMNS * 2]
        values[destination:destination + COLUMNS * 2] = value[source:source + COLUMNS * 2]
    return {"name": fixture_name(("append", rows, pages, 8192)), "symbol": APPEND,
            "groups": 1, "scalars": [rows, 1, STRIDE_MAX, pages],
            "buffers": [core.buffer("key", key, 2), core.buffer("value", value, 2),
                        core.buffer("positions", pack_u32(positions + [0xffffffff] * (CAPACITY - rows)), 4),
                        core.buffer("tables", pack_u32(tables), 4),
                        core.buffer("key_cache", initial, 2, keys),
                        core.buffer("value_cache", initial, 2, values)]}


def value_at(row, head, dimension, token):
    return ((row % 4) * 4 + head % 4 + 1) / 8.0 + (dimension % 8) / 64.0 + (token % 2) / 32.0


def attention_case(core, rows, pages, context):
    validate_spec("attention", rows, pages, context)
    stride = (context + 15) // 16
    positions = [context - 1 if rows == 1 else row % context for row in range(rows)]
    tables = [0xffffffff] * (CAPACITY * stride)
    keys = bytearray(struct.pack("<H", 0x7fc0) * (pages * PAGE_TOKENS * COLUMNS))
    values = bytearray(keys)
    expected = bytearray(struct.pack("<H", SENTINEL) * (CAPACITY * 4096))
    selected = set()
    for row, position in enumerate(positions):
        token_values = [pack_words(bf16(value_at(row, head, dimension, parity))
                                  for head in range(8) for dimension in range(128)) for parity in (0, 1)]
        for logical in range(position // 16 + 1):
            # Reverse placement is deliberately independent of logical order.
            page = pages - 1 - (row * stride + logical)
            require(page not in selected, "fixture page collision")
            selected.add(page)
            tables[row * stride + logical] = page
        for token in range(position + 1):
            page = tables[row * stride + token // 16]
            base = (page * 16 + token % 16) * COLUMNS * 2
            keys[base:base + COLUMNS * 2] = b"\0" * (COLUMNS * 2)
            values[base:base + COLUMNS * 2] = token_values[token % 2]
        # Zero QK gives exact unit weights. All dyadic sums fit FP32 exactly;
        # this independent count formula performs only the final division.
        odd_tokens = (position + 1) // 2
        output = pack_words(bf16(value_at(row, query_head // 4, dimension, 0)
                                 + odd_tokens / ((position + 1) * 32.0))
                            for query_head in range(32) for dimension in range(128))
        expected[row * 4096 * 2:(row + 1) * 4096 * 2] = output
    query = b"\0" * (rows * 4096 * 2) + struct.pack("<H", 0x7fc0) * ((CAPACITY - rows) * 4096)
    return {"name": fixture_name(("attention", rows, pages, context)), "symbol": ATTENTION,
            "groups": rows * 32, "scalars": [rows, 1, stride, pages, context],
            "buffers": [core.buffer("query", query, 2), core.buffer("keys", keys, 2),
                        core.buffer("values", values, 2),
                        core.buffer("positions", pack_u32(positions + [0xffffffff] * (CAPACITY - rows)), 4),
                        core.buffer("tables", pack_u32(tables), 4),
                        core.buffer("output", struct.pack("<H", SENTINEL) * (CAPACITY * 4096), 2, expected)]}


def make_case(core, spec):
    kind, rows, pages, context = spec
    validate_spec(*spec)
    return append_case(core, rows, pages) if kind == "append" else attention_case(core, rows, pages, context)


def self_test(core):
    for spec in SPECS:
        validate_spec(*spec)
    require(len(SPECS) == 9 and len({fixture_name(spec) for spec in SPECS}) == 9, "closed native case roster")
    for rows in (1, 16, 17, 32):
        for kind in ("append", "attention"):
            case = make_case(core, (kind, rows, 513, 17 if kind == "attention" else 8192))
            for record in case["buffers"]:
                require(len(record["data"]) == len(record["expected"]), "fixture extent")
                require(len(record["data"]) % record["element_bytes"] == 0, "element extent")
            del case
    for row in range(32):
        for position in (0, 1, 15, 16, 8191):
            for head in range(8):
                dimension = 127
                direct = sum(value_at(row, head, dimension, token) for token in range(position + 1)) / (position + 1)
                count = value_at(row, head, dimension, 0) + ((position + 1) // 2) / ((position + 1) * 32.0)
                require(bf16(direct) == bf16(count), "independent dense uniform reference")
    print("PASS: nine bounded specs, 513-page full-byte fixtures and independent logical8191 references; no GPU")


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
    require(available >= 12 * 1024**3, "reserve at least 12 GiB host headroom for full-cache custody")
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
        for spec in SPECS:
            case = make_case(core, spec)
            result = core.probe(worker, case, artifact, args.artifact_sha256)
            result["full_output_exact"] = True
            result["active_rows"], result["physical_pages"], result["logical_context"] = spec[1:]
            results.append(result)
            del case
            print(result["name"] + ": PASS", flush=True)
        worker.finish()
        require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                and core.identity(os.fstat(artifact_fd)) == core.identity(artifact_stat)
                and core.digest(Path(__file__).read_bytes()) == source_hash, "retained identity drift")
        report = {"schema": "FerricTpLargeKvProbeV9", "authority": "none", "benchmark": False,
                  "model_inference": False, "model_parity_qualified": False, "checks_pass": True,
                  "source_sha256": source_hash, "helper_sha256": HELPER_SHA256,
                  "worker_sha256": args.worker_sha256, "artifact_sha256": args.artifact_sha256,
                  "runtime_operational": args.operational, "device_unique_id": args.device_unique_id,
                  "worker_pid": worker.process.pid, "worker_start_ticks": worker.start, "clean_teardown": True,
                  "timing_scope": "diagnostic dispatch only, not model performance", "results": results}
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
