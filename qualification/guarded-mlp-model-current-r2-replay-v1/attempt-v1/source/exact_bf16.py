"""Integer-only exact BF16 dot products, not a floating-point acceptance rule.

Every finite BF16 is an integer multiple of 2**-133. Products and their exact
sum therefore use 2**-266 units. Only the final conversion rounds, with RNE.
Exact cancellation has canonical +0; negative nonzero underflow retains -0.
Overflow returns the appropriately signed infinity encoding, never a clamp.
"""


def decode_units(word):
    """Return a finite BF16 word's signed integer value in 2**-133 units."""
    if type(word) is not int or not 0 <= word <= 0xffff:
        raise ValueError('BF16 word must be an unsigned 16-bit integer')
    exponent = (word >> 7) & 0xff
    fraction = word & 0x7f
    if exponent == 0xff:
        raise ValueError('nonfinite BF16 input')
    magnitude = fraction if exponent == 0 else (128 + fraction) << (exponent - 1)
    return -magnitude if word & 0x8000 else magnitude


def dot_units(left, right):
    """Exact sum in 2**-266 units; callers own shape and resource bounds."""
    if len(left) != len(right):
        raise ValueError('equal dot-product extents required')
    total = 0
    for a, b in zip(left, right):
        total += decode_units(a) * decode_units(b)
    return total


def round_dot_units(total):
    """Round an exact 2**-266-unit integer once to a BF16 encoding."""
    if type(total) is not int:
        raise ValueError('exact dot total must be an integer')
    if total == 0:
        return 0
    sign = 0x8000 if total < 0 else 0
    magnitude = abs(total)
    # Subnormals have a fixed quantum. Normals retain eight significand bits.
    shift = max(133, magnitude.bit_length() - 8)
    significand, remainder = divmod(magnitude, 1 << shift)
    halfway = 1 << (shift - 1)
    if remainder > halfway or (remainder == halfway and significand & 1):
        significand += 1
    if significand < 128:
        return sign | significand
    if significand == 256:
        significand = 128
        shift += 1
    exponent = shift - 132
    if exponent >= 0xff:
        return sign | 0x7f80
    return sign | (exponent << 7) | (significand - 128)


def exact_dot(left, right):
    """Return (exact integer sum at scale 2**-266, once-rounded BF16 word)."""
    total = dot_units(left, right)
    return total, round_dot_units(total)


def distance_units(total, observed_word):
    """Exact absolute error of a finite observed BF16, in 2**-266 units."""
    if type(total) is not int:
        raise ValueError('exact dot total must be an integer')
    return abs(total - (decode_units(observed_word) << 133))
