"""Small standard-library tests; installed Torch is not imported by this suite."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('rope_reference_under_test', Path(__file__).with_name('run.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)


class ArithmeticTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.oracle = R.load_oracle()

    def test_multiply_exact_normal_words(self):
        self.assertEqual(R.multiply(0x3f810000, 0x3fc00000), 0x3fc18000)
        self.assertEqual(R.multiply(0xbf800000, 0x40000000), 0xc0000000)

    def test_multiply_signed_zero(self):
        self.assertEqual(R.multiply(0x80000000, 0x3f800000), 0x80000000)
        self.assertEqual(R.multiply(0x80000000, 0xbf800000), 0)

    def test_fp32_subnormal_ties(self):
        self.assertEqual(R.multiply(1, 0x3f000000), 0)
        self.assertEqual(R.multiply(3, 0x3f000000), 2)
        self.assertEqual(R.multiply(0x80000001, 0x3f000000), 0x80000000)

    def test_bf16_product_even_and_odd_ties(self):
        self.assertEqual(self.oracle.narrow_bf16_rne(R.multiply(0x3f810000, 0x3fc00000)), 0x3fc2)
        self.assertEqual(self.oracle.narrow_bf16_rne(R.multiply(0x3f830000, 0x3fc00000)), 0x3fc4)

    def test_materialization_changes_cancelled_result(self):
        values = [0x3f81] * 64 + [0x3f80] * 64
        cosine, sine = [0x3fc00000] * 128, [0x3f800000] * 128
        rounded = R.rotate(values, cosine, sine, self.oracle, True)
        fused_boundary = R.rotate(values, cosine, sine, self.oracle, False)
        self.assertEqual(rounded[:64], [0x3f04] * 64)
        self.assertEqual(fused_boundary[:64], [0x3f03] * 64)
        self.assertGreater(R.compare(rounded, fused_boundary)['different_words'], 0)

    def test_negation_before_product_preserves_negative_zero(self):
        values = [0] * 128
        result = R.rotate(values, [0x80000000] * 128, [0] * 128, self.oracle, True)
        self.assertEqual(result[:64], [0x8000] * 64)
        self.assertEqual(result[64:], [0] * 64)

    def test_zero_table_identity_for_nonzero_values(self):
        values = [0x3f81] * 64 + [0xbf81] * 64
        cosine, sine = R.f64_table_model(0)
        self.assertEqual(cosine, [0x3f800000] * 128)
        self.assertEqual(sine, [0] * 128)
        self.assertEqual(R.rotate(values, cosine, sine, self.oracle, True), values)

    def test_nonzero_positions_and_duplicated_halves(self):
        zero = R.f64_table_model(0)
        for position in R.POSITIONS[1:]:
            cosine, sine = R.f64_table_model(position)
            self.assertNotEqual((cosine, sine), zero)
            self.assertEqual(cosine[:64], cosine[64:])
            self.assertEqual(sine[:64], sine[64:])
        with self.assertRaises(ValueError):
            R.f64_table_model(True)

    def test_nonfinite_and_overflow_refused(self):
        with self.assertRaises(ValueError):
            R.multiply(0x7f800000, 0)
        with self.assertRaises(OverflowError):
            R.multiply(0x7f7fffff, 0x40000000)
        with self.assertRaises(self.oracle.FinitePolicyError):
            self.oracle.narrow_bf16_rne(0x7f7fffff)

    def test_bad_shape_and_mixed_half_tables_refused(self):
        with self.assertRaises(ValueError):
            R.rotate([0] * 127, [0] * 128, [0] * 128, self.oracle, True)
        cosine = [0] * 128
        cosine[64] = 0x3f800000
        with self.assertRaises(ValueError):
            R.rotate([0] * 128, cosine, [0] * 128, self.oracle, True)

    def test_exact_word_pack_roundtrip(self):
        values = [0, 0x8000, 1, 0x8001, 0x3f81]
        self.assertEqual(R.words(R.pack_words(values, 16), 16), values)
        with self.assertRaises(ValueError):
            R.words(b'\x00', 16)
        with self.assertRaises(ValueError):
            R.words(b'\x80\x7f', 16)

    def test_comparison_exposes_signed_zero_bits(self):
        row = R.compare([0, 0x8000], [0, 0])
        self.assertEqual(row['exact_words'], 1)
        self.assertEqual(row['first_differences'], [{'index': 1, 'actual_bits': 0x8000, 'expected_bits': 0}])
        with self.assertRaises(ValueError):
            R.compare([0], [])

    def test_synthetic_corpus_is_finite_and_bounded(self):
        q, k = R.synthetic()
        self.assertEqual((len(q), len(k)), (256, 128))
        for data in (q, k):
            R.words(R.pack_words(data, 16), 16)
            self.assertIn(0x8000, data)
            self.assertIn(1, data)

    def test_file_pin_and_symlink_refusal(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary).resolve() / 'source'
            path.write_bytes(b'actual body')
            digest = hashlib.sha256(b'actual body').hexdigest()
            self.assertEqual(R.read(path, digest, 11), b'actual body')
            with self.assertRaises(ValueError):
                R.read(path, '0' * 64)
            with self.assertRaises(ValueError):
                R.read(path, digest, 12)
            link = path.with_name('link')
            link.symlink_to(path)
            with self.assertRaises(ValueError):
                R.read(link, digest)


if __name__ == '__main__':
    unittest.main(verbosity=2)
