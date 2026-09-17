#!/usr/bin/env python3
"""Extract one exact, independently hashed BF16 checkpoint tensor without Torch."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import stat

MODEL = "Qwen/Qwen3-0.6B"
REVISION = "c1899de289a04d12100db370d81485cdf75e47ca"
CHECKPOINT_SHA256 = "f47f71177f32bcd101b7573ec9171e6a57f4f4d31148d38e382306f42996874b"
TENSOR = "model.layers.0.self_attn.k_proj.weight"
SHAPE = [1024, 1024]
SIZES = {"BF16": 2, "F16": 2, "F32": 4, "F64": 8,
         "I8": 1, "U8": 1, "I16": 2, "U16": 2, "I32": 4, "U32": 4,
         "I64": 8, "U64": 8, "BOOL": 1}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate safetensors JSON key")
        result[key] = value
    return result


def inspect_layout(header, data_bytes):
    require(type(header) is dict and 0 <= data_bytes <= 2**31, "invalid checkpoint header/size")
    intervals = []
    for name, tensor in header.items():
        if name == "__metadata__":
            require(type(tensor) is dict and all(type(k) is str and type(v) is str
                                               for k, v in tensor.items()), "invalid metadata")
            continue
        require(type(tensor) is dict and set(tensor) == {"dtype", "shape", "data_offsets"},
                "unexpected tensor record fields")
        shape, offsets, dtype = tensor["shape"], tensor["data_offsets"], tensor["dtype"]
        require(type(shape) is list and len(shape) <= 8
                and all(type(size) is int and 0 <= size <= 2**31 for size in shape),
                "invalid tensor shape")
        require(type(dtype) is str and dtype in SIZES and type(offsets) is list and len(offsets) == 2
                and all(type(offset) is int for offset in offsets), "invalid tensor layout")
        start, end = offsets
        require(0 <= start <= end <= data_bytes
                and end - start == math.prod(shape) * SIZES[dtype], "tensor extent mismatch")
        intervals.append((start, end, name))
    cursor = 0
    for start, end, _ in sorted(intervals):
        require(start == cursor, "overlapping tensor aliases or uncovered checkpoint bytes")
        cursor = end
    require(cursor == data_bytes, "checkpoint data tail is not described")
    require(TENSOR in header and header[TENSOR]["dtype"] == "BF16"
            and header[TENSOR]["shape"] == SHAPE, "fixed key-projection tensor mismatch")
    return header[TENSOR]


def stat_identity(value):
    return (value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns, value.st_ctime_ns)


def extract(checkpoint, output):
    with checkpoint.open("rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode), "checkpoint must be a regular file")
        header_length_bytes = stream.read(8)
        require(len(header_length_bytes) == 8, "truncated safetensors header length")
        header_length = struct.unpack("<Q", header_length_bytes)[0]
        require(2 <= header_length <= 16 * 1024 * 1024
                and 8 + header_length <= before.st_size, "header length exceeds fixed cap")
        header_bytes = stream.read(header_length)
        header = json.loads(header_bytes, object_pairs_hook=unique_object)
        tensor = inspect_layout(header, before.st_size - 8 - header_length)
        stream.seek(0)
        checksum = hashlib.sha256()
        while chunk := stream.read(1024 * 1024):
            checksum.update(chunk)
        require(checksum.hexdigest() == CHECKPOINT_SHA256, "checkpoint SHA256 differs from pinned revision")
        start, end = tensor["data_offsets"]
        absolute_offset = 8 + header_length + start
        stream.seek(absolute_offset)
        weights = stream.read(end - start)
        require(len(weights) == 1024 * 1024 * 2, "incomplete selected tensor read")
        require(stat_identity(before) == stat_identity(os.fstat(stream.fileno())),
                "checkpoint identity changed during inspection")
    record = {
        "schema": "ferric-qwen3-kproj-checkpoint-v1", "model": MODEL, "revision": REVISION,
        "checkpoint_sha256": CHECKPOINT_SHA256, "checkpoint_bytes": before.st_size,
        "tensor_key": TENSOR, "dtype": "BF16", "shape": SHAPE,
        "header_bytes": header_length, "header_sha256": hashlib.sha256(header_bytes).hexdigest(),
        "data_offsets": tensor["data_offsets"], "absolute_file_offset": absolute_offset,
        "tensor_bytes": len(weights), "tensor_sha256": hashlib.sha256(weights).hexdigest(),
        "layout": "unmodified contiguous row-major checkpoint bytes",
        "extractor_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    (output / "weights.bf16le").write_bytes(weights)
    (output / "checkpoint.json").write_text(json.dumps(record, indent=2) + "\n")
    return record


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkpoint", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(extract(args.checkpoint, args.out_dir), sort_keys=True))


if __name__ == "__main__":
    main()
