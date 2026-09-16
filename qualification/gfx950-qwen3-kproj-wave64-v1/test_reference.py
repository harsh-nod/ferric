from fractions import Fraction
import math
import unittest

import reference


class WavePolicyTests(unittest.TestCase):
    def test_gamma22_bounds_six_add_levels_after_sixteen_fmas(self):
        unit = Fraction(1, 2**24)
        gamma16 = 16 * unit / (1 - 16 * unit)
        gamma6 = 6 * unit / (1 - 6 * unit)
        gamma22 = 22 * unit / (1 - 22 * unit)
        self.assertLessEqual((1 + gamma16) * (1 + gamma6) - 1, gamma22)
        self.assertGreaterEqual(Fraction(reference.GAMMA22), gamma22)
        self.assertLess(reference.GAMMA22, reference.serial.GAMMA32)
        self.assertEqual(reference.DEPTH, 22)

    def test_invalid_results_still_fail_exact_and_bound_contracts(self):
        expected = [0.0] * 1024
        bounds = [0.0] * 1024
        output = expected.copy()
        output[0] = math.ulp(0.0)
        with self.assertRaisesRegex(ValueError, "exact"):
            reference.serial.compare(output, expected, bounds, [0])
        with self.assertRaisesRegex(ValueError, "bound"):
            reference.serial.compare(output, expected, bounds, [])


if __name__ == "__main__":
    unittest.main()
