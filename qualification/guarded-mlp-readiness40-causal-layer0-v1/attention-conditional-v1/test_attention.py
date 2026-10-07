from decimal import Decimal
import struct
import unittest

import attention as G
import attention_reference as A
import head as H


def packed(words):
    return struct.pack('<%dH' % len(words), *words)


def constant_case(tokens, value, output=None):
    return dict(query=bytes(4096), used_key=bytes(tokens * 1024),
        used_value=packed([value] * (tokens * 512)),
        attention=packed([value if output is None else output] * 2048))


class AttentionTests(unittest.TestCase):
    def test_original_policy_and_native_scale_are_unchanged(self):
        self.assertEqual(A.SCALE_BITS, 0x3db504f3)
        self.assertEqual(A.POLICY, dict(schema='ferric-attention-tolerance-v214', exact_position_zero=True,
            bf16_steps=1, cancellation_coefficient=5e-5,
            acceptance='exact at position 0; otherwise steps <= 1 OR abs_error <= 5e-5 * per_kv_head_max_abs_V',
            reference_rounding='FP64 oracle -> FP32 ties-to-even -> BF16 ties-to-even',
            max_abs_V='maximum magnitude over all causal value words for the mapped KV head',
            nonfinite='reject query, every causal K/V word, scores and outputs; future poison is never read',
            gpu_execution=False, adaptive_tolerance=False))

    def test_position_zero_is_exact_value_for_every_head_and_channel(self):
        values = [0x3f80 + (h * 128 + d) % 64 for h in range(4) for d in range(128)]
        expected = [values[(h // 4) * 128 + d] for h in range(16) for d in range(128)]
        case = dict(query=packed([0x3f00] * 2048), used_key=packed([0x3f80] * 512),
                    used_value=packed(values), attention=packed(expected))
        result, _ = G.conditional(case, 0)
        self.assertTrue(result['passed'])
        self.assertEqual(result['metrics']['exact'], 2048)
        changed = dict(case, attention=packed([expected[0] + 1] + expected[1:]))
        self.assertFalse(G.conditional(changed, 0)[0]['passed'])

    def test_uniform_three_token_cancellation_and_corruption(self):
        case = dict(query=bytes(4096), used_key=bytes(3 * 1024),
                    used_value=packed([0x3f80] * 512 + [0xbf80] * 512 + [0] * 512),
                    attention=bytes(4096))
        result, expected = G.conditional(case, 2)
        self.assertTrue(result['passed'])
        self.assertTrue(all(x == 0 for x in expected))
        changed = dict(case, attention=packed([0x3f80] + [0] * 2047))
        self.assertFalse(G.conditional(changed, 2)[0]['passed'])

    def test_each_side_uses_its_own_inputs_not_the_other_outputs(self):
        native, framework = constant_case(2, 0x3f80), constant_case(2, 0xbf80)
        self.assertTrue(G.conditional(native, 1)[0]['passed'])
        self.assertTrue(G.conditional(framework, 1)[0]['passed'])
        self.assertFalse(G.conditional(dict(native, attention=framework['attention']), 1)[0]['passed'])

    def test_cache_permutation_preserves_token_head_channel(self):
        raw = packed([h * 768 + t * 128 + d for h in range(8) for t in range(6) for d in range(128)])
        for rank in (0, 1):
            expected = packed([h * 768 + t * 128 + d for t in range(6)
                               for h in range(rank * 4, rank * 4 + 4) for d in range(128)])
            self.assertEqual(G.cache_rows(raw, 5, rank), expected)
        for args in ((raw, 5, True), (raw, 5, 2), (raw[:-2], 5, 0), (raw, 6, 0)):
            with self.subTest(args=args[1:]), self.assertRaises(ValueError):
                G.cache_rows(*args)

    def test_nonfinite_wrong_extent_and_future_words_refused(self):
        for role in ('query', 'used_key', 'used_value', 'attention'):
            for mutation in ('nan', 'extra', 'missing'):
                case = constant_case(3, 0x3f80)
                if mutation == 'nan': case[role] = packed([0x7fc1]) + case[role][2:]
                elif mutation == 'extra': case[role] += b'\0\0'
                else: del case[role]
                with self.subTest(role=role, mutation=mutation), self.assertRaises(ValueError):
                    G.conditional(case, 2)
        with self.assertRaises(ValueError):
            G.analyze([])

    def test_decimal_rounding_has_signed_ties_and_subnormal_boundaries(self):
        for value, word in [('1.00390625', 0x3f80), ('1.01171875', 0x3f82),
                            ('-1.00390625', 0xbf80), ('-0', 0x8000), ('0', 0)]:
            self.assertEqual(G.decimal_bf16(Decimal(value)), word)
        from decimal import localcontext
        with localcontext() as context:
            context.prec = 160
            quantum = Decimal(2) ** -133
            self.assertEqual(G.decimal_bf16(quantum), 1)
            self.assertEqual(G.decimal_bf16(quantum / 2), 0)
            self.assertEqual(G.decimal_bf16(-quantum / 2), 0x8000)
        with self.assertRaises(ValueError):
            G.decimal_bf16(Decimal('NaN'))

    def test_high_precision_zero_score_mean_is_independent_of_online_order(self):
        case = dict(query=bytes(4096), used_key=bytes(3 * 1024),
                    used_value=packed([0x3f80] * 512 + [0xbf80] * 512 + [0x3f80] * 512),
                    attention=packed([0x3eab] * 2048))
        for precision in (80, 160):
            row = G.high_precision_scalar(case, 2, 1804, precision)
            self.assertEqual(row['direct_bf16_word'], 0x3eab)
            self.assertEqual(row['exact_qk_dot_units_2_pow_minus266'], ['0', '0', '0'])
            self.assertEqual((row['local_query_head'], row['local_kv_head'], row['channel']), (14, 3, 12))
        with self.assertRaises(ValueError):
            G.high_precision_scalar(case, 2, 2048, 80)


if __name__ == '__main__':
    unittest.main()
