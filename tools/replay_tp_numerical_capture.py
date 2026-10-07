#!/usr/bin/env python3
"""Offline bounded operand analysis, never model parity or performance acceptance."""

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

MAX_BYTES = 224 * 1024 * 1024
VOCABULARY = 151936


def require(condition, message):
    if not condition:
        raise ValueError(message)


def no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def read_file(path, maximum, digest=None):
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC), "rb") as source:
        before = os.fstat(source.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_nlink == 1
                and 0 < before.st_size <= maximum, "capture file type/extent/link count")
        data = source.read(maximum + 1)
        after = os.fstat(source.fileno())
        identity = lambda value: (value.st_dev, value.st_ino, value.st_mode, value.st_nlink,
                                  value.st_size, value.st_mtime_ns, value.st_ctime_ns)
        require(identity(before) == identity(after) and len(data) == before.st_size, "capture changed during read")
    actual = hashlib.sha256(data).hexdigest()
    require(digest is None or actual == digest, "capture SHA-256 mismatch")
    return data, actual


def descriptors(value):
    if isinstance(value, dict):
        if "file" in value:
            yield value
        for item in value.values():
            yield from descriptors(item)
    elif isinstance(value, list):
        for item in value:
            yield from descriptors(item)


def load_capture(directory, digest):
    require(directory.is_absolute() and not directory.is_symlink(), "absolute owned capture directory required")
    raw, actual = read_file(directory / "manifest.json", 1024 * 1024, digest)
    manifest = json.loads(raw, object_pairs_hook=no_duplicates)
    require(manifest["schema"] == "FerricTpNumericalCaptureV1" and manifest["complete"] is True
            and manifest["performance_qualified"] is False and manifest["model_parity_qualified"] is False
            and manifest["maximum_total_bytes"] == MAX_BYTES, "diagnostic manifest scope")
    require(manifest["identity"]["tensor_parallel"] == 1
            and manifest["identity"]["projection"] in ("baseline", "mfma"), "diagnostic arithmetic profile")
    payloads, records = {}, {}
    total = len(raw)
    for entry in descriptors(manifest):
        name = entry["file"]
        require(type(name) is str and Path(name).name == name and name not in (".", "..", "manifest.json"), "flat payload name")
        require(type(entry["bytes"]) is int and 0 < entry["bytes"] <= MAX_BYTES, "payload extent")
        if name in records:
            require(records[name] == entry, "conflicting repeated file descriptor")
            continue
        total += entry["bytes"]
        require(total <= MAX_BYTES, "aggregate capture cap")
        data, _ = read_file(directory / name, entry["bytes"], entry["sha256"])
        require(len(data) == entry["bytes"], "payload exact length")
        payloads[name], records[name] = data, entry
    require(total - len(raw) == manifest["payload_bytes_before_manifest"], "payload aggregate receipt")
    require({path.name for path in directory.iterdir()} == set(payloads) | {"manifest.json"}, "unexpected/incomplete capture files")
    return manifest, payloads, actual, total


def words(data, width=2):
    require(len(data) % width == 0, "element byte extent")
    values = array.array("H" if width == 2 else "I")
    values.frombytes(data)
    if sys.byteorder != "little":
        values.byteswap()
    return values


def bf16(bits):
    value = struct.unpack("<f", struct.pack("<I", bits << 16))[0]
    require(math.isfinite(value), "nonfinite captured BF16")
    return value


def fp32(value):
    result = struct.unpack("<f", struct.pack("<f", value))[0]
    require(math.isfinite(result), "nonfinite serial reference")
    return result


def narrow(value):
    bits = struct.unpack("<I", struct.pack("<f", value))[0]
    return ((bits + 0x7fff + ((bits >> 16) & 1)) >> 16) & 0xffff


def sampled_dot(left, right):
    require(len(left) == len(right) and len(left) > 0, "dot extent")
    serial = 0.0
    products = []
    for a, b in zip(left, right, strict=True):
        product = bf16(a) * bf16(b)
        products.append(product)
        serial = fp32(serial + fp32(product))
    return serial, math.fsum(products)


def full_transpose_equal(original, actual, n, k):
    require(len(original) == len(actual) == n * k, "transpose extent")
    return all(original[column * k + inner] == actual[inner * n + column]
               for column in range(n) for inner in range(k))


def analyze(manifest, payloads):
    projection = manifest["projection"]
    rows, n, k = (projection[key] for key in ("rows", "n", "k"))
    require(type(rows) is int and 1 <= rows <= 16 and n in (1024, 4096, 12288)
            and k in (4096, 12288) and rows == len(manifest["execution_rows"]), "projection shape")
    get = lambda entry: payloads[entry["file"]]
    left, weights = words(get(projection["input"])), words(get(projection["weights_nk"]))
    require(len(left) == rows * k and len(weights) == n * k, "projection operand shape")
    transposed = projection["actual_weight_layout"] == "kn"
    require(transposed == (manifest["identity"]["projection"] == "mfma"), "actual layout/profile")
    if transposed:
        require(full_transpose_equal(weights, words(get(projection["actual_weights"])), n, k), "actual MFMA transpose differs from original BF16 bytes")
    else:
        require(projection["actual_weight_layout"] == "nk"
                and projection["weights_nk"] == projection["actual_weights"], "baseline weight identity")
    width = projection["output"]["element_bytes"]
    require(width in (2, 4), "projection output width")
    output = words(get(projection["output"]), width)
    require(len(output) == rows * n, "projection output shape")
    columns = sorted({0, 1, 15, 16, n // 3, n // 2, n - 2, n - 1})
    samples = []
    for row in range(rows):
        for column in columns:
            serial, precise = sampled_dot(left[row*k:(row+1)*k], weights[column*k:(column+1)*k])
            bits = output[row*n+column]
            observed = bf16(bits) if width == 2 else struct.unpack("<f", struct.pack("<I", bits))[0]
            require(math.isfinite(observed), "nonfinite sampled output")
            serial_bits = narrow(serial) if width == 2 else struct.unpack("<I", struct.pack("<f", serial))[0]
            samples.append({"row":row, "column":column, "observed_bits":bits, "observed":observed,
                            "serial_fp32":serial, "serial_output_bits":serial_bits,
                            "matches_serial_bits":bits == serial_bits, "fp64_product_sum":precise,
                            "observed_abs_error_fp64":abs(observed-precise)})
    return {"full_weight_transpose_exact": True if transposed else None,
            "sampled_columns":columns, "projection_samples":samples,
            "all_sampled_outputs_match_serial":all(sample["matches_serial_bits"] for sample in samples),
            "head_request_rows":manifest["head"]["request_rows"],
            "reference_scope":"sampled actual-operand serial FP32 and FP64 exact-product sums; not an MFMA instruction-order model"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, required=True)
    parser.add_argument("--manifest-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    manifest, payloads, digest, total = load_capture(args.capture, args.manifest_sha256)
    result = analyze(manifest, payloads)
    result.update({"schema":"FerricTpNumericalReplayV1", "authority":"none", "performance_qualified":False,
                   "model_parity_qualified":False, "manifest_sha256":digest, "total_bytes":total,
                   "identity":manifest["identity"], "execution_rows":manifest["execution_rows"],
                   "source_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                   "custody_scope":"payload hashes only; require separate zero-exit, close receipt, trace identity and fixed-token validation"})
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(result, output, sort_keys=True, indent=2, allow_nan=False)
        output.write("\n")


if __name__ == "__main__":
    main()
