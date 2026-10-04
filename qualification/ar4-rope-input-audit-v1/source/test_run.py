"""Synthetic integer-rounding and recorded-layout tests; no real input reads."""
import copy
import unittest
import run as M


def fixture():
    tables, requests = [], []
    for position in range(4):
        row = dict(position=position)
        for suffix, word, width in (('cos', 0x3f800000, 32), ('sin', 0x00000000, 32)):
            row['f64_model_' + suffix + '_f32_bits'] = [word] * 128
            row['framework_companion_' + suffix + '_f32_bits'] = [word] * 128
            row['framework_' + suffix + '_bf16_bits'] = [word >> 16] * 128
        tables.append(row)
        requests.append(dict(protocol=1, id=position + 1, command=dict(op='forward',
            generation=position + 1, cache_metadata=[position], rotary_bits=[0x3f800000] * 64 + [0] * 64)))
    return tables, requests


class AuditTests(unittest.TestCase):
    def test_exact_finite_and_signed_zero_words(self):
        for bits, expected in ((0, 0), (0x80000000, 0x8000), (0x3f800000, 0x3f80),
                               (0xbf800000, 0xbf80), (0x00800000, 0x0080)):
            self.assertEqual(M.bf16_rne(bits), expected)

    def test_ties_to_even_positive_and_negative(self):
        for bits, expected in ((0x3f808000, 0x3f80), (0x3f818000, 0x3f82),
                               (0xbf808000, 0xbf80), (0xbf818000, 0xbf82)):
            self.assertEqual(M.bf16_rne(bits), expected)

    def test_subnormal_boundary_rounding(self):
        for bits, expected in ((0x00008000, 0), (0x00008001, 1), (0x00018000, 2),
                               (0x80008000, 0x8000), (0x80008001, 0x8001)):
            self.assertEqual(M.bf16_rne(bits), expected)

    def test_invalid_words_and_overflow_refuse(self):
        for bits in (True, -1, 1 << 32, 1.0, 0x7f800000, 0xff800000, 0x7fc00000, 0x7f7fffff):
            with self.subTest(bits=bits), self.assertRaises(ValueError):
                M.bf16_rne(bits)

    def test_all_four_split_half_tables(self):
        tables, requests = fixture()
        result = M.compare(tables, requests)
        self.assertEqual([row['position'] for row in result], [0, 1, 2, 3])
        self.assertEqual(sum(row['bf16_exact_words'] for row in result), 512)

    def test_raw_difference_can_disappear_at_bf16_boundary(self):
        tables, requests = fixture()
        requests[1]['command']['rotary_bits'][0] += 1
        row = M.compare(tables, requests)[1]
        self.assertEqual(row['bf16_exact_words'], 128)
        self.assertEqual(row['raw_f32_equal_to_framework_companion'], 127)

    def test_material_coefficient_difference_is_reported_not_accepted(self):
        tables, requests = fixture()
        requests[2]['command']['rotary_bits'][7] = 0x3f810000
        row = M.compare(tables, requests)[2]
        self.assertEqual(row['bf16_exact_words'], 127)
        self.assertEqual(row['bf16_different_indices'], [7])

    def test_wrong_extent_or_half_duplication_refused(self):
        tables, requests = fixture()
        requests[0]['command']['rotary_bits'].pop()
        with self.assertRaises(ValueError):
            M.compare(tables, requests)
        tables, requests = fixture()
        tables[0]['framework_cos_bf16_bits'][64] = 0x3f81
        with self.assertRaises(ValueError):
            M.compare(tables, requests)

    def test_missing_duplicate_or_wrong_position_refused(self):
        tables, requests = fixture()
        for changed in (tables[:-1], tables + [tables[0]]):
            with self.assertRaises(ValueError):
                M.compare(changed, requests)
        wrong = copy.deepcopy(requests)
        wrong[1]['command']['cache_metadata'][0] = 3
        with self.assertRaises(ValueError):
            M.compare(tables, wrong)

    def test_wrong_generation_and_bool_identity_refused(self):
        tables, requests = fixture()
        for key, value in (('generation', 1), ('generation', True)):
            wrong = copy.deepcopy(requests)
            wrong[1]['command'][key] = value
            with self.assertRaises(ValueError):
                M.compare(tables, wrong)
        requests[0]['id'] = True
        with self.assertRaises(ValueError):
            M.compare(tables, requests)

    def test_duplicate_and_nonfinite_json_refused(self):
        for raw in (b'{"x":1,"x":2}', b'{"x":NaN}', b'{"x":Infinity}'):
            with self.assertRaises(ValueError):
                M.parse(raw)


if __name__ == '__main__':
    unittest.main(verbosity=2)
