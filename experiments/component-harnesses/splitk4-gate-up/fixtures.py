"""Exact C1 gate/up fixture in both resident weight layouts, plus split-K partials."""
from dataclasses import dataclass
import math
import struct

N, K, PARTITIONS = 12288, 4096, 4
OUTPUT_CAPACITY_BYTES = 32 * N * 2


def bf16(value):
    if not math.isfinite(value):
        raise ValueError('finite fixture input required')
    bits = struct.unpack('<I', struct.pack('<f', value))[0]
    rounded = ((bits + 0x7fff + ((bits >> 16) & 1)) >> 16) & 0xffff
    if rounded & 0x7f80 == 0x7f80:
        raise ValueError('finite BF16 fixture required')
    return struct.pack('<H', rounded)


def repeat(pattern, elements, width=2):
    if (type(pattern) is not bytes or type(width) is not int or width not in (2, 4)
            or not pattern or len(pattern) % width or type(elements) is not int or elements <= 0):
        raise ValueError('positive complete typed pattern required')
    count = elements * width
    copies, tail = divmod(count, len(pattern))
    return pattern * copies + pattern[:tail]


def partial_numerator(partition, column):
    if (type(partition) is not int or type(column) is not int
            or not (0 <= partition < PARTITIONS and 0 <= column < N)):
        raise ValueError('fixed partition and column required')
    return sum((inner % 17 - 8) * ((3 * inner + 5 * column) % 19 - 9)
               for inner in range(partition * 1024, (partition + 1) * 1024))


@dataclass(frozen=True)
class Fixture:
    input_bf16: bytes
    weights_nk_bf16: bytes
    weights_kn_bf16: bytes
    partials_f32: bytes
    output_bf16: bytes


def build_fixture():
    # All intermediate sums have exact dyadic numerators below 2**24.
    if K * 8 * 9 >= 1 << 24:
        raise ValueError('exact FP32 numerator bound')
    left = repeat(b''.join(bf16((inner - 8) / 16) for inner in range(17)), K)
    kn_rows = [repeat(b''.join(bf16(((3 * inner + 5 * column) % 19 - 9) / 16)
                              for column in range(19)), N) for inner in range(19)]
    complete, tail = divmod(K, 19)
    kn = b''.join(kn_rows) * complete + b''.join(kn_rows[:tail])
    nk_rows = [repeat(b''.join(bf16(((3 * inner + 5 * column) % 19 - 9) / 16)
                              for inner in range(19)), K) for column in range(19)]
    complete, tail = divmod(N, 19)
    nk = b''.join(nk_rows) * complete + b''.join(nk_rows[:tail])
    numerators = [[partial_numerator(partition, column) for column in range(19)]
                  for partition in range(PARTITIONS)]
    partials = b''.join(repeat(b''.join(struct.pack('<f', value / 256) for value in row), N, 4)
                        for row in numerators)
    output = repeat(b''.join(bf16(sum(row[column] for row in numerators) / 256)
                             for column in range(19)), N)
    if tuple(map(len, (left, nk, kn, partials, output))) != (8192, 100663296, 100663296, 196608, 24576):
        raise ValueError('complete fixed fixture byte extents')
    return Fixture(left, nk, kn, partials, output)


def validate_output(actual, expected, name):
    if type(actual) is not bytes or type(expected) is not bytes or actual != expected:
        raise ValueError(name + ': exact fixture bytes differ')
