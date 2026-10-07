#!/usr/bin/env python3
"""CPU-only production-shape and fail-closed MFMA diagnostic regressions."""

import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("mfma_differential", ROOT / "mfma_differential.py")
MFMA = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MFMA)
DIFF = MFMA.load_differential(ROOT / "projection_differential.py")
PROBE = DIFF.load_probe(ROOT / "probe.py")
CORE = PROBE.load_helper(ROOT.parents[2] / "proofs/tensor-parallel-kernels-v1/probe.py")


class MfmaNumericsTests(unittest.TestCase):
    def test_closed_roster_covers_all_tp_reduction_widths(self):
        self.assertEqual(len(list(MFMA.fixture_specs("tp1"))), 15)
        shapes = list(MFMA.fixture_specs("shapes"))
        self.assertEqual(len(shapes), 45)
        self.assertEqual({shape[3] for shape in shapes}, {1, 2, 8})
        self.assertEqual({shape[2] for shape in shapes if shape[5]}, {512, 1536, 2048, 4096, 6144, 12288})
        self.assertEqual({shape[0] for shape in shapes}, {1, 3, 16})
        for shape in shapes:
            MFMA.checked_shape(*shape[:6])

    def test_bad_shapes_are_rejected_before_allocation(self):
        for shape in ((0, 4096, 4096, 1, 1, False), (17, 4096, 4096, 1, 1, False),
                      (True, 4096, 4096, 1, 1, False), (1, 4096, 4096, True, 1, False),
                      (1, 4096, 4096, 4, 1, False), (1, 1024, 4096, 1, 1, False),
                      (1, 4096, 4096, 1, 2, True), (1, 151936, 4096, 1, 6, False)):
            with self.subTest(shape=shape), self.assertRaises(ValueError):
                MFMA.checked_shape(*shape)

    def test_pinned_helper_rejects_replacement_and_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            replacement = Path(directory) / "replacement.py"
            replacement.write_bytes((ROOT / "projection_differential.py").read_bytes() + b"\n")
            with self.assertRaisesRegex(RuntimeError, "identity mismatch"):
                MFMA.load_differential(replacement)
            linked = Path(directory) / "linked.py"
            linked.symlink_to(ROOT / "projection_differential.py")
            with self.assertRaises(OSError):
                MFMA.load_differential(linked)

    def test_bf16_rounding_preserves_sign_and_ties_to_even(self):
        for value, expected in ((0.0, 0x0000), (-0.0, 0x8000),
                                (1.00390625, 0x3F80), (1.01171875, 0x3F82),
                                (-1.00390625, 0xBF80), (-1.01171875, 0xBF82)):
            self.assertEqual(PROBE.bf16_bits(value), expected)
        for bits in range(65536):
            if bits & 0x7F80 != 0x7F80:
                value = struct.unpack("<f", struct.pack("<I", bits << 16))[0]
                self.assertEqual(PROBE.bf16_bits(value), bits)

    def test_dense_bits_are_deterministic_finite_and_mixed(self):
        data = MFMA.deterministic_bf16(8192, "fixture")
        self.assertEqual(data, MFMA.deterministic_bf16(8192, "fixture"))
        self.assertNotEqual(data, MFMA.deterministic_bf16(8192, "other"))
        bits = MFMA.words(data)
        self.assertEqual({value >> 15 for value in bits}, {0, 1})
        self.assertEqual({value & 127 for value in bits}, set(range(128)))
        self.assertTrue(all(0 < value & 0x7F80 < 0x7F80 for value in bits))

    def test_fp64_comparison_encoding_explicitly_includes_fp32_rounding(self):
        # Above the exact BF16 midpoint, but FP32 rounds back to that midpoint.
        value = 1.00390625 + 2.0 ** -30
        self.assertGreater(value, 1.00390625)
        self.assertEqual(MFMA.f32(value), 1.00390625)
        self.assertEqual(MFMA.encode(PROBE, value, False), struct.pack("<H", 0x3F80))

    def test_full_transpose_bit_parity_includes_tails_and_special_values(self):
        for n, k in ((1, 1), (7, 19), (16, 17), (33, 31), (129, 65)):
            original = [((column * 271 + inner * 37) & 65535)
                        for column in range(n) for inner in range(k)]
            transposed = MFMA.words(MFMA.transpose_nk(MFMA.word_bytes(original), n, k))
            self.assertEqual(list(transposed), [original[column * k + inner]
                                               for inner in range(k) for column in range(n)])
        with self.assertRaisesRegex(ValueError, "byte extent"):
            MFMA.transpose_nk(b"\x00\x00", 2, 1)

    def test_real_width_reference_dots_and_cancellation(self):
        for k in (512, 1536, 2048, 4096, 6144, 12288):
            with self.subTest(k=k):
                left = [1.0] * k
                right = [0.0] * k
                right[0], right[1], right[16], right[17] = 2.0 ** 24, 1.0, -(2.0 ** 24), 1.0
                refs = MFMA.reference_dots(left, right)
                self.assertEqual(refs["serial_fp32"], 1.0)
                self.assertEqual(refs["fp64_products_sum"], 2.0)
                self.assertEqual(refs["sum_abs_products"], 2.0 ** 25 + 2.0)
                self.assertEqual(refs["chunk16_once_rounded_hypothesis"], 1.0)
                onehot = [0.0] * k
                onehot[k - 1] = -1.0
                self.assertEqual(MFMA.reference_dots(onehot, [2.0] * k)["serial_fp32"], -2.0)

    def test_real_shape_full_transpose_and_exact_layout_reference(self):
        # One full-array case for every unique TP shape; no model files or GPU.
        for spec in MFMA.fixture_specs("shapes"):
            if spec[-1] != "layout":
                continue
            with self.subTest(spec=spec):
                case = MFMA.make_fixture(CORE, PROBE, spec)
                rows, n, k = spec[:3]
                source = MFMA.words(case["buffers"][1]["data"])
                transposed = MFMA.words(MFMA.transpose_nk(case["buffers"][1]["data"], n, k))
                for column in range(n):
                    self.assertEqual(transposed[column::n], source[column * k:(column + 1) * k])
                expected = MFMA.layout_expected(case, PROBE)
                size = 4 if case["partial"] else 2
                self.assertEqual(len(expected), rows * n * size)
                for row, inner in enumerate(MFMA.layout_indices(k)):
                    for column in MFMA.sampled_columns(n):
                        right = MFMA.values(case["buffers"][1]["data"][column * k * 2:(column + 1) * k * 2])
                        activation = [0.0] * k
                        activation[inner] = 1.0
                        reference = MFMA.reference_dots(activation, right)["serial_fp32"]
                        offset = (row * n + column) * size
                        self.assertEqual(expected[offset:offset + size], MFMA.encode(PROBE, reference, case["partial"]))

    def test_actual_comparator_rejects_layout_corruption(self):
        case = MFMA.make_fixture(CORE, PROBE, (1, 128, 4096, 8, 2, False, "layout"))
        expected = MFMA.layout_expected(case, PROBE)
        result = MFMA.compare(CORE, PROBE, case, expected, expected)
        self.assertTrue(result["full_layout_exact"])
        self.assertTrue(result["diagnostic_checks_pass"])
        changed = bytearray(expected)
        changed[14:16] = struct.pack("<H", 0x3F80)  # An unsampled column must still fail.
        self.assertNotEqual(changed, expected)
        result = MFMA.compare(CORE, PROBE, case, expected, bytes(changed))
        self.assertFalse(result["full_layout_exact"])
        self.assertFalse(result["diagnostic_checks_pass"])

    def test_dense_comparator_labels_drift_without_claiming_parity(self):
        case = MFMA.make_fixture(CORE, PROBE, (1, 128, 4096, 8, 2, False, "cancellation"))
        activation = MFMA.values(case["buffers"][0]["data"])
        baseline, exact = bytearray(), bytearray()
        for column in range(128):
            right = MFMA.values(case["buffers"][1]["data"][column * 8192:(column + 1) * 8192])
            refs = MFMA.reference_dots(activation, right)
            baseline.extend(MFMA.encode(PROBE, refs["serial_fp32"], False))
            exact.extend(MFMA.encode(PROBE, refs["fp64_products_sum"], False))
        result = MFMA.compare(CORE, PROBE, case, bytes(baseline), bytes(exact))
        self.assertTrue(result["diagnostic_checks_pass"])
        self.assertGreater(result["different_output_bits"], 0)
        self.assertTrue(all(sample["mfma_matches_fp64_via_fp32"] for sample in result["samples"]))
        self.assertTrue(all(not sample["mfma_matches_serial"] for sample in result["samples"]))
        corrupted = bytearray(exact)
        corrupted[:2] = struct.pack("<H", PROBE.bf16_bits(1e20))
        self.assertFalse(MFMA.compare(CORE, PROBE, case, bytes(baseline), bytes(corrupted))["diagnostic_checks_pass"])

    def test_nonfinite_and_extent_rejections(self):
        case = MFMA.make_fixture(CORE, PROBE, (1, 128, 4096, 8, 2, False, "layout"))
        expected = MFMA.layout_expected(case, PROBE)
        with self.assertRaisesRegex(RuntimeError, "active output extent"):
            MFMA.compare(CORE, PROBE, case, expected[:-2], expected)
        with self.assertRaisesRegex(RuntimeError, "nonfinite projection output"):
            MFMA.compare(CORE, PROBE, case, expected, struct.pack("<H", 0x7FC0) * 128)


if __name__ == "__main__":
    unittest.main()
