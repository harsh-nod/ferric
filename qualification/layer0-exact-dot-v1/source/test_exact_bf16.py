"""Independent Fraction checks; synthetic CPU coverage, not native evidence."""
from fractions import Fraction
import random
import unittest

import exact_bf16 as E


def power(exponent):
    return Fraction(2 ** exponent) if exponent >= 0 else Fraction(1, 2 ** -exponent)


def reference_value(word):
    exponent, fraction = (word // 128) % 256, word % 128
    if exponent == 255:
        raise ValueError('finite reference input required')
    value = (Fraction(fraction, 128) * power(-126) if exponent == 0
             else (1 + Fraction(fraction, 128)) * power(exponent - 127))
    return -value if word >= 32768 else value


def reference_round(value):
    if value == 0:
        return 0
    sign, magnitude = (0x8000 if value < 0 else 0), abs(value)
    # Independent nearest-neighbor search over the positive BF16 lattice.
    low, high = 0, 0x7f7f
    while low < high:
        middle = (low + high + 1) // 2
        if reference_value(middle) <= magnitude:
            low = middle
        else:
            high = middle - 1
    upper = low + 1
    upper_value = power(128) if upper == 0x7f80 else reference_value(upper)
    lower_distance = abs(magnitude - reference_value(low))
    upper_distance = abs(upper_value - magnitude)
    chosen = upper if (upper_distance < lower_distance or
                      (upper_distance == lower_distance and upper % 2 == 0)) else low
    return sign | chosen


def reference_dot(left, right):
    return sum((reference_value(a) * reference_value(b) for a, b in zip(left, right)), Fraction())


class ExactBf16Tests(unittest.TestCase):
    def assert_dot(self, left, right):
        total, rounded = E.exact_dot(left, right)
        value = reference_dot(left, right)
        self.assertEqual(Fraction(total, 1 << 266), value)
        self.assertEqual(rounded, reference_round(value))
        return total, rounded

    def test_literal_integer_units_and_signed_zero(self):
        expected = {0: 0, 0x8000: 0, 1: 1, 0x8001: -1, 0x007f: 127,
                    0x0080: 128, 0x3f80: 1 << 133, 0xbf80: -(1 << 133),
                    0x7f7f: 255 << 253}
        for word, units in expected.items():
            self.assertEqual(E.decode_units(word), units)
            self.assertEqual(Fraction(units, 1 << 133), reference_value(word))

    def test_nonfinite_inputs_and_nonword_types_refuse(self):
        for word in (-1, 65536, True, 1.0, '1', None, 0x7f80, 0xff80, 0x7fc0, 0xffff):
            with self.subTest(word=word), self.assertRaises(ValueError):
                E.decode_units(word)
        with self.assertRaises(ValueError):
            E.dot_units([0, 0x7fc0], [0x3f80, 0])
        with self.assertRaises(ValueError):
            E.dot_units([0], [0x7f80])

    def test_extent_mismatch_refuses_and_empty_dot_is_zero(self):
        with self.assertRaises(ValueError):
            E.exact_dot([0x3f80], [])
        self.assertEqual(E.exact_dot([], []), (0, 0))

    def test_round_and_distance_require_integer_totals(self):
        for total in (True, 1.0, '1', None):
            with self.assertRaises(ValueError):
                E.round_dot_units(total)
            with self.assertRaises(ValueError):
                E.distance_units(total, 0)
        with self.assertRaises(ValueError):
            E.distance_units(0, 0x7f80)

    def test_simple_dot_has_no_intermediate_rounding(self):
        total, word = self.assert_dot([0x3f80, 0x4000], [0x3f80, 0x4040])
        self.assertEqual((total, word), (7 << 266, 0x40e0))

    def test_even_and_odd_normal_halfway_products(self):
        self.assertEqual(self.assert_dot([0x3f80, 0x3d80], [0x3f80, 0x3d80])[1], 0x3f80)
        self.assertEqual(self.assert_dot([0x3f80, 0x3e40], [0x3f80, 0x3d80])[1], 0x3f82)
        self.assertEqual(self.assert_dot([0xbf80, 0xbd80], [0x3f80, 0x3d80])[1], 0xbf80)

    def test_subnormal_halfway_underflow_and_negative_zero(self):
        for left, right, expected in ((1, 0x3f00, 0), (3, 0x3f00, 2),
                                      (0x8001, 0x3f00, 0x8000), (0x8003, 0x3f00, 0x8002)):
            self.assertEqual(self.assert_dot([left], [right])[1], expected)
        self.assertEqual(self.assert_dot([1], [1])[1], 0)

    def test_largest_subnormal_normal_boundary_ties_even(self):
        midpoint = (255 << 132)
        self.assertEqual(E.round_dot_units(midpoint - 1), 0x007f)
        self.assertEqual(E.round_dot_units(midpoint), 0x0080)
        self.assertEqual(E.round_dot_units(midpoint + 1), 0x0080)
        self.assertEqual(E.round_dot_units(-midpoint), 0x8080)
        self.assertEqual(self.assert_dot([0x007f, 1], [0x3f80, 0x3f00])[1], 0x0080)

    def test_significand_carry_at_normal_exponent_boundary(self):
        midpoint = 511 << 258
        for value, expected in ((midpoint - 1, 0x3fff), (midpoint, 0x4000), (midpoint + 1, 0x4000)):
            self.assertEqual(E.round_dot_units(value), expected)
            self.assertEqual(E.round_dot_units(value), reference_round(Fraction(value, 1 << 266)))

    def test_overflow_threshold_tie_and_sign(self):
        midpoint = 511 << 385
        for value, expected in ((midpoint - 1, 0x7f7f), (midpoint, 0x7f80),
                                (midpoint + 1, 0x7f80), (-midpoint, 0xff80)):
            self.assertEqual(E.round_dot_units(value), expected)
            self.assertEqual(E.round_dot_units(value), reference_round(Fraction(value, 1 << 266)))
        self.assertEqual(self.assert_dot([0x7f7f], [0x4000])[1], 0x7f80)

    def test_exact_cancellation_survives_products_above_fp32_range(self):
        self.assertEqual(self.assert_dot([0x7f7f, 0xff7f], [0x7f7f, 0x7f7f]), (0, 0))
        self.assertEqual(self.assert_dot([0x7f7f, 0x3f80, 0xff7f],
                                       [0x7f7f, 0x3f80, 0x7f7f])[1], 0x3f80)

    def test_exact_sum_is_order_independent_and_cancellation_zero_is_positive(self):
        left, right = [0x4b80, 0x3f80, 0xcb80], [0x3f80] * 3
        self.assertEqual(self.assert_dot(left, right)[1], 0x3f80)
        self.assertEqual(E.exact_dot(left[::-1], right[::-1]), E.exact_dot(left, right))
        self.assertEqual(self.assert_dot([0x8000, 0], [0x3f80, 0xbf80]), (0, 0))

    def test_exact_distance_does_not_presume_native_or_framework_is_right(self):
        total = E.decode_units(0xb8c2) << 133
        self.assertEqual(E.distance_units(total, 0xb8c2), 0)
        self.assertGreater(E.distance_units(total, 0xb8c3), 0)
        midpoint = ((E.decode_units(0xb8c2) + E.decode_units(0xb8c3)) << 132)
        self.assertEqual(E.distance_units(midpoint, 0xb8c2), E.distance_units(midpoint, 0xb8c3))
        self.assertEqual(E.round_dot_units(midpoint), 0xb8c2)

    def test_representable_values_round_trip_across_every_exponent(self):
        for exponent in range(255):
            for fraction in (0, 1, 63, 64, 126, 127):
                for sign in (0, 0x8000):
                    word = sign | (exponent << 7) | fraction
                    total = E.decode_units(word) << 133
                    self.assertEqual(E.round_dot_units(total), 0 if word == 0x8000 else word)

    def test_neighbor_midpoints_and_one_integer_unit_on_both_sides(self):
        for low in (0, 1, 2, 126, 127, 128, 255, 256, 0x3f7f, 0x3f80, 0x3f81, 0x7f7e):
            midpoint = (E.decode_units(low) + E.decode_units(low + 1)) << 132
            for delta in (-1, 0, 1):
                for sign in (1, -1):
                    total = sign * (midpoint + delta)
                    self.assertEqual(E.round_dot_units(total), reference_round(Fraction(total, 1 << 266)))

    def test_finite_adversarial_scalar_cross_product_matches_fraction(self):
        words = (0, 0x8000, 1, 0x007f, 0x0080, 0x0081, 0x3f00, 0x3f7f,
                 0x3f80, 0x3f81, 0xbf80, 0x7f7f, 0xff7f)
        for a in words:
            for b in words:
                self.assert_dot([a], [b])

    def test_seeded_mixed_sign_dots_match_independent_fraction(self):
        generator = random.Random(0x228)
        for length in (1, 2, 3, 7, 32, 64, 128):
            left = [generator.randrange(255) * 128 + generator.randrange(128)
                    + generator.randrange(2) * 32768 for _ in range(length)]
            right = [generator.randrange(255) * 128 + generator.randrange(128)
                     + generator.randrange(2) * 32768 for _ in range(length)]
            self.assert_dot(left, right)

    def test_full_4096_term_shape_matches_fraction_without_float(self):
        left = [0x3f80, 0xbf80, 0x3d80, 1] * 1024
        right = [0x3f80, 0x3f80, 0x3d80, 1] * 1024
        self.assert_dot(left, right)


if __name__ == '__main__':
    unittest.main(verbosity=2)
