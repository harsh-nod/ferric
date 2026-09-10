#!/usr/bin/env python3
"""Host-only regression checks for the stronger projection diagnostic."""

import importlib.util
from pathlib import Path
import struct
import unittest


ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("projection_differential", ROOT / "projection_differential.py")
DIFF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DIFF)
PROBE = DIFF.load_probe(ROOT / "probe.py")
CORE = PROBE.load_helper(ROOT.parents[2] / "proofs/tensor-parallel-kernels-v1/probe.py")


class DifferentialTests(unittest.TestCase):
    def test_shape_roster_covers_decode_batch_and_rank_local_widths(self):
        specs = list(DIFF.fixture_specs())
        self.assertEqual(len(specs), 14)
        self.assertEqual({spec[0] for spec in specs}, {1, 3, 16})
        self.assertEqual({spec[2] for spec in specs if spec[4]}, {512, 1536})
        self.assertEqual({spec[1] for spec in specs if not spec[4]}, {128, 512, 1536})

    def test_mixed_mantissas_and_signs_are_deterministic(self):
        data = DIFF.finite_bits(4096, 91, 117, 15)
        self.assertEqual(data, DIFF.finite_bits(4096, 91, 117, 15))
        values = [bits for (bits,) in struct.iter_unpack("<H", data)]
        self.assertEqual({bits & 127 for bits in values}, set(range(128)))
        self.assertEqual({bits >> 15 for bits in values}, {0, 1})
        self.assertEqual(len({(bits >> 7) & 255 for bits in values}), 15)

    def test_cancellation_distinguishes_scalar_and_cooperative_orders(self):
        right = [0.0] * 128
        right[0], right[1], right[64], right[65] = 2.0 ** 24, 1.0, -(2.0 ** 24), 1.0
        serial, wave, precise, sum_abs = DIFF.reference_dots([1.0] * 128, right)
        self.assertEqual((serial, wave, precise), (1.0, 2.0, 2.0))
        self.assertEqual(sum_abs, 2.0 ** 25 + 2.0)

    def test_actual_comparator_accepts_own_orders_and_rejects_corruption(self):
        # Small host-only geometry keeps the numerical comparator test bounded.
        case = DIFF.make_fixture(CORE, (1, 32, 128, 1, True, "wide"))
        left = DIFF.unpack_bf16(case["buffers"][0]["data"])
        baseline, wave = bytearray(), bytearray()
        for column in range(32):
            right = DIFF.unpack_bf16(case["buffers"][1]["data"][column * 256:(column + 1) * 256])
            serial, cooperative, _, _ = DIFF.reference_dots(left, right)
            baseline.extend(struct.pack("<f", serial))
            wave.extend(struct.pack("<f", cooperative))
        result = DIFF.compare(CORE, case, PROBE, bytes(baseline), bytes(wave))
        self.assertTrue(result["sampled_own_order_pass"])
        self.assertEqual(result["elements"], 32)
        wave[:4] = struct.pack("<f", 1000000.0)
        result = DIFF.compare(CORE, case, PROBE, bytes(baseline), bytes(wave))
        self.assertFalse(result["sampled_own_order_pass"])
        self.assertGreater(result["different_output_bits"], 0)

    def test_nonfinite_output_is_rejected(self):
        case = DIFF.make_fixture(CORE, (1, 32, 128, 1, True, "mixed"))
        with self.assertRaisesRegex(RuntimeError, "nonfinite output"):
            DIFF.compare(CORE, case, PROBE, struct.pack("<f", float("nan")) * 32,
                         struct.pack("<f", 0.0) * 32)


if __name__ == "__main__":
    unittest.main()
