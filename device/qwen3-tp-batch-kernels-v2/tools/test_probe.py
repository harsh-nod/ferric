import importlib.util
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


SOURCE = Path(__file__).with_name("probe.py")
SPEC = importlib.util.spec_from_file_location("batch_probe", SOURCE)
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)
HELPER = SOURCE.resolve().parents[3] / "proofs/tensor-parallel-kernels-v1/probe.py"


class BatchProbeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.core = PROBE.load_helper(HELPER)
        cls.fixtures = {case["name"]: case for case in PROBE.self_test(cls.core)}

    def test_roster_and_source_layouts(self):
        self.assertEqual(len(self.fixtures), 11)
        layouts = {
            "gemm_bf16_f32_bf16_v2": (3, 5), "gemm_partial_bf16_f32_v2": (3, 5),
            "swiglu_bf16_f32_v2": (3, 2), "rope_v2": (7, 2),
            "paged_kv_append_v2": (6, 4), "paged_gqa_bf16_f32_v2": (6, 5),
            "argmax_bf16_v2": (2, 1),
        }
        for case in self.fixtures.values():
            self.assertEqual((len(case["buffers"]), len(case["scalars"])),
                             layouts[case["symbol"].removeprefix(PROBE.PREFIX)])
            self.assertGreater(case["groups"], 0)

    def test_projection_rows_and_inactive_tail(self):
        for rows in (1, 3, 16):
            case = self.fixtures[f"column_rows_{rows}"]
            output = case["buffers"][2]
            for row in range(rows):
                self.assertEqual(output["expected"][row * 1024:(row + 1) * 1024],
                                 PROBE.pack_bf16([float(row + 1)] * 512))
            self.assertEqual(output["expected"][rows * 1024:], output["data"][rows * 1024:])

    def test_partials_keep_precision_and_per_row_values(self):
        for rows in (1, 3, 16):
            output = self.fixtures[f"partial_rows_{rows}"]["buffers"][2]
            for row in range(rows):
                value = struct.unpack_from("<f", output["expected"], row * 4096 * 4)[0]
                self.assertEqual(value, (row + 1) * 1.00390625)
            self.assertEqual(output["expected"][rows * 16384:], output["data"][rows * 16384:])

    def test_append_changes_only_three_distinct_selected_slots(self):
        case = self.fixtures["append_page_boundary_preserves_pool"]
        for record in case["buffers"][4:]:
            changed = {slot for slot in range(64)
                       if record["data"][slot * 256:(slot + 1) * 256]
                       != record["expected"][slot * 256:(slot + 1) * 256]}
            self.assertEqual(changed, {0, 16, 47})
        self.assertEqual(case["scalars"], [3, 8, 2, 4])

    def test_attention_nonzero_scores_and_masked_future_storage(self):
        case = self.fixtures["attention_nonzero_qk_mixed_positions"]
        query, keys, values, positions, tables, output = case["buffers"]
        self.assertEqual(positions["data"], PROBE.pack_u32([1, 16]))
        self.assertEqual(tables["data"], PROBE.pack_u32([2, 0xFFFFFFFF, 1, 3]))
        self.assertNotEqual(query["data"][:2], b"\x00\x00")
        self.assertEqual(keys["data"][-2:], b"\xc0\x7f")
        self.assertEqual(values["data"][-2:], b"\xc0\x7f")
        self.assertNotEqual(output["expected"][:2], PROBE.pack_bf16([2.0]))
        self.assertNotEqual(output["expected"][1024:1026], PROBE.pack_bf16([0.5]))
        self.assertEqual(output["data"][2048:], output["expected"][2048:])

    def test_argmax_tie_is_lowest_index_per_row(self):
        case = self.fixtures["argmax_rows_distinct_and_lowest_tie"]
        self.assertEqual(case["buffers"][1]["expected"],
                         PROBE.pack_u32([0, 151935, 17] + [0xFFFFFFFF] * 13))
        logits = case["buffers"][0]["data"]
        self.assertEqual(logits[(2 * 151936 + 17) * 2:(2 * 151936 + 19) * 2], b"\x00\x40" * 2)

    def test_unverified_helper_side_effect_is_never_executed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sentinel = root / "side-effect"
            hostile = root / "helper.py"
            hostile.write_bytes(HELPER.read_bytes() +
                                f"\nopen({str(sentinel)!r}, 'w').write('bad')\n".encode())
            with self.assertRaisesRegex(RuntimeError, "identity mismatch"):
                PROBE.load_helper(hostile)
            self.assertFalse(sentinel.exists())

    def test_symlink_helper_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            link = Path(temporary) / "helper.py"
            link.symlink_to(HELPER)
            with self.assertRaises(OSError):
                PROBE.load_helper(link)

    def test_growing_helper_is_rejected_during_bounded_read(self):
        with mock.patch.object(PROBE.os, "read", side_effect=[b"x" * (128 * 1024), b"x"]) as reader:
            with self.assertRaisesRegex(RuntimeError, "grew beyond byte limit"):
                PROBE.load_helper(HELPER)
            self.assertEqual([call.args[1] for call in reader.call_args_list], [128 * 1024 + 1, 1])

    def test_shallow_deployment_help_does_not_resolve_default_helper(self):
        with tempfile.TemporaryDirectory(prefix="ferric-batch-probe-host-", dir="/tmp") as temporary:
            source = Path(temporary) / "probe.py"
            source.write_bytes(SOURCE.read_bytes())
            result = subprocess.run([sys.executable, str(source), "--help"],
                                    capture_output=True, text=True, timeout=30, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("--helper", result.stdout)

    def test_shallow_deployment_explicit_helper_self_test(self):
        with tempfile.TemporaryDirectory(prefix="ferric-batch-probe-host-", dir="/tmp") as temporary:
            source = Path(temporary) / "probe.py"
            source.write_bytes(SOURCE.read_bytes())
            result = subprocess.run([sys.executable, str(source), "--helper", str(HELPER), "--self-test"],
                                    capture_output=True, text=True, timeout=30, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("no GPU worker launched", result.stdout)


if __name__ == "__main__":
    unittest.main()
