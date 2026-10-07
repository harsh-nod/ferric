"""Synthetic exact-arithmetic tests; no model, native capture, Torch, or GPU."""
from fractions import Fraction
import struct
import unittest

import head as H


def fraction(word):
    sign = -1 if word & 32768 else 1
    exponent, mantissa = (word >> 7) & 255, word & 127
    if exponent == 0:
        return sign * Fraction(mantissa, 1 << 133)
    return sign * Fraction(128 + mantissa) * Fraction(2) ** (exponent - 134)


def header():
    return {'lm_head.weight': {'dtype': 'BF16', 'shape': [151936, 4096], 'data_offsets': [0, 1244659712]}}


class HeadTests(unittest.TestCase):
    def test_decimal_is_exact_not_float(self):
        self.assertEqual(H.decimal_dyadic(0), '0')
        self.assertEqual(H.decimal_dyadic(-3, 1), '-1.5')
        self.assertEqual(H.decimal_dyadic(315, 4), '19.6875')
        self.assertEqual(Fraction(H.decimal_dyadic(1)), Fraction(1, 1 << 266))

    def test_exact_decode_matches_fraction(self):
        for word in (0, 0x8000, 1, 127, 128, 0x3f80, 0xbf81, 0x7f7f, 0xff7f):
            self.assertEqual(Fraction(H.units(word), 1 << 133), fraction(word))
        for invalid in (True, -1, 65536, 0x7f80, 0xff80, 0x7fc1):
            with self.assertRaises(ValueError):
                H.units(invalid)

    def test_dot_cancellation_and_exact_products(self):
        left = [0x3f80, 0xbf80, 1, 0x3f81] + [0] * 4092
        right = [0x3f80, 0x3f80, 1, 0x3f81] + [0] * 4092
        expected = sum(fraction(a) * fraction(b) for a, b in zip(left, right))
        self.assertEqual(Fraction(H.exact_dot(left, right), 1 << 266), expected)
        with self.assertRaises(ValueError):
            H.exact_dot(left[:-1], right)

    def test_roundtrip_finite_samples(self):
        for word in range(0, 0x7f80, 127):
            for sign in (0, 32768):
                actual = H.nearest_bf16(H.units(word | sign) << 133)
                self.assertEqual(actual, word | sign if word else 0)

    def test_ties_even_and_signs(self):
        for lower in (0x3f80, 0x3f81, 0x419d, 0x419e, 0x7f7d):
            value = (fraction(lower) + fraction(lower + 1)) / 2
            exact = int(value * (1 << 266))
            expected = lower if lower % 2 == 0 else lower + 1
            self.assertEqual(H.nearest_bf16(exact), expected)
            self.assertEqual(H.nearest_bf16(-exact), expected | 32768)
            self.assertEqual(H.nearest_bf16(exact - 1), lower)
            self.assertEqual(H.nearest_bf16(exact + 1), lower + 1)

    def test_subnormal_normal_and_signed_underflow(self):
        self.assertEqual(H.nearest_bf16(1), 0)
        self.assertEqual(H.nearest_bf16(-1), 0x8000)
        self.assertEqual(H.nearest_bf16(1 << 132), 0)
        self.assertEqual(H.nearest_bf16((1 << 132) + 1), 1)
        edge = int((fraction(127) + fraction(128)) / 2 * (1 << 266))
        self.assertEqual(H.nearest_bf16(edge), 128)

    def test_overflow_boundary(self):
        threshold = (fraction(0x7f7f) + Fraction(2) ** 119) * (1 << 266)
        self.assertEqual(H.nearest_bf16(int(threshold) - 1), 0x7f7f)
        self.assertEqual(H.nearest_bf16(int(threshold)), 0x7f80)
        self.assertEqual(H.nearest_bf16(-int(threshold)), 0xff80)

    def test_fraction_nearest_independent_local_oracle(self):
        for lower in (1, 126, 128, 1024, 0x3f7f, 0x419d, 0x6001):
            lo, hi = fraction(lower), fraction(lower + 1)
            for part in (1, 2, 3):
                value = lo + (hi - lo) * Fraction(part, 4)
                candidates = range(max(0, lower - 2), lower + 4)
                expected = min(candidates, key=lambda w: (abs(fraction(w) - value), w % 2))
                self.assertEqual(H.nearest_bf16(int(value * (1 << 266))), expected)

    def test_shard_layout_two_row_offsets(self):
        start, rows = H.tensor_layout(header(), 120, 1244659840)
        self.assertEqual(start, 128)
        self.assertEqual(rows, {2: (16512, 8192), 9112: (74645632, 8192)})

    def test_header_rejects_shape_dtype_extent_and_duplicate(self):
        for key, value in [('dtype', 'F16'), ('shape', [True, 4096]), ('data_offsets', [0, 1244659711])]:
            item = header()
            item['lm_head.weight'][key] = value
            with self.assertRaises(ValueError):
                H.tensor_layout(item, 120, 1244659840)
        for length in (0, 1 << 21):
            with self.assertRaises(ValueError):
                H.tensor_layout(header(), length, 1244659840)
        with self.assertRaises(ValueError):
            H.parse(b'{"a":1,"a":2}')

    def test_payload_extent_and_nonfinite(self):
        with self.assertRaises(ValueError):
            H.payload(bytes(10))
        value = bytearray(H.PAYLOAD_BYTES)
        struct.pack_into('<H', value, H.FINAL_OFFSET, 0x7fc1)
        with self.assertRaises(ValueError):
            H.payload(bytes(value))

    def test_four_dots_and_delta_decomposition(self):
        a, b = bytearray(H.PAYLOAD_BYTES), bytearray(H.PAYLOAD_BYTES)
        struct.pack_into('<H', a, H.FINAL_OFFSET, 0x4000)
        struct.pack_into('<H', b, H.FINAL_OFFSET, 0x3f80)
        rows = {2: struct.pack('<H', 0x3f80) + bytes(8190),
                9112: struct.pack('<H', 0xbf80) + bytes(8190)}
        result = H.analyze(bytes(a), bytes(b), rows)
        self.assertEqual(result['dot_count'], 4)
        self.assertEqual(int(result['margin_change_units_2_pow_minus266']), 2 << 266)
        self.assertFalse(result['numerical_acceptance'])
        self.assertFalse(result['mfma_emulation'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
