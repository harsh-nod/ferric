"""Independent Fraction nearest-neighbor tests; no hardware or model fixture."""
from fractions import Fraction
import unittest

import fp32_replay as R


def value(word):
    exponent, fraction = (word >> 23) & 255, word & 0x7fffff
    if exponent == 255:
        raise ValueError('finite Fraction input')
    significand = fraction if exponent == 0 else fraction + (1 << 23)
    power = -149 if exponent == 0 else exponent - 150
    result = Fraction(significand << power) if power >= 0 else Fraction(significand, 1 << -power)
    return -result if word >> 31 else result


def nearest(number, negative_zero=False):
    """Search the ordered finite encoding space, independently of bit-length RNE."""
    if not number:
        return 0x80000000 if negative_zero else 0
    sign = 0x80000000 if number < 0 else 0
    number = abs(number)
    maximum = 0x7f7fffff
    if number >= value(maximum) + (1 << 103):
        return sign | 0x7f800000
    low, high = 0, maximum
    while low < high:
        middle = (low + high + 1) // 2
        if value(middle) <= number:
            low = middle
        else:
            high = middle - 1
    if low == maximum or value(low) == number:
        return sign | low
    upper = low + 1
    below, above = number - value(low), value(upper) - number
    chosen = low if below < above or (below == above and low % 2 == 0) else upper
    return sign | chosen


def units(number):
    scaled = number * (1 << 266)
    assert scaled.denominator == 1
    return scaled.numerator


def reference_rank(left, right):
    lanes = [0] * 64
    for lane in range(64):
        accumulator = 0
        for index in range(lane, 2048, 64):
            product = nearest(value(left[index] << 16) * value(right[index] << 16),
                              bool((left[index] ^ right[index]) & 0x8000))
            accumulator = nearest(value(accumulator) + value(product),
                                  accumulator == product == 0x80000000)
        lanes[lane] = accumulator
    width = 1
    while width < 64:
        old = lanes[:]
        lanes = [nearest(value(old[i]) + value(old[i ^ width]),
                         old[i] == old[i ^ width] == 0x80000000) for i in range(64)]
        width *= 2
    return lanes[0]


