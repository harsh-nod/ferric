#!/usr/bin/env python3
"""Closed Draft06B kernel fixtures; CPU self-test or explicit root-owned GPU run."""
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
N, CAPACITY = 151936, 32
PREFIX = "ferric_qwen3_draft_batch32_"
SENTINEL = 0x3555
SHAPES = {
    "query": (2048, 1024, 1, False),
    "key": (1024, 1024, 2, False),
    "value": (1024, 1024, 3, False),
    "gate": (3072, 1024, 4, False),
    "up": (3072, 1024, 5, False),
    "attention_output": (1024, 2048, 1, True),
    "down": (1024, 3072, 2, True),
    "head": (N, 1024, 6, True),
}
ROOTS = tuple(PREFIX + suffix + "_v10" for suffix in (
    "rmsnorm", "embedding_bf16", "gemm_bf16_f32_bf16", "mfma_gemm_bf16",
    "gemm_partial_bf16_f32", "mfma_gemm_partial_f32", "swiglu_bf16_f32",
    "rope", "paged_kv_append", "paged_gqa_bf16_f32", "residual_bf16",
    "head_bf16_f32", "mfma_head_f32", "argmax_f32"))


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


def positions(k):
    require(k in (1024, 2048, 3072), "draft reduction width")
    return (0, 1, 15, 16, k - 1)


def reference(row, column, k):
    # Every product/sum is exactly representable in FP32 for this fixture.
    return sum(activation(row, inner) * weight
               for inner, weight in zip(positions(k), coefficients(column), strict=True))


def words_bytes(values):
    words = array.array("H", values)
    if sys.byteorder != "little":
        words.byteswap()
    return words.tobytes()


def weights(layout, n, k):
    require(layout in ("nk", "kn") and type(n) is int and 102 <= n <= N, "weight fixture bounds")
    data = bytearray(n * k * 2)
    if layout == "nk":
        for column in range(n):
            for inner, value in zip(positions(k), coefficients(column), strict=True):
                struct.pack_into("<H", data, (column * k + inner) * 2, bf16(value))
    else:
        for index, inner in enumerate(positions(k)):
            data[inner * n * 2:(inner + 1) * n * 2] = words_bytes(
                bf16(coefficients(column)[index]) for column in range(n))
    return bytes(data)


def rounded_bf16(value):
    bits = struct.unpack("<I", struct.pack("<f", value))[0]
    require(math.isfinite(value), "finite fixture output")
    return ((bits + 0x7fff + ((bits >> 16) & 1)) >> 16) & 0xffff


def u32(values):
    return b"".join(struct.pack("<I", value) for value in values)


