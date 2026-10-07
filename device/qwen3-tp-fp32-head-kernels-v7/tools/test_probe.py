import math
import struct
import types
import unittest

import probe


def record(name, data, element_bytes, expected=None):
    return {"name": name, "data": bytes(data), "element_bytes": element_bytes,
            "expected": bytes(data if expected is None else expected),
            "access": "read" if expected is None else "write"}


class Fixtures(unittest.TestCase):
    def test_shape_boundaries_reject_before_allocating(self):
        core = types.SimpleNamespace(buffer=record)
        for rows in (0, 17, -1, True, 1.0):
            with self.assertRaises(RuntimeError):
                probe.head_case(core, rows, False)
            with self.assertRaises(RuntimeError):
                probe.argmax_case(core, rows)
        for layout, n in (("other", 128), ("nk", 0), ("kn", probe.N + 1), ("nk", True)):
            with self.assertRaises(RuntimeError):
                probe.weights(layout, n)

    def test_independent_layouts_match_all_elements_at_real_reduction_width(self):
        n = 128
        nk, kn = probe.weights("nk", n), probe.weights("kn", n)
        self.assertEqual(len(nk), n * 4096 * 2)
        for column in range(n):
            for inner in range(probe.K):
                self.assertEqual(nk[(column * probe.K + inner) * 2:(column * probe.K + inner + 1) * 2],
                                 kn[(inner * n + column) * 2:(inner * n + column + 1) * 2])

    def test_fp32_winner_does_not_collapse_to_bf16_tie(self):
        def narrowed(value):
            bits = struct.unpack("<I", struct.pack("<f", value))[0]
            return (bits + 0x7fff + ((bits >> 16) & 1)) >> 16
        for row in (0, 15):
            left, right = probe.reference(row, 7), probe.reference(row, 101)
            self.assertLess(left, right)
            self.assertEqual(narrowed(left), narrowed(right))
        self.assertEqual(narrowed(24.375), narrowed(24.40625))

    def test_argmax_active_rows_preserve_tail_and_lowest_id_rules(self):
        core = types.SimpleNamespace(buffer=record)
        for rows in (1, 3, 16):
            case = probe.argmax_case(core, rows)
            logits, choices = case["buffers"]
            self.assertEqual(case["groups"], rows)
            self.assertEqual(choices["expected"][rows * 4:], b"\xa5" * ((16 - rows) * 4))
            for row in range(rows):
                values = struct.unpack_from(f"<{probe.N}f", logits["data"], row * probe.N * 4)
                self.assertTrue(all(math.isfinite(value) for value in values))
                actual = max(range(probe.N), key=values.__getitem__)
                expected = struct.unpack_from("<I", choices["expected"], row * 4)[0]
                self.assertEqual(actual, expected)
            if rows < 16:
                self.assertTrue(math.isnan(struct.unpack_from("<f", logits["data"], rows * probe.N * 4)[0]))


if __name__ == "__main__":
    unittest.main()
