"""Exact finite component inputs for the closed TP1/C1 down shape.

These fixtures neither launch a worker nor claim model-level correctness.
All products and intermediate sums are exact binary32 dyadics, so layout,
partition coverage and merge errors need no numerical tolerance.
"""

from dataclasses import dataclass
import struct


K = 12288
N = 4096
SPLITS = 8
PART_K = K // SPLITS
DENOMINATOR = 16
WEIGHT_PERIOD = 19


def input_numerator(k):
    if type(k) is not int or not 0 <= k < K:
        raise ValueError("input coordinate outside the fixed shape")
    return k % 17 - 8


def weight_numerator(k, n):
    if type(k) is not int or type(n) is not int or not 0 <= k < K or not 0 <= n < N:
        raise ValueError("weight coordinate outside the fixed shape")
    return (3 * k + 5 * n) % WEIGHT_PERIOD - 9


def exact_bf16(numerator):
    if type(numerator) is not int or not -9 <= numerator <= 9:
        raise ValueError("closed dyadic fixture range")
    bits, = struct.unpack("<I", struct.pack("<f", numerator / DENOMINATOR))
    if bits & 0xFFFF:
        raise ValueError("fixture value is not exactly BF16")
    return struct.pack("<H", bits >> 16)


def repeated_prefix(pattern, elements, element_bytes):
    if not pattern or len(pattern) % element_bytes or elements <= 0:
        raise ValueError("nonempty complete pattern required")
    count = len(pattern) // element_bytes
    return pattern * (elements // count) + pattern[:elements % count * element_bytes]


def partition_numerators():
    # Periodicity in N bounds the independent integer reference to 19 columns.
    return tuple(tuple(sum(input_numerator(k) * weight_numerator(k, n)
                           for k in range(part * PART_K, (part + 1) * PART_K))
                       for n in range(WEIGHT_PERIOD))
                 for part in range(SPLITS))


@dataclass(frozen=True)
class Fixture:
    input_bf16: bytes
    weights_nk_bf16: bytes
    weights_kn_bf16: bytes
    partials_f32: bytes
    output_f32: bytes


def make_fixture():
    a = b"".join(exact_bf16(input_numerator(k)) for k in range(K))
    nk_rows = tuple(repeated_prefix(
        b"".join(exact_bf16(weight_numerator(k, n)) for k in range(WEIGHT_PERIOD)), K, 2)
        for n in range(WEIGHT_PERIOD))
    kn_rows = tuple(repeated_prefix(
        b"".join(exact_bf16(weight_numerator(k, n)) for n in range(WEIGHT_PERIOD)), N, 2)
        for k in range(WEIGHT_PERIOD))
    nk = b"".join(nk_rows[n % WEIGHT_PERIOD] for n in range(N))
    kn = b"".join(kn_rows[k % WEIGHT_PERIOD] for k in range(K))
    parts = partition_numerators()
    scale = DENOMINATOR * DENOMINATOR
    partials = b"".join(struct.pack("<f", parts[part][n % WEIGHT_PERIOD] / scale)
                        for part in range(SPLITS) for n in range(N))
    totals = tuple(sum(parts[part][n] for part in range(SPLITS))
                   for n in range(WEIGHT_PERIOD))
    output = b"".join(struct.pack("<f", totals[n % WEIGHT_PERIOD] / scale)
                      for n in range(N))
    result = Fixture(a, nk, kn, partials, output)
    if tuple(map(len, (a, nk, kn, partials, output))) != (K * 2, N * K * 2,
                                                        K * N * 2, SPLITS * N * 4, N * 4):
        raise ValueError("fixed fixture extent drifted")
    return result


def validate_output(actual, expected, label):
    if not isinstance(actual, bytes) or len(actual) != len(expected):
        raise ValueError(label + ": exact byte extent required")
    if actual != expected:
        raise ValueError(label + ": exact dyadic output differs")


CONTROL_KERNELS = {
    "wave": {"symbol": "ferric_qwen3_tp_batch32_wave_gemv_partial_f32_v5",
             "groups": N, "workgroup": 64, "weights": "weights_nk_bf16",
             "scalars": (1, N, K, 1, 2)},
    "mfma": {"symbol": "ferric_qwen3_tp_batch32_mfma_gemm_partial_f32_v5",
             "groups": N // 16, "workgroup": 64, "weights": "weights_kn_bf16",
             "scalars": (1, N, K, 1, 2)},
}
