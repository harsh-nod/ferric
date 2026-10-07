"""Exact dyadic M32 gate/up data, independent of either GPU implementation."""
from dataclasses import dataclass
import math
import struct

ROWS, N, K = 32, 12288, 4096


def bf16(value):
    if not math.isfinite(value):
        raise ValueError('finite fixture input required')
    bits = struct.unpack('<I', struct.pack('<f', value))[0]
    rounded = ((bits + 0x7fff + ((bits >> 16) & 1)) >> 16) & 0xffff
    if rounded & 0x7f80 == 0x7f80:
        raise ValueError('finite BF16 fixture required')
    return struct.pack('<H', rounded)


def repeat(pattern, elements):
    if not pattern or len(pattern) % 2 or type(elements) is not int or elements <= 0:
        raise ValueError('positive complete BF16 pattern required')
    count = elements * 2
    copies, tail = divmod(count, len(pattern))
    return pattern * copies + pattern[:tail]


def output_numerator(row, column):
    if type(row) is not int or type(column) is not int or not (0 <= row < ROWS and 0 <= column < N):
        raise ValueError('fixed fixture coordinates')
    return (row + 1) * sum((inner % 17 - 8) * ((3 * inner + 5 * column) % 19 - 9)
                           for inner in range(K))


@dataclass(frozen=True)
class Fixture:
    input_bf16: bytes
    weights_kn_bf16: bytes
    output_bf16: bytes


def build_fixture():
    # Every possible partial sum is a dyadic numerator below 2**24 in magnitude.
    # Thus FP32 MFMA accumulation order cannot change this fixture's exact sum.
    if K * ROWS * 8 * 9 >= 1 << 24:
        raise ValueError('exact FP32 numerator bound')
    left = b''.join(repeat(b''.join(bf16((row + 1) * (i - 8) / 16)
                                    for i in range(17)), K) for row in range(ROWS))
    weight_rows = [repeat(b''.join(bf16(((3 * inner + 5 * col) % 19 - 9) / 16)
                                   for col in range(19)), N) for inner in range(19)]
    complete, tail = divmod(K, 19)
    weights = b''.join(weight_rows) * complete + b''.join(weight_rows[:tail])
    base = [output_numerator(0, col) for col in range(19)]
    output = b''.join(repeat(b''.join(bf16((row + 1) * value / 256) for value in base), N)
                      for row in range(ROWS))
    if (len(left), len(weights), len(output)) != (262144, 100663296, 786432):
        raise ValueError('fixed complete fixture byte extents')
    return Fixture(left, weights, output)


def validate_output(actual, expected, name):
    if type(actual) is not bytes or type(expected) is not bytes or actual != expected:
        raise ValueError(name + ': exact BF16 bytes differ')
