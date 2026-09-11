import importlib.util
from pathlib import Path
import struct
import unittest

SPEC = importlib.util.spec_from_file_location("large_kv_probe", Path(__file__).with_name("probe.py"))
probe = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probe)


class Buffers:
    @staticmethod
    def buffer(name, data, element_bytes, expected=None):
        return {"name": name, "data": bytes(data), "element_bytes": element_bytes,
                "expected": bytes(data if expected is None else expected),
                "access": "read" if expected is None else "write"}


class LargeKvFixtures(unittest.TestCase):
    def test_closed_nine_case_roster_and_independent_capacity_bounds(self):
        self.assertEqual(len(probe.SPECS), 9)
        self.assertEqual({spec[1] for spec in probe.SPECS}, {1, 16, 17, 32})
        self.assertEqual({spec[2] for spec in probe.SPECS}, {513, 8192, 16384})
        long_cases = [spec for spec in probe.SPECS if spec[0] == "attention" and spec[3] == 8192]
        self.assertEqual(long_cases, [("attention", 1, 16384, 8192)])
        for spec in probe.SPECS:
            probe.validate_spec(*spec)
        for spec in [("wave", 1, 513, 1), ("append", 0, 513, 1),
                     ("append", 33, 513, 1), ("append", True, 513, 1),
                     ("append", 1, 0, 1), ("append", 1, 16385, 1),
                     ("append", 1, True, 1), ("append", 1, 513, 0),
                     ("attention", 1, 513, 8193), ("attention", 32, 1, 17)]:
            with self.subTest(spec=spec), self.assertRaises(RuntimeError):
                probe.validate_spec(*spec)

    def test_append_full_arrays_preserve_every_unselected_word(self):
        rows, pages = 32, 513
        case = probe.append_case(Buffers, rows, pages)
        key, value, positions, table, keys, values = case["buffers"]
        self.assertEqual(case["symbol"], probe.APPEND)
        self.assertEqual(case["groups"], 1)
        self.assertEqual(case["scalars"], [32, 1, 512, 513])
        selected = []
        for row in range(rows):
            position = struct.unpack_from("<I", positions["data"], row * 4)[0]
            page = struct.unpack_from("<I", table["data"], (row * 512 + position // 16) * 4)[0]
            selected.append(page * 16 + position % 16)
        self.assertEqual(selected[-1], pages * 16 - 1)
        self.assertEqual(len(set(selected)), rows)
        for source, destination in [(key, keys), (value, values)]:
            independent = bytearray(destination["data"])
            for row, slot in enumerate(selected):
                independent[slot * 2048:(slot + 1) * 2048] = source["data"][row * 2048:(row + 1) * 2048]
            self.assertEqual(independent, destination["expected"])

    def test_attention_full_outputs_dense_reference_and_inactive_tail(self):
        rows = 17
        case = probe.attention_case(Buffers, rows, 513, 17)
        query, keys, values, positions, table, output = case["buffers"]
        self.assertEqual(case["symbol"], probe.ATTENTION)
        self.assertEqual(case["groups"], rows * 32)
        self.assertEqual(case["scalars"], [rows, 1, 2, 513, 17])
        for row in range(rows):
            position = struct.unpack_from("<I", positions["data"], row * 4)[0]
            for head in range(32):
                for dimension in range(128):
                    total = 0.0
                    for token in range(position + 1):
                        page = struct.unpack_from("<I", table["data"], (row * 2 + token // 16) * 4)[0]
                        offset = ((page * 16 + token % 16) * 1024 + head // 4 * 128 + dimension) * 2
                        self.assertEqual(keys["data"][offset:offset + 2], b"\0\0")
                        bits = struct.unpack_from("<H", values["data"], offset)[0]
                        total += struct.unpack("<f", struct.pack("<I", bits << 16))[0]
                    expected = probe.bf16(total / (position + 1))
                    actual = struct.unpack_from("<H", output["expected"], (row * 4096 + head * 128 + dimension) * 2)[0]
                    self.assertEqual(actual, expected)
        active = rows * 4096 * 2
        self.assertEqual(output["expected"][active:], output["data"][active:])
        self.assertEqual(query["data"][active:active + 2], b"\xc0\x7f")
        self.assertEqual(struct.unpack_from("<I", positions["data"], rows * 4)[0], 0xffffffff)

    def test_nonfinite_reference_rejected(self):
        for value in (float("nan"), float("inf"), -float("inf")):
            with self.assertRaises(RuntimeError):
                probe.bf16(value)


if __name__ == "__main__":
    unittest.main()
