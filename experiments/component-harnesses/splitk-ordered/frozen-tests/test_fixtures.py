"""CPU-only tests, to be executed under the remote bounded CPU profile."""

import math
import struct
import unittest

import fixtures as f


def bf16_at(data, index):
    value, = struct.unpack_from("<H", data, 2 * index)
    return struct.unpack("<f", struct.pack("<I", value << 16))[0]


class FixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = f.make_fixture()

    def test_closed_shape_and_exact_sum_bound(self):
        self.assertEqual((f.K, f.N, f.SPLITS, f.PART_K), (12288, 4096, 8, 1536))
        self.assertLess(f.K * 8 * 9, 1 << 24)
        self.assertEqual(len(self.fixture.partials_f32), 131072)
        self.assertEqual(len(self.fixture.output_f32), 16384)

    def test_inputs_and_both_weight_layouts_cover_tile_and_partition_edges(self):
        for k in (0, 1, 15, 16, 17, 18, 19, 1535, 1536, 3071, 3072, f.K - 1):
            self.assertEqual(bf16_at(self.fixture.input_bf16, k), (k % 17 - 8) / 16)
            for n in (0, 1, 15, 16, 18, 19, 255, 256, 511, 512, f.N - 1):
                expected = ((3 * k + 5 * n) % 19 - 9) / 16
                self.assertEqual(bf16_at(self.fixture.weights_nk_bf16, n * f.K + k), expected)
                self.assertEqual(bf16_at(self.fixture.weights_kn_bf16, k * f.N + n), expected)

    def test_partition_and_final_reference_from_encoded_inputs(self):
        for n in (0, 15, 16, 255, 256, f.N - 1):
            observed = []
            for part in range(f.SPLITS):
                exact = math.fsum(bf16_at(self.fixture.input_bf16, k)
                                 * bf16_at(self.fixture.weights_nk_bf16, n * f.K + k)
                                 for k in range(part * f.PART_K, (part + 1) * f.PART_K))
                actual, = struct.unpack_from("<f", self.fixture.partials_f32,
                                             (part * f.N + n) * 4)
                self.assertEqual(actual, exact)
                observed.append(actual)
            final, = struct.unpack_from("<f", self.fixture.output_f32, n * 4)
            self.assertEqual(final, math.fsum(observed))

    def test_all_eight_partitions_have_distinct_signatures(self):
        signatures = [self.fixture.partials_f32[part * f.N * 4:(part + 1) * f.N * 4]
                      for part in range(f.SPLITS)]
        self.assertEqual(len(set(signatures)), f.SPLITS)

    def test_exact_output_rejects_nan_signed_zero_and_extent_drift(self):
        expected = struct.pack("<ff", 0.0, 1.0)
        f.validate_output(expected, expected, "valid")
        for invalid in (expected[:-1], expected + b"\0", bytearray(expected),
                        struct.pack("<ff", -0.0, 1.0), struct.pack("<ff", float("nan"), 1.0),
                        struct.pack("<ff", 0.0, 2.0)):
            with self.assertRaises(ValueError):
                f.validate_output(invalid, expected, "invalid")

    def test_control_is_current_wave_and_separate_unsplit_mfma(self):
        self.assertEqual(set(f.CONTROL_KERNELS), {"wave", "mfma"})
        self.assertEqual(f.CONTROL_KERNELS["wave"]["groups"], 4096)
        self.assertEqual(f.CONTROL_KERNELS["mfma"]["groups"], 256)
        self.assertEqual(f.CONTROL_KERNELS["wave"]["weights"], "weights_nk_bf16")
        self.assertEqual(f.CONTROL_KERNELS["mfma"]["weights"], "weights_kn_bf16")
        for record in f.CONTROL_KERNELS.values():
            self.assertEqual(record["scalars"], (1, 4096, 12288, 1, 2))

    def test_out_of_contract_coordinates_and_values_reject(self):
        for k in (-1, f.K, True, 1.5):
            with self.assertRaises(ValueError):
                f.input_numerator(k)
        for k, n in ((0, -1), (0, f.N), (f.K, 0), (True, 0), (0, False)):
            with self.assertRaises(ValueError):
                f.weight_numerator(k, n)
        for value in (-10, 10, True, 0.5):
            with self.assertRaises(ValueError):
                f.exact_bf16(value)


if __name__ == "__main__":
    unittest.main()
