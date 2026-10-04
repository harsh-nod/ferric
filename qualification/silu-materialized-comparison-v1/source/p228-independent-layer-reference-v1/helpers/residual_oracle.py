"""Independent bit-level oracle for the existing v18 TP2 residual policy.

No receipts, kernel execution, or launch authority are handled here. Binary32
addition uses exact integer multiples of 2**-149 followed by round-to-nearest,
ties-to-even. Neither host floating-point arithmetic nor Rust BF16 helpers are
used. The result is conditional on the caller's authenticated input bit patterns.
"""
import struct


class FinitePolicyError(ValueError):
    """The v18 finite-input/intermediate/output policy would reject this value."""


def _word(value, bits):
    if type(value) is not int or not 0 <= value < 1 << bits:
        raise ValueError(f'exact unsigned {bits}-bit word required')
    return value


def _finite_f32(word, stage):
    _word(word, 32)
    if word & 0x7f800000 == 0x7f800000:
        raise FinitePolicyError(stage)
    return word


def _units(word):
    exponent, fraction = (word >> 23) & 255, word & 0x7fffff
    magnitude = fraction if exponent == 0 else ((1 << 23) | fraction) << (exponent - 1)
    return -magnitude if word >> 31 else magnitude


def add_f32_rne(left, right, stage='addition'):
    """Finite IEEE binary32 addition with gradual underflow and signed zero."""
    _finite_f32(left, stage + ':left')
    _finite_f32(right, stage + ':right')
    total = _units(left) + _units(right)
    if total == 0:
        # RNE cancellation produces positive zero, except -0 plus -0.
        return 0x80000000 if left == right == 0x80000000 else 0
    sign = 0x80000000 if total < 0 else 0
    magnitude = abs(total)
    if magnitude < 1 << 23:
        return sign | magnitude
    shift = max(0, magnitude.bit_length() - 24)
    significand = magnitude >> shift
    if shift:
        remainder = magnitude - (significand << shift)
        half = 1 << (shift - 1)
        if remainder > half or (remainder == half and significand & 1):
            significand += 1
    if significand == 1 << 24:
        significand >>= 1
        shift += 1
    exponent = shift + 1
    if exponent >= 255:
        raise FinitePolicyError(stage + ':result')
    return sign | (exponent << 23) | (significand - (1 << 23))


def narrow_bf16_rne(word):
    """Round finite binary32 to BF16, rejecting a nonfinite rounded result."""
    _finite_f32(word, 'before-bf16')
    upper, lower = word >> 16, word & 0xffff
    if lower > 0x8000 or (lower == 0x8000 and upper & 1):
        upper += 1
    if upper & 0x7f80 == 0x7f80:
        raise FinitePolicyError('bf16-result')
    return upper


def staged_residual_bits(p0_bits, p1_bits, residual_bits):
    """Return (first_f32, rank_sum_f32, residual_sum_f32, final_bf16) words.

    The initial positive zero is intentional: the existing v18 kernel evaluates
    its rank additions starting from +0, including IEEE signed-zero behavior.
    """
    _word(residual_bits, 16)
    first = add_f32_rne(0, p0_bits, 'rank0')
    rank_sum = add_f32_rne(first, p1_bits, 'rank1')
    value = add_f32_rne(rank_sum, residual_bits << 16, 'residual')
    return first, rank_sum, value, narrow_bf16_rne(value)


def ordered_residual_bits(p0_bits, p1_bits, residual_bits):
    return staged_residual_bits(p0_bits, p1_bits, residual_bits)[3]


def residual_vector(p0_f32le, p1_f32le, residual_bf16le):
    """Compute a 4096-element result in memory without mutating any input.

    The caller must authenticate both accepted rank observations and the common
    original residual separately. A numeric array alone is not GPU evidence.
    No partial result is returned if even the final element fails its policy.
    """
    if not all(type(data) is bytes for data in (p0_f32le, p1_f32le, residual_bf16le)):
        raise ValueError('immutable little-endian byte payloads required')
    if (len(p0_f32le), len(p1_f32le), len(residual_bf16le)) != (16384, 16384, 8192):
        raise ValueError('4096 elements required for both partials and residual')
    output = []
    for index, (p0, p1, residual) in enumerate(zip(
        struct.unpack('<4096I', p0_f32le), struct.unpack('<4096I', p1_f32le),
        struct.unpack('<4096H', residual_bf16le), strict=True
    )):
        try:
            output.append(ordered_residual_bits(p0, p1, residual))
        except FinitePolicyError as error:
            raise FinitePolicyError(f'element {index}: {error}') from error
    return struct.pack('<4096H', *output)
