#!/usr/bin/env python3
"""Eight analytical V15 fixtures; no image binding or native launch entrypoint."""
import argparse
import hashlib
import os
from pathlib import Path
import stat
import struct
from types import ModuleType

HELPER_SHA256 = "a9e702e77f00e414984ac44dc1d72da15145ae3013c6077932c2daef5004203b"
ROOT = "ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15"
WIDTH, WAVE, EPSILON_BITS = 4096, 64, 0x358637BD
EXPECTED_ABI = (14, 96, 96, 352)
SENTINEL = struct.pack("<H", 0x55AA)


def require(ok, message):
    if not ok:
        raise ValueError(message)


def load_helper(path):
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 128 * 1024, "helper extent")
        with os.fdopen(os.dup(fd), "rb") as source:
            raw = source.read(128 * 1024 + 1)
        after = os.fstat(fd)
        fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
        require(all(getattr(before, key) == getattr(after, key) for key in fields)
                and len(raw) == before.st_size and hashlib.sha256(raw).hexdigest() == HELPER_SHA256,
                "helper identity drift")
    finally:
        os.close(fd)
    module = ModuleType("ferric_v15_frozen_probe_core")
    module.__file__ = str(path)
    exec(compile(raw, str(path), "exec", dont_inherit=True), module.__dict__)
    return module


def specifications():
    return tuple((family, rows) for family in ("uniform", "signed") for rows in (1, 16, 17, 32))


def make_case(core, spec):
    require(type(spec) is tuple and len(spec) == 2 and type(spec[0]) is str
            and type(spec[1]) is int and spec in specifications(), "closed fixture specification")
    family, rows = spec
    elements = rows * WIDTH
    if family == "uniform":
        inputs = core.bf16(0x3F80, elements)
        weights = core.bf16(0x3F80, WIDTH)
        expected = inputs
    else:
        inputs, expected = bytearray(), bytearray()
        weights = struct.pack("<4096H", *(0x3780 + column for column in range(WIDTH)))
        for row in range(rows):
            for column in range(WIDTH):
                lane, component = column % WAVE, column // WAVE
                sign = (((row + 1) & lane).bit_count() + component.bit_count()) & 1
                inputs.extend(struct.pack("<H", 0x3F80 | (sign << 15)))
                expected.extend(struct.pack("<H", (0x3780 + column) | (sign << 15)))
    return dict(name=f"wave_rmsnorm_v15_{family}_rows{rows}", symbol=ROOT, groups=rows,
        scalars=[rows, WIDTH, EPSILON_BITS, 0], buffers=[
            core.buffer("input", inputs, 2),
            core.buffer("residual", b"", 2),
            core.buffer("weight", weights, 2),
            core.buffer("fused_residual", b"", 2, b""),
            core.buffer("normalized", SENTINEL * elements, 2, expected)])


def validate_case(core, case):
    require(type(case) is dict and set(case) == {"name", "symbol", "groups", "scalars", "buffers"},
            "closed case fields")
    matches = [spec for spec in specifications()
               if case["name"] == f"wave_rmsnorm_v15_{spec[0]}_rows{spec[1]}"]
    require(len(matches) == 1 and type(case["name"]) is str and type(case["symbol"]) is str,
            "closed case name and root")
    require(type(case["groups"]) is int and type(case["scalars"]) is list
            and len(case["scalars"]) == 4 and all(type(value) is int for value in case["scalars"]),
            "exact integer launch/scalar values")
    require(type(case["buffers"]) is list and len(case["buffers"]) == 5, "five buffers")
    for buffer in case["buffers"]:
        require(type(buffer) is dict and set(buffer) == {"name", "data", "element_bytes", "expected", "access"},
                "closed buffer fields")
        require(type(buffer["name"]) is str and type(buffer["access"]) is str
                and type(buffer["element_bytes"]) is int and buffer["element_bytes"] == 2
                and type(buffer["data"]) is bytes and type(buffer["expected"]) is bytes,
                "immutable BF16 buffer carriers")
    require(case == make_case(core, matches[0]), "exact analytical bytes, access, extents and launch")


def self_test(core):
    specs = specifications()
    require(len(specs) == len(set(specs)) == 8, "eight-case roster")
    for spec in specs:
        validate_case(core, make_case(core, spec))
    print("PASS: 8 analytical V15 cases; 40 active-buffer records and 80 guards planned; no GPU")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--self-test", action="store_true", required=True)
    parser.add_argument("--helper", type=Path, required=True)
    args = parser.parse_args(argv)
    self_test(load_helper(args.helper))


if __name__ == "__main__":
    main()