class Fp32ReplayTests(unittest.TestCase):
    def test_encoding_decoding_matches_independent_fraction(self):
        for word in (0, 1, 2, 0x7fffff, 0x800000, 0x800001, 0x3f000001,
                     0x3f800000, 0x4b800000, 0x7f7fffff):
            for sign in (0, 0x80000000):
                with self.subTest(word=hex(word | sign)):
                    self.assertEqual(Fraction(R.f32_units(word | sign), 1 << 266), value(word | sign))
                    self.assertEqual(R.round_f32(units(value(word | sign)), sign != 0), word | sign)

    def test_fraction_nearest_neighbor_midpoints_and_neighbors(self):
        for low in (0, 1, 2, 0x7ffffe, 0x7fffff, 0x800000, 0x3effffff,
                    0x3f000000, 0x3f7fffff, 0x3f800000, 0x3f800001,
                    0x4b800000, 0x7f7ffffe):
            middle = (value(low) + value(low + 1)) / 2
            for delta in (-Fraction(1, 1 << 266), Fraction(0), Fraction(1, 1 << 266)):
                for sign in (-1, 1):
                    number = sign * (middle + delta)
                    with self.subTest(low=hex(low), delta=delta, sign=sign):
                        self.assertEqual(R.round_f32(units(number)), nearest(number))

    def test_gradual_underflow_and_signed_zero(self):
        half = Fraction(1, 1 << 150)
        for number in (half, -half, half + Fraction(1, 1 << 266),
                       -half - Fraction(1, 1 << 266), 3 * half, -3 * half):
            self.assertEqual(R.round_f32(units(number)), nearest(number))
        self.assertEqual(R.add_f32(0x80000000, 0x80000000), 0x80000000)
        self.assertEqual(R.add_f32(0, 0x80000000), 0)
        self.assertEqual(R.add_f32(0x3f800000, 0xbf800000), 0)
        self.assertEqual(R.narrow_f32(0x80000000), 0x8000)

    def test_overflow_threshold_both_signs(self):
        threshold = value(0x7f7fffff) + (1 << 103)
        for number in (threshold - 1, threshold, threshold + 1, 2 * threshold):
            for sign in (-1, 1):
                self.assertEqual(R.round_f32(units(sign * number)), nearest(sign * number))
        self.assertEqual(R.add_f32(0x7f7fffff, 0x7f7fffff), 0x7f800000)

    def test_addition_matches_fraction_not_host_float(self):
        values = (0, 0x80000000, 1, 0x80000001, 0x7fffff, 0x800000,
                  0x3f800000, 0xbf800000, 0x33800000, 0x4b800000,
                  0xcb800000, 0x7f7fffff, 0xff7fffff)
        for left in values:
            for right in values:
                self.assertEqual(R.add_f32(left, right),
                                 nearest(value(left) + value(right), left == right == 0x80000000))

    def test_lane_cancellation_preserves_actual_association(self):
        left, right = [0] * 2048, [0x3f80] * 2048
        left[0], left[64], left[128] = 0x4b80, 0x3f80, 0xcb80
        report = R.replay_rank(left, right)
        self.assertEqual(report['word'], 0)
        self.assertEqual(report['exact_units'], 1 << 266)
        self.assertEqual(report['word'], reference_rank(left, right))
        left[64], left[128] = left[128], left[64]
        self.assertEqual(R.replay_rank(left, right)['word'], 0x3f800000)

    def test_all_lanes_all_steps_and_xor_tree_fraction_reference(self):
        values = (0, 0x8000, 0x3f80, 0xbf80, 0x3a80, 0xba80, 0x4100, 0xc100)
        left = [values[(i * 3 + i // 64) % len(values)] for i in range(2048)]
        right = [values[(i * 5 + i // 13 + 1) % len(values)] for i in range(2048)]
        report = R.replay_rank(left, right)
        self.assertEqual(report['word'], reference_rank(left, right))
        exact = sum((value(a << 16) * value(b << 16) for a, b in zip(left, right)), Fraction(0))
        self.assertEqual(report['exact_units'], units(exact))
        self.assertEqual(report['rounded_products'], 0)

    def test_product_underflow_and_subnormal_counters(self):
        left, right = [0] * 2048, [0] * 2048
        left[0] = right[0] = 1
        left[1], right[1] = 1, 0x3780
        report = R.replay_rank(left, right)
        self.assertEqual(report['rounded_products'], 1)
        self.assertEqual(report['subnormal_products'], 1)
        self.assertGreater(report['subnormal_sums'], 0)
        self.assertEqual(report['word'], 1)
        self.assertEqual(report['word'], reference_rank(left, right))

    def test_bf16_boundary_materializes_before_residual(self):
        even = R.combined(0x3f800000, 0x3b800000, 0xbf80)
        self.assertEqual(even, dict(sum_word=0x3f808000, projection=0x3f80, residual=0))
        fused = R.narrow_f32(R.add_f32(even['sum_word'], 0xbf800000))
        self.assertEqual(fused, 0x3b80)
        self.assertNotEqual(fused, even['residual'])
        odd = R.combined(0x3f800000, 0x3c400000, 0xbf80)
        self.assertEqual(odd['projection'], 0x3f82)
        self.assertEqual(odd['residual'], 0x3c80)

    def test_partials_are_not_individually_narrowed(self):
        report = R.combined(0x3f808000, 0xbb800000, 0)
        self.assertEqual(report['projection'], 0x3f80)
        wrong = R.narrow_f32(R.add_f32(R.bf16_f32(R.narrow_f32(0x3f808000)), 0xbb800000))
        self.assertEqual(wrong, 0x3f7f)
        self.assertNotEqual(wrong, report['projection'])

    def test_midpoint_distance_has_explicit_exact_scale(self):
        middle = (value(0x3f800000) + value(0x3f810000)) / 2
        report = R.midpoint(units(middle), 0x3f80, 0x3f81)
        self.assertEqual(report['distance_numerator'], '0')
        self.assertTrue(report['left_even'])
        self.assertFalse(report['right_even'])
        for sign in (-1, 1):
            actual = middle + Fraction(sign, 1 << 266)
            report = R.midpoint(units(actual), 0x3f80, 0x3f81)
            self.assertEqual(Fraction(int(report['distance_numerator']), 1 << 267), abs(actual - middle))
            self.assertEqual(int(report['signed_offset_numerator']), 2 * sign)

    def test_nonfinite_shape_and_overflow_refusals(self):
        for word in (-1, 1 << 32, True, 0x7f800000, 0xff800000, 0x7fc00001):
            with self.assertRaises(ValueError):
                R.f32_units(word)
        for left, right in (([], []), ([0] * 2047, [0] * 2048), ([0] * 2048, [0] * 2049)):
            with self.assertRaises(ValueError):
                R.replay_rank(left, right)
        bad = [0] * 2048
        bad[17] = 0x7f80
        with self.assertRaises(ValueError):
            R.replay_rank(bad, [0] * 2048)
        with self.assertRaises(ValueError):
            R.replay_rank([0x7f7f] * 2048, [0x7f7f] * 2048)
        with self.assertRaises(ValueError):
            R.combined(0x7f7fffff, 0x7f7fffff, 0)
        with self.assertRaises(ValueError):
            R.combined(0, 0, 0x7f80)
        with self.assertRaises(ValueError):
            R.midpoint(0, 0, 0x8000)


if __name__ == '__main__':
    unittest.main()
