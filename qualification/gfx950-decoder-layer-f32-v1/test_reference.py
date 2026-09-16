import importlib.util
import math
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("reference", Path(__file__).with_name("reference.py"))
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)


class ReferenceTests(unittest.TestCase):
    def test_matrix_row_orientation(self):
        self.assertEqual(reference.matvec([[1, 2], [3, 4]], [5, 6]), [17, 39])

    def test_rope_split_half(self):
        self.assertEqual(reference.rotate([1, 2], 0, 1), [-2, 1])

    def test_epsilon_and_zero(self):
        self.assertEqual(reference.norm([0, 0], [1, 2]), [0, 0])
        expected = 1e-8 / math.sqrt(1e-16 + reference.EPSILON)
        self.assertAlmostEqual(reference.norm([1e-8, -1e-8], [1, 1])[0], expected)

    def test_softmax_and_silu_extremes(self):
        self.assertEqual(reference.softmax([1000, 1000]), [0.5, 0.5])
        self.assertEqual(reference.silu(1000), 1000)
        self.assertEqual(reference.silu(-1000), 0)

    def test_zero_layer(self):
        self.assertEqual(reference.decoder_layer([0.0] * 12, [1.0] * 110), [0.0] * 40)

    def test_gqa_shares_values_between_query_heads(self):
        record = [0.0] * 8 + [2.0, 4.0, -1.0, 2.0]
        result = reference.decoder_layer(record, [0.0] * 110)
        for got, expected in zip(result[12:16], [1 / 3, 2.0, 1 / 3, 2.0]):
            self.assertAlmostEqual(got, expected)

    def test_all_cases_finite_and_roundtrip(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ("mixed", "zero", "saturation", "skewed-attention"):
                case = Path(directory) / name
                reference.generate(case, name)
                expected = reference.decode((case / "expected.f64le").read_bytes(), 256 * 40, "d")
                actual = case / "rounded.f32le"
                actual.write_bytes(reference.encode(expected, "f"))
                self.assertTrue(reference.check(case, actual)["passed"])

    def test_bad_output_and_nonfinite_rejected(self):
        expected = [0.0] * (256 * 40)
        for value in (1.0, math.nan, math.inf):
            actual = expected.copy()
            actual[128 * 40 + 36] = value
            result = reference.compare(expected, actual)
            self.assertFalse(result["passed"])
            self.assertEqual(result["first_failures"][0]["stage"], "final_residual")

    def test_wrong_extent_and_changed_case_rejected(self):
        with self.assertRaises(ValueError):
            reference.decode(b"\0", 1, "f")
        with tempfile.TemporaryDirectory() as directory:
            case = Path(directory) / "case"
            reference.generate(case, "zero")
            with self.assertRaises(FileExistsError):
                reference.generate(case, "zero")
            (case / "weights.f32le").write_bytes(b"corrupted")
            with self.assertRaises(ValueError):
                reference.check(case, case / "missing-output")


if __name__ == "__main__":
    unittest.main()
