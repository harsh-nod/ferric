"""CPU-only arithmetic/protocol fixtures. Never constructs a worker."""
import math
from pathlib import Path
import struct
import tempfile
import unittest
from unittest import mock

import probe


class Core:
    @staticmethod
    def bf16(bits, count):
        return struct.pack("<H", bits) * count

    @staticmethod
    def buffer(name, data, element_bytes, expected=None):
        return {"name": name, "data": data, "element_bytes": element_bytes,
                "expected": data if expected is None else expected}


class ProbeTests(unittest.TestCase):
    def test_closed36_specs_are_unique_and_draft_only(self):
        specs = probe.case_specs()
        self.assertEqual(len(specs), 36)
        self.assertEqual(len(set(specs)), 36)
        self.assertEqual(len(probe.ROOTS), 14)
        self.assertTrue(all(name.startswith(probe.PREFIX) and name.endswith("_v10") for name in probe.ROOTS))
        self.assertEqual(set(probe.SHAPES), {"query", "key", "value", "gate", "up", "attention_output", "down", "head"})

    def test_scalar_and_mfma_real_shapes_preserve_transpose_outputs_and_tails(self):
        for role in ("query", "key", "value", "gate", "up", "attention_output", "down"):
            n, k, tag, fp32 = probe.SHAPES[role]
            for rows in (1, 5, 17, 32):
                scalar = probe.projection_case(Core, rows, role, False)
                mfma = probe.projection_case(Core, rows, role, True)
                self.assertEqual(scalar["scalars"], [rows, n, k, 1, tag])
                self.assertEqual(scalar["groups"], ((rows + 15) // 16) * (n // 16))
                self.assertEqual(scalar["buffers"][0], mfma["buffers"][0])
                self.assertEqual(scalar["buffers"][2], mfma["buffers"][2])
                nk, kn = scalar["buffers"][1]["data"], mfma["buffers"][1]["data"]
                for column in (0, 7, 101, n - 1):
                    for inner in range(k):
                        self.assertEqual(nk[(column * k + inner) * 2:(column * k + inner + 1) * 2],
                                         kn[(inner * n + column) * 2:(inner * n + column + 1) * 2])
                output = scalar["buffers"][2]
                width = 4 if fp32 else 2
                self.assertEqual(output["expected"][rows * n * width:], b"\xa5" * ((32 - rows) * n * width))
                for row in (0, rows - 1):
                    for column in (0, 7, 101, n - 1):
                        expected = probe.reference(row, column, k)
                        encoded = struct.pack("<f", expected) if fp32 else struct.pack("<H", probe.rounded_bf16(expected))
                        offset = (row * n + column) * width
                        self.assertEqual(output["expected"][offset:offset + width], encoded)

    def test_full_argmax_active_rows_and_nan_tail_are_strict(self):
        for rows in (1, 17, 32):
            case = probe.argmax_case(Core, rows)
            logits, output = case["buffers"]
            for row in range(rows):
                values = struct.unpack_from(f"<{probe.N}f", logits["data"], row * probe.N * 4)
                self.assertTrue(all(math.isfinite(v) for v in values))
                winner = max(range(probe.N), key=values.__getitem__)
                self.assertEqual(winner, struct.unpack_from("<I", output["expected"], row * 4)[0])
            self.assertEqual(output["expected"][rows * 4:], b"\xa5" * ((32 - rows) * 4))
            if rows < 32:
                self.assertTrue(math.isnan(struct.unpack_from("<f", logits["data"], rows * probe.N * 4)[0]))

    def test_rope_distinct_heads_obey_exact_quarter_turn(self):
        case = probe.rope_case(Core)
        for source, output, heads in [(case["buffers"][0], case["buffers"][5], 16),
                                       (case["buffers"][1], case["buffers"][6], 8)]:
            for row in range(32):
                for head in range(heads):
                    for lane in range(64):
                        base = (row * heads + head) * 128
                        first = struct.unpack_from("<H", source["data"], (base + lane) * 2)[0]
                        second = struct.unpack_from("<H", source["data"], (base + lane + 64) * 2)[0]
                        self.assertEqual(struct.unpack_from("<H", output["expected"], (base + lane) * 2)[0], second ^ 0x8000)
                        self.assertEqual(struct.unpack_from("<H", output["expected"], (base + lane + 64) * 2)[0], first)

    def test_append_last_physical_page_and_causal_attention_mapping(self):
        append = probe.append_case(Core)
        self.assertEqual(append["scalars"], [32, 1, 512, 512])
        for row in range(32):
            slot = (511 if row < 16 else 0) * 16 + row % 16
            for input_index, cache_index in [(0, 4), (1, 5)]:
                source = append["buffers"][input_index]["data"]
                output = append["buffers"][cache_index]["expected"]
                self.assertEqual(source[row * 2048:(row + 1) * 2048], output[slot * 2048:(slot + 1) * 2048])
        for last in (False, True):
            attention = probe.attention_case(Core, last)
            rows = attention["scalars"][0]
            positions = struct.unpack(f"<{rows}I", attention["buffers"][3]["data"])
            table = attention["buffers"][4]["data"]
            for row, position in enumerate(positions):
                for logical in range(position // 16 + 1):
                    page = struct.unpack_from("<I", table, (row * 512 + logical) * 4)[0]
                    self.assertLess(page, 512)
                for query_head in range(16):
                    expected = probe.bf16((query_head // 2 + 1) / 2.0)
                    offset = (row * 2048 + query_head * 128) * 2
                    self.assertEqual(attention["buffers"][5]["expected"][offset:offset + 256], struct.pack("<H", expected) * 128)
            if not last:
                self.assertEqual(struct.unpack_from("<I", table, 4)[0], 0xffffffff)

    def test_host_only_invalid_fixture_inputs_and_helper_pin(self):
        for rows in (0, 33, True):
            with self.assertRaises(RuntimeError):
                probe.projection_case(Core, rows, "head", False)
            with self.assertRaises(RuntimeError):
                probe.argmax_case(Core, rows)
        for role, mode in [("target", False), ("query", 1)]:
            with self.assertRaises(RuntimeError):
                probe.projection_case(Core, 1, role, mode)
        for value in (float("inf"), float("nan"), 1.0001):
            with self.assertRaises(RuntimeError):
                probe.bf16(value)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "helper.py"
            path.write_bytes(b"raise AssertionError('must not run')\n")
            with self.assertRaises(RuntimeError):
                probe.load_helper(path)
            link = Path(directory) / "link.py"
            link.symlink_to(path)
            with self.assertRaises(OSError):
                probe.load_helper(link)

    def test_head_sparse_fixture_retains_declared_full_shape_without_cpu_large_allocation(self):
        with mock.patch.object(probe, "weights", side_effect=RuntimeError("stopped before allocation")) as weights:
            with self.assertRaisesRegex(RuntimeError, "stopped before allocation"):
                probe.projection_case(Core, 1, "head", True)
            weights.assert_called_once_with("kn", 151936, 1024)


if __name__ == "__main__":
    unittest.main()
