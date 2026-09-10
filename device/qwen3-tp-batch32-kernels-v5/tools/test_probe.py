#!/usr/bin/env python3
"""Host-only checks of the 32-row native fixture data and launch boundaries."""

import importlib.util
from pathlib import Path
import struct
import unittest


ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("batch32_probe", ROOT / "probe.py")
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)
CORE = PROBE.load_helper(ROOT.parents[2] / "proofs/tensor-parallel-kernels-v1/probe.py")


class Batch32FixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixtures = {case["name"]: case for case in PROBE.self_test(CORE)}

    def test_exact_roster_has_true_wide_rows(self):
        self.assertEqual(len(self.fixtures), 30)
        self.assertEqual({case["scalars"][0] for case in self.fixtures.values()}, {17, 31, 32})
        self.assertEqual(len({case["symbol"] for case in self.fixtures.values()}), 14)
        wave = [case for case in self.fixtures.values() if "_mfma_" not in case["symbol"]]
        self.assertEqual((len(wave), len({case["symbol"] for case in wave})), (24, 12))

    def test_second_projection_tile_uses_distinct_rows_and_no_input_padding(self):
        for rows in (17, 31, 32):
            for mode in ("baseline", "wave", "mfma"):
                case = self.fixtures[f"{mode}_column_rows_{rows}"]
                self.assertEqual(len(case["buffers"][0]["data"]), rows * 4096 * 2)
                self.assertEqual(case["groups"], rows * 512 if mode == "wave" else 64)
                output = case["buffers"][2]["expected"]
                self.assertEqual(struct.unpack_from("<H", output, 16 * 512 * 2)[0], PROBE.bf16_bits(17.0))
                self.assertEqual(output[rows * 512 * 2:], b"\xA5" * ((32 - rows) * 512 * 2))
                partial = self.fixtures[f"{mode}_partial_rows_{rows}"]
                self.assertEqual(partial["groups"], rows * 4096 if mode == "wave" else 512)
                self.assertEqual(struct.unpack_from("<f", partial["buffers"][2]["expected"],
                                                   (rows - 1) * 4096 * 4)[0], rows * 1.00390625)

    def test_append_uses_32_unique_slots_and_preserves_unused_pages(self):
        case = self.fixtures["append_rows_32_distinct_slots_cross_page"]
        self.assertEqual(case["groups"], 1)
        positions = [value for (value,) in struct.iter_unpack("<I", case["buffers"][2]["data"])]
        table = [value for (value,) in struct.iter_unpack("<I", case["buffers"][3]["data"])]
        slots = [table[row * 2 + position // 16] * 16 + position % 16
                 for row, position in enumerate(positions)]
        self.assertEqual(len(set(slots)), 32)
        for record in case["buffers"][4:]:
            for page in (0, 2):
                start, end = page * 16 * 256, (page + 1) * 16 * 256
                self.assertEqual(record["data"][start:end], record["expected"][start:end])

    def test_attention_checks_row31_and_page_boundaries(self):
        case = self.fixtures["wave_attention_rows_32_causal_pages"]
        positions = [value for (value,) in struct.iter_unpack("<I", case["buffers"][3]["data"])]
        self.assertIn(16, positions)
        self.assertEqual(case["groups"], 128)
        first = case["buffers"][5]["expected"][:1024]
        last = case["buffers"][5]["expected"][-1024:]
        self.assertNotEqual(first, last)

    def test_rmsnorm_keeps_zero_extent_inactive_slices(self):
        case = self.fixtures["rmsnorm_rows_32"]
        self.assertEqual(case["buffers"][1]["data"], b"")
        self.assertEqual(case["buffers"][3]["data"], b"")
        self.assertEqual(case["buffers"][4]["expected"], case["buffers"][0]["data"])


if __name__ == "__main__":
    unittest.main()
