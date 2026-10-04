import hashlib
import os
from pathlib import Path
import struct
import types
import unittest
from fractions import Fraction

import diagnostic as D


ORACLE_PATH = Path(os.environ.get('SILU_RESIDUAL_ORACLE',
    '/home/harmenon/ferric-asrock-42/evidence/finite-resident-integration-v220/'
    'p228-independent-layer-reference-v1/helpers/residual_oracle.py'))
ORACLE_SHA = '551f89f968cad8f63073ca975346c593a31dad624115a93059b28a328e312ef3'


def oracle():
    if ORACLE_PATH.resolve(strict=True) != ORACLE_PATH:
        raise ValueError('canonical frozen BF16 oracle')
    raw = ORACLE_PATH.read_bytes()
    if len(raw) != 4347 or hashlib.sha256(raw).hexdigest() != ORACLE_SHA:
        raise ValueError('unchanged independent BF16 oracle')
    value = types.ModuleType('silu_test_bf16_oracle')
    value.__file__ = str(ORACLE_PATH)
    exec(compile(raw, str(ORACLE_PATH), 'exec'), value.__dict__)
    return value


def fraction(word):
    exponent, bits = (word >> 23) & 255, word & 0x7fffff
    significand = bits if exponent == 0 else bits + (1 << 23)
    shift = -149 if exponent == 0 else exponent - 150
    magnitude = Fraction(significand * (1 << max(shift, 0)), 1 << max(-shift, 0))
    return -magnitude if word & 0x80000000 else magnitude


def fraction_product(left, right):
    """Independent nearest-neighbor search over the ordered finite F32 lattice."""
    wanted = abs(fraction(left) * fraction(right))
    sign = (left ^ right) & 0x80000000
    largest = fraction(0x7f7fffff)
    if wanted > largest:
        if wanted >= Fraction((1 << 128) - (1 << 103)):
            raise ValueError('overflow')
        return sign | 0x7f7fffff
    lo, hi = 0, 0x7f7fffff
    while lo < hi:
        middle = (lo + hi + 1) // 2
        if fraction(middle) <= wanted:
            lo = middle
        else:
            hi = middle - 1
    if fraction(lo) == wanted or lo == 0x7f7fffff:
        return sign | lo
    lower_distance = wanted - fraction(lo)
    upper_distance = fraction(lo + 1) - wanted
    return sign | (lo + int(upper_distance < lower_distance or
                            (upper_distance == lower_distance and lo & 1)))


def packed(values):
    return struct.pack('<' + str(len(values)) + 'H', *values)


def fixture():
    framework = {name: packed([word] * 12288) for name, word in
        (('gate', 0x3f80), ('silu-input', 0x3f80), ('silu', 0x3f00),
         ('up', 0x4000), ('product', 0x3f80))}
    ranks = [{name: packed([word] * 6144) for name, word in
        (('gate', 0x3f80), ('up', 0x4000), ('activation', 0x3f80))} for _ in range(2)]
    return framework, ranks


def replace(body, index, word):
    return body[:index * 2] + struct.pack('<H', word) + body[index * 2 + 2:]


class ExactProductTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.O = oracle()

    def test_signed_zero(self):
        self.assertEqual(D.mul_f32_rne(0x80000000, 0x3f800000), 0x80000000)
        self.assertEqual(D.mul_f32_rne(0x80000000, 0xbf800000), 0)
        self.assertEqual(D.mul_f32_rne(0, 0xbf800000), 0x80000000)

    def test_normal_products_and_sign(self):
        self.assertEqual(D.mul_f32_rne(0x3fc00000, 0x40000000), 0x40400000)
        self.assertEqual(D.mul_f32_rne(0xbfc00000, 0x40000000), 0xc0400000)

    def test_binary32_halfway_rounds_even_both_directions(self):
        self.assertEqual(D.mul_f32_rne(0x3f800001, 0x3fc00000), 0x3fc00002)
        self.assertEqual(D.mul_f32_rne(0x3f800003, 0x3fc00000), 0x3fc00004)

    def test_binary32_subnormal_halfway_and_negative_zero(self):
        self.assertEqual(D.mul_f32_rne(1, 0x3f000000), 0)
        self.assertEqual(D.mul_f32_rne(3, 0x3f000000), 2)
        self.assertEqual(D.mul_f32_rne(0x80000001, 0x3f000000), 0x80000000)

    def test_normal_subnormal_boundary(self):
        self.assertEqual(D.mul_f32_rne(0x00800000, 0x3f000000), 0x00400000)
        self.assertEqual(D.mul_f32_rne(0x007fffff, 0x3f800001), 0x00800000)

    def test_bf16_product_ties_use_existing_oracle(self):
        self.assertEqual(D.product_word(0x3f81, 0x3fc0, self.O.narrow_bf16_rne), 0x3fc2)
        self.assertEqual(D.product_word(0x3f83, 0x3fc0, self.O.narrow_bf16_rne), 0x3fc4)

    def test_bf16_tiny_and_underflow(self):
        narrow = self.O.narrow_bf16_rne
        self.assertEqual(D.product_word(1, 0x3f00, narrow), 0)
        self.assertEqual(D.product_word(3, 0x3f00, narrow), 2)
        self.assertEqual(D.product_word(0x8001, 1, narrow), 0x8000)

    def test_nonfinite_inputs_rejected_even_with_zero(self):
        for value in (0x7f800000, 0xff800000, 0x7fc00000):
            with self.assertRaises(ValueError):
                D.mul_f32_rne(value, 0)
        for value in (0x7f80, 0xff80, 0x7fc0):
            with self.assertRaises(ValueError):
                D.product_word(0, value, self.O.narrow_bf16_rne)

    def test_intermediate_and_final_overflow_rejected(self):
        with self.assertRaises(ValueError):
            D.mul_f32_rne(0x7f7fffff, 0x40000000)
        with self.assertRaises(self.O.FinitePolicyError):
            D.product_word(0x7f7e, 0x3f81, self.O.narrow_bf16_rne)

    def test_invalid_word_types_and_ranges(self):
        for value in (True, -1, 1 << 32, 1.0):
            with self.assertRaises(ValueError):
                D.mul_f32_rne(value, 0x3f800000)
        with self.assertRaises(ValueError):
            D.product_word(65536, 0, self.O.narrow_bf16_rne)

    def test_fraction_neighbor_oracle_deterministic_vectors(self):
        state = 0x22851234
        for index in range(256):
            state = (1664525 * state + 1013904223) & 0xffffffff
            left = state & 0xfeffffff
            state = (1664525 * state + 1013904223) & 0xffffffff
            right = state & 0xfeffffff
            with self.subTest(index=index):
                try:
                    expected = fraction_product(left, right)
                except ValueError:
                    with self.assertRaises(ValueError):
                        D.mul_f32_rne(left, right)
                else:
                    self.assertEqual(D.mul_f32_rne(left, right), expected)

    def test_full_framework_control_and_nonclaims(self):
        framework, ranks = fixture()
        result = D.compare(framework, ranks, self.O.narrow_bf16_rne)
        self.assertEqual(result['framework_product_control']['exact_words'], 12288)
        self.assertEqual([row['native_equal_materialized'] for row in result['ranks']], [6144, 6144])
        for name in ('numerical_acceptance', 'exponential_evaluated', 'native_exp_error_measured',
                     'materialization_only_cause_proven', 'full_model_correctness', 'gpu_execution',
                     'performance_claim', 'production_authority'):
            self.assertIs(result[name], False)
        self.assertIsNone(result['acceptance_threshold'])

    def test_same_gate_partition_uses_native_up(self):
        framework, ranks = fixture()
        ranks[0]['up'] = replace(ranks[0]['up'], 1, 0x3f80)
        ranks[0]['activation'] = replace(ranks[0]['activation'], 1, 0x3f00)
        ranks[0]['gate'] = replace(replace(ranks[0]['gate'], 2, 0x4000), 3, 0x4000)
        ranks[0]['up'] = replace(ranks[0]['up'], 3, 0x3f80)
        result = D.compare(framework, ranks, self.O.narrow_bf16_rne)['ranks'][0]
        self.assertEqual(result['partition_counts'], dict(same_gate_same_up=6141,
            same_gate_different_up=1, different_gate_same_up=1, different_gate_different_up=1))
        self.assertEqual(result['same_gate_elements'], 6142)
        self.assertEqual(result['native_different_materialized'], 0)
        self.assertNotIn(2, result['prediction_indices'])
        self.assertNotIn(3, result['prediction_indices'])

    def test_prediction_mismatch_preserved_without_acceptance(self):
        framework, ranks = fixture()
        ranks[1]['activation'] = replace(ranks[1]['activation'], 5, 0x3f81)
        result = D.compare(framework, ranks, self.O.narrow_bf16_rne)
        self.assertEqual(result['ranks'][1]['mismatch_indices'], [5])
        self.assertEqual(result['ranks'][1]['same_gate_same_up_native_different_framework_indices'], [5])
        self.assertFalse(result['numerical_acceptance'])

    def test_different_gate_is_excluded_not_called_exp_error(self):
        framework, ranks = fixture()
        ranks[0]['gate'] = replace(ranks[0]['gate'], 4, 0x4000)
        ranks[0]['activation'] = replace(ranks[0]['activation'], 4, 0)
        result = D.compare(framework, ranks, self.O.narrow_bf16_rne)
        self.assertEqual(result['ranks'][0]['native_different_materialized'], 0)
        self.assertFalse(result['native_exp_error_measured'])

    def test_gate_silu_input_join_rejected(self):
        framework, ranks = fixture()
        framework['silu-input'] = replace(framework['silu-input'], 0, 0x4000)
        with self.assertRaisesRegex(ValueError, 'gate equals SiLU'):
            D.compare(framework, ranks, self.O.narrow_bf16_rne)

    def test_wrong_framework_product_rejected(self):
        framework, ranks = fixture()
        framework['product'] = replace(framework['product'], 0, 0x3f81)
        with self.assertRaisesRegex(ValueError, 'product control failed'):
            D.compare(framework, ranks, self.O.narrow_bf16_rne)

    def test_extent_roster_and_nonfinite_capture_rejected(self):
        framework, ranks = fixture()
        with self.assertRaises(ValueError):
            D.compare(framework, ranks[:1], self.O.narrow_bf16_rne)
        framework['silu'] = framework['silu'][:-2]
        with self.assertRaisesRegex(ValueError, 'extent'):
            D.compare(framework, ranks, self.O.narrow_bf16_rne)
        framework, ranks = fixture()
        ranks[1]['gate'] = replace(ranks[1]['gate'], 6143, 0x7f80)
        with self.assertRaisesRegex(ValueError, 'nonfinite'):
            D.compare(framework, ranks, self.O.narrow_bf16_rne)


if __name__ == '__main__':
    unittest.main()