def projection_case(core, rows, role, mfma):
    require(type(rows) is int and 1 <= rows <= CAPACITY and role in SHAPES
            and type(mfma) is bool, "projection fixture shape")
    n, k, tag, fp32 = SHAPES[role]
    inputs = words_bytes(bf16(activation(row, inner)) for row in range(rows) for inner in range(k))
    if role == "head":
        symbol = ROOTS[12 if mfma else 11]
    elif fp32:
        symbol = ROOTS[5 if mfma else 4]
    else:
        symbol = ROOTS[3 if mfma else 2]
    width = 4 if fp32 else 2
    output = b"\xa5" * (CAPACITY * n * width)
    active = (b"".join(struct.pack("<f", reference(row, column, k))
                       for row in range(rows) for column in range(n)) if fp32 else
              words_bytes(rounded_bf16(reference(row, column, k))
                          for row in range(rows) for column in range(n)))
    return {"name": f"{'mfma' if mfma else 'scalar'}_{role}_rows{rows}", "symbol": symbol,
            "groups": ((rows + 15) // 16) * (n // 16), "scalars": [rows, n, k, 1, tag],
            "buffers": [core.buffer("a", inputs, 2),
                        core.buffer("weights_kn" if mfma else "weights_nk", weights("kn" if mfma else "nk", n, k), 2),
                        core.buffer("output", output, width, active + output[len(active):])]}


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
    expected = u32(winners) + choices[rows * 4:]
    return {"name": f"fp32_argmax_rows{rows}", "symbol": ROOTS[13], "groups": rows, "scalars": [rows],
            "buffers": [core.buffer("logits", logits, 4), core.buffer("choices", choices, 4, expected)]}


def embedding_case(core):
    tokens = [0, 1, N - 1, 317] * 8
    weight = bytearray(N * 1024 * 2)
    for token in set(tokens):
        weight[token * 2048:(token + 1) * 2048] = words_bytes(
            bf16((token % 17 + column % 31) / 32.0) for column in range(1024))
    expected = b"".join(weight[token * 2048:(token + 1) * 2048] for token in tokens)
    return {"name": "embedding_rows32_distinct_tokens", "symbol": ROOTS[1], "groups": 512,
            "scalars": [32], "buffers": [core.buffer("tokens", u32(tokens), 4), core.buffer("weight", weight, 2),
                core.buffer("output", core.bf16(SENTINEL, 32 * 1024), 2, expected)]}


def norm_case(core, width):
    require(width in (128, 1024), "norm fixture width")
    rows = 512 if width == 128 else 32
    ones = core.bf16(bf16(1.0), rows * width)
    return {"name": f"rmsnorm_width{width}_rows{rows}", "symbol": ROOTS[0], "groups": rows,
            "scalars": [rows, width, 0x358637bd, 0], "buffers": [core.buffer("input", ones, 2),
                core.buffer("residual", b"", 2), core.buffer("weight", core.bf16(bf16(1.0), width), 2),
                core.buffer("fused", b"", 2, b""),
                core.buffer("normalized", core.bf16(SENTINEL, rows * width), 2, ones)]}


def rope_case(core):
    def data(heads):
        original, rotated = [], []
        for row in range(32):
            for head in range(heads):
                first = words_bytes(bf16((head + 1) / 4.0 + row / 32.0) for _ in range(64))
                second = words_bytes(bf16(-(head + 1) / 8.0) for _ in range(64))
                negative_second = words_bytes(bf16((head + 1) / 8.0) for _ in range(64))
                original.extend((first, second))
                rotated.extend((negative_second, first))
        return b"".join(original), b"".join(rotated)
    query, expected_query = data(16)
    key, expected_key = data(8)
    return {"name": "rope_rows32_q16_kv8_quarter_turn", "symbol": ROOTS[7], "groups": 32,
            "scalars": [32, 1], "buffers": [core.buffer("query", query, 2), core.buffer("key", key, 2),
                core.buffer("cos", struct.pack("<f", 0.0) * 2048, 4),
                core.buffer("sin", struct.pack("<f", 1.0) * 2048, 4),
                core.buffer("positions", u32([row * 7 for row in range(32)]), 4),
                core.buffer("rotated_query", core.bf16(SENTINEL, 32 * 2048), 2, expected_query),
                core.buffer("rotated_key", core.bf16(SENTINEL, 32 * 1024), 2, expected_key)]}


def activation_case(core):
    return {"name": "swiglu_rows32_width3072", "symbol": ROOTS[6], "groups": 1536, "scalars": [32, 1],
            "buffers": [core.buffer("gate", core.bf16(0, 32 * 3072), 2),
                core.buffer("up", core.bf16(bf16(1.0), 32 * 3072), 2),
                core.buffer("output", core.bf16(SENTINEL, 32 * 3072), 2, core.bf16(0, 32 * 3072))]}


def append_case(core):
    rows, pages, columns = 32, 512, 1024
    key = b"".join(words_bytes(bf16((row + 1) / 4.0 + (column // 128) / 8.0)
                              for column in range(columns)) for row in range(rows))
    value = b"".join(words_bytes(bf16(-(row + 1) / 8.0 - (column // 128) / 16.0)
                                for column in range(columns)) for row in range(rows))
    initial = core.bf16(SENTINEL, pages * 16 * columns)
    keys, values = bytearray(initial), bytearray(initial)
    table = [u32([511, 0] + [0xffffffff] * 510)] * rows
    for row in range(rows):
        slot = (511 if row < 16 else 0) * 16 + row % 16
        keys[slot * 2048:(slot + 1) * 2048] = key[row * 2048:(row + 1) * 2048]
        values[slot * 2048:(slot + 1) * 2048] = value[row * 2048:(row + 1) * 2048]
    return {"name": "append_rows32_boundary_physical511", "symbol": ROOTS[8], "groups": 1,
            "scalars": [rows, 1, 512, pages], "buffers": [core.buffer("key", key, 2), core.buffer("value", value, 2),
                core.buffer("positions", u32(range(rows)), 4), core.buffer("table", b"".join(table), 4),
                core.buffer("key_cache", initial, 2, keys), core.buffer("value_cache", initial, 2, values)]}


def attention_case(core, last):
    require(type(last) is bool, "attention fixture selector")
    rows, pages, stride = (1 if last else 17), 512, 512
    positions_value = [8191] if last else [15 + row % 2 for row in range(rows)]
    values_per_token = words_bytes(bf16((head + 1) / 2.0) for head in range(8) for _ in range(128))
    if last:
        table = u32(range(512))
        keys = core.bf16(0, pages * 16 * 1024)
        values = values_per_token * (pages * 16)
    else:
        table = b"".join(u32([511, 0 if position == 16 else 0xffffffff] + [0xffffffff] * 510)
                         for position in positions_value)
        keys = bytearray(core.bf16(0x7fc0, pages * 16 * 1024))
        values = bytearray(keys)
        for page in (0, 511):
            keys[page * 32768:(page + 1) * 32768] = core.bf16(0, 16 * 1024)
            values[page * 32768:(page + 1) * 32768] = values_per_token * 16
    expected = words_bytes(bf16((head // 2 + 1) / 2.0) for _ in range(rows)
                           for head in range(16) for _ in range(128))
    initial = core.bf16(SENTINEL, CAPACITY * 2048)
    return {"name": "attention_logical8191_physical511" if last else "attention_rows17_causal_q16_kv8",
            "symbol": ROOTS[9], "groups": rows * 16, "scalars": [rows, 1, stride, pages, 8192 if last else 17],
            "buffers": [core.buffer("query", core.bf16(0, rows * 2048), 2), core.buffer("keys", keys, 2),
                core.buffer("values", values, 2), core.buffer("positions", u32(positions_value), 4),
                core.buffer("table", table, 4), core.buffer("output", initial, 2, expected + initial[len(expected):])]}


def residual_case(core):
    elements = 32 * 1024
    return {"name": "residual_rows32_single_round", "symbol": ROOTS[10], "groups": 512, "scalars": [32],
            "buffers": [core.buffer("partial", struct.pack("<f", 1.00390625) * elements, 4),
                core.buffer("residual", core.bf16(bf16(0.5), elements), 2),
                core.buffer("output", core.bf16(SENTINEL, elements), 2,
                            core.bf16(rounded_bf16(1.50390625), elements))]}


def case_specs():
    result = [("projection", 5, role, mode) for role in SHAPES if role != "head" for mode in (False, True)]
    result += [("projection", rows, "query", mode) for rows in (17, 32) for mode in (False, True)]
    result += [("projection", rows, "head", mode) for rows in (1, 17, 32) for mode in (False, True)]
    result += [("argmax", rows) for rows in (1, 17, 32)]
    result += [("embedding",), ("norm", 1024), ("norm", 128), ("rope",), ("activation",),
               ("append",), ("attention", False), ("attention", True), ("residual",)]
    require(len(result) == 36, "closed fixture count")
    return result


FACTORIES = {"projection": projection_case, "argmax": argmax_case, "embedding": embedding_case,
             "norm": norm_case, "rope": rope_case, "activation": activation_case,
             "append": append_case, "attention": attention_case, "residual": residual_case}


def self_test(core):
    for k in (1024, 2048, 3072):
        nk, kn = weights("nk", 128, k), weights("kn", 128, k)
        for column in range(128):
            for inner in range(k):
                require(nk[(column * k + inner) * 2:(column * k + inner + 1) * 2]
                        == kn[(inner * 128 + column) * 2:(inner * 128 + column + 1) * 2], "full transpose")
        for row in (0, 4, 16, 31):
            for column in range(128):
                total = 0.0
                for inner in range(k):
                    word = struct.unpack_from("<H", nk, (column * k + inner) * 2)[0]
                    value = struct.unpack("<f", struct.pack("<I", word << 16))[0]
                    total = struct.unpack("<f", struct.pack("<f", total + activation(row, inner) * value))[0]
                require(total == reference(row, column, k), "independent real-K serial FP32 reference")
    for row in (0, 4, 16, 31):
        require(max(range(N), key=lambda c: reference(row, c, 1024)) == 101, "full-vocabulary ordinary-ID winner")
    symbols = set()
    for spec in case_specs():
        kind = spec[0]
        if kind == "embedding" or kind == "projection":
            # CPU tests avoid full model-weight allocations. Native paths retain them.
            if kind == "embedding":
                symbols.add(ROOTS[1])
            else:
                _, _, role, mode = spec
                symbols.add(ROOTS[(12 if mode else 11) if role == "head" else
                                  (5 if mode else 4) if SHAPES[role][3] else (3 if mode else 2)])
            continue
        case = FACTORIES[kind](core, *spec[1:])
        symbols.add(case["symbol"])
        require(0 < case["groups"] <= 18992, "bounded launch")
        for record in case["buffers"]:
            require(len(record["data"]) == len(record["expected"])
                    and len(record["data"]) % record["element_bytes"] == 0, "fixture extents")
    require(symbols == set(ROOTS), "all14 root coverage")
    print("PASS:36 native specifications;14 roots; real-K complete transpose and independent FP32 references; no GPU")


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
    require(available >= 12 * 1024**3, "reserve12GiB host headroom for full-weight custody")
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
        for spec in case_specs():
            case = FACTORIES[spec[0]](core, *spec[1:])
            result = core.probe(worker, case, artifact, args.artifact_sha256)
            result["full_output_exact"] = True
            result["fixture"] = list(spec)
            results.append(result)
            print(result["name"] + ": PASS", flush=True)
            del case
        require(len(results) == 36 and {r["symbol"] for r in results} == set(ROOTS), "complete root fixture roster")
        worker.finish()
        require(core.identity(os.fstat(worker_fd)) == core.identity(worker_stat)
                and core.identity(os.fstat(artifact_fd)) == core.identity(artifact_stat)
                and core.digest(Path(__file__).read_bytes()) == source_hash, "retained identity drift")
        report = {"schema": "FerricDraftBatch32SyntheticKernelProbeV10", "authority": "none", "benchmark": False,
                  "model_inference": False, "model_parity_qualified": False, "checks_pass": True,
                  "source_sha256": source_hash, "helper_sha256": HELPER_SHA256,
                  "worker_sha256": args.worker_sha256, "artifact_sha256": args.artifact_sha256,
                  "runtime_operational": args.operational, "device_unique_id": args.device_unique_id,
                  "worker_pid": worker.process.pid, "worker_start_ticks": worker.start, "clean_teardown": True,
                  "timing_scope": "diagnostic dispatch only, not model or accepted performance", "results": results}
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
