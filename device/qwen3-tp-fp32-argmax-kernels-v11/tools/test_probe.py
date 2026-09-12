import contextlib
import importlib.util
import io
from pathlib import Path
import struct
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location("argmax_v11_probe", Path(__file__).with_name("probe.py"))
probe = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probe)


class Core:
    @staticmethod
    def buffer(name, data, element_bytes, expected=None):
        data = bytes(data)
        return {"name": name, "data": data, "element_bytes": element_bytes,
                "expected": data if expected is None else expected,
                "access": "read" if expected is None else "write"}


class ProbeTests(unittest.TestCase):
    def test_closed_roster_shapes_and_one_symbol(self):
        specs = list(probe.specifications())
        self.assertEqual(len(specs), 14)
        self.assertEqual(len(set(specs)), 14)
        self.assertEqual({spec[1] for spec in specs}, {1, 2, 16, 17, 31, 32})
        self.assertIn(("random", 1, 1, 0, 0), specs)
        self.assertEqual(probe.ROOT, "ferric_qwen3_tp_batch32_wave_argmax_f32_v11")

    def test_mixed_full_vocabulary_extremes_ties_subnormals_and_tails(self):
        case = probe.make_case(Core, ("mixed", 17, 32, 0, 0))
        probe.validate_case(case)
        logits, choices = case["buffers"]
        expected = [101, 63, 63, 0, probe.N - 1, 129, 0, 0, 64, 127]
        self.assertEqual(list(struct.unpack_from("<10I", choices["expected"])), expected)
        self.assertEqual(logits["data"], logits["expected"])
        self.assertEqual(choices["expected"][17 * 4:], probe.SENTINEL * 15)
        self.assertEqual(struct.unpack_from("<I", logits["data"], 17 * probe.N * 4)[0], 0x7fc01234)

    def test_every_lane_and_scan_boundary_has_a_native_fixture(self):
        for step in (0, 1187, 2373):
            winners = set()
            for first_lane in (0, 32):
                for row in range(32):
                    data, winner = probe.row_values("lanes", row, step, first_lane)
                    self.assertEqual(probe.scalar_argmax(data), winner)
                    winners.add(winner)
            self.assertEqual(winners, set(range(step * 64, step * 64 + 64)))

    def test_random_bits_are_deterministic_finite_and_minimum_capacity(self):
        left = probe.make_case(Core, ("random", 1, 1, 0, 0))
        right = probe.make_case(Core, ("random", 1, 1, 0, 0))
        self.assertEqual(left, right)
        probe.validate_case(left)
        self.assertEqual(len(left["buffers"][0]["data"]), probe.N * 4)
        self.assertEqual(len(left["buffers"][1]["data"]), 4)

    def test_malformed_fixture_parameters_reject_without_a_worker(self):
        for spec in [("bad", 1, 32, 0, 0), ("mixed", 0, 32, 0, 0),
                     ("mixed", 33, 33, 0, 0), ("mixed", True, 32, 0, 0),
                     ("mixed", 2, 1, 0, 0), ("lanes", 32, 32, 2374, 0),
                     ("lanes", 32, 32, 0, 64)]:
            with self.assertRaises(RuntimeError):
                probe.make_case(Core, spec)

    def test_active_nonfinite_reference_rejects_on_cpu_only(self):
        for invalid in (float("nan"), float("inf"), -float("inf")):
            data = bytearray(struct.pack("<f", 0.0) * probe.N)
            struct.pack_into("<f", data, (probe.N - 1) * 4, invalid)
            with self.assertRaises(RuntimeError):
                probe.scalar_argmax(data)

    def test_helper_substitution_is_rejected_before_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            helper = Path(directory) / "helper.py"
            helper.write_text("raise AssertionError('must never execute')\n")
            with self.assertRaises(RuntimeError):
                probe.load_helper(helper)

    def test_conflicting_execution_modes_reject_before_helper_access(self):
        with self.assertRaises(RuntimeError):
            probe.main(["--self-test", "--run", "--helper", "/missing-helper"])
        with self.assertRaises(RuntimeError):
            probe.main(["--helper", "/missing-helper"])

    def test_self_test_requires_no_worker_capability(self):
        with contextlib.redirect_stdout(io.StringIO()) as output:
            probe.self_test(Core)
        self.assertIn("no GPU", output.getvalue())


if __name__ == "__main__":
    unittest.main()
