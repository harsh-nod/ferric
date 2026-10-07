"""Integer RNE model of the fixed O-projection FP32 association, not GPU code."""
from exact_bf16 import decode_units, round_dot_units

SCALE = -266


def require(ok, message):
    if not ok:
        raise ValueError(message)


def f32_units(word):
    require(type(word) is int and 0 <= word <= 0xffffffff, 'FP32 encoding')
    exponent, fraction = (word >> 23) & 255, word & 0x7fffff
    require(exponent != 255, 'finite FP32 encoding')
    magnitude = fraction if exponent == 0 else (0x800000 | fraction) << (exponent - 1)
    magnitude <<= 117
    return -magnitude if word >> 31 else magnitude


def round_f32(total, negative_zero=False):
    require(type(total) is int and type(negative_zero) is bool, 'integer exact FP32 input')
    if total == 0:
        return 0x80000000 if negative_zero else 0
    sign = 0x80000000 if total < 0 else 0
    magnitude = abs(total)
    shift = max(117, magnitude.bit_length() - 24)
    significand, remainder = divmod(magnitude, 1 << shift)
    midpoint = 1 << (shift - 1)
    if remainder > midpoint or (remainder == midpoint and significand & 1):
        significand += 1
    if significand < 0x800000:
        return sign | significand
    if significand == 0x1000000:
        significand = 0x800000
        shift += 1
    exponent = shift - 116
    if exponent >= 255:
        return sign | 0x7f800000
    return sign | exponent << 23 | (significand - 0x800000)


def add_f32(left, right):
    total = f32_units(left) + f32_units(right)
    return round_f32(total, left == right == 0x80000000)


def narrow_f32(word):
    total = f32_units(word)
    return 0x8000 if word == 0x80000000 else round_dot_units(total)


def bf16_f32(word):
    decode_units(word)
    return word << 16


def finite(word):
    return word & 0x7f800000 != 0x7f800000


def replay_rank(left_words, right_words, left_units=None, table=None):
    require(len(left_words) == len(right_words) == 2048, 'one 2048-term O rank')
    left = [decode_units(word) for word in left_words] if left_units is None else left_units
    require(len(left) == 2048, 'decoded rank extent')
    lanes = [0] * 64
    exact, absolute = 0, 0
    rounded_products = subnormal_products = subnormal_sums = 0
    for step in range(32):
        for lane in range(64):
            index = step * 64 + lane
            right = decode_units(right_words[index]) if table is None else table[right_words[index]]
            require(right is not None, 'finite O weight')
            product = left[index] * right
            exact += product
            absolute += abs(product)
            word = round_f32(product, bool((left_words[index] ^ right_words[index]) & 0x8000))
            require(finite(word), 'finite FP32 product')
            rounded_products += f32_units(word) != product
            subnormal_products += 0 < (word & 0x7fffffff) < 0x800000
            lanes[lane] = add_f32(lanes[lane], word)
            require(finite(lanes[lane]), 'finite lane sum')
            subnormal_sums += 0 < (lanes[lane] & 0x7fffffff) < 0x800000
    for mask in (1, 2, 4, 8, 16, 32):
        previous = lanes
        lanes = [add_f32(previous[lane], previous[lane ^ mask]) for lane in range(64)]
        require(all(finite(word) for word in lanes), 'finite XOR tree')
        subnormal_sums += sum(0 < (word & 0x7fffffff) < 0x800000 for word in lanes)
    return dict(word=lanes[0], exact_units=exact, absolute_units=absolute,
                rounded_products=rounded_products, subnormal_products=subnormal_products,
                subnormal_sums=subnormal_sums)


def combined(partial0, partial1, residual):
    summed = add_f32(add_f32(0, partial0), partial1)
    require(finite(summed), 'finite ordered TP sum')
    projection = narrow_f32(summed)
    require(projection & 0x7f80 != 0x7f80, 'finite materialized projection')
    value = add_f32(bf16_f32(projection), bf16_f32(residual))
    require(finite(value), 'finite residual FP32 addition')
    output = narrow_f32(value)
    require(output & 0x7f80 != 0x7f80, 'finite residual BF16 output')
    return dict(sum_word=summed, projection=projection, residual=output)


def midpoint(total, left, right):
    """Distance to the exact midpoint of two distinct finite BF16 values."""
    a, b = decode_units(left) << 133, decode_units(right) << 133
    require(a != b, 'distinct numerical BF16 values')
    # Numerators use half of one 2^-266 dot unit to retain exact midpoints.
    return dict(distance_numerator=str(abs(2 * total - a - b)),
                signed_offset_numerator=str(2 * total - a - b),
                scale_power_of_two=-267,
                left_even=(left & 1) == 0, right_even=(right & 1) == 0)
