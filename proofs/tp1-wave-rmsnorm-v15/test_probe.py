"""CPU fixture/analytical checks, not device math or capability emulation."""
from contextlib import redirect_stderr, redirect_stdout
import copy
from fractions import Fraction
import hashlib
import io
import os
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

import probe


def words(data):
    return [word[0] for word in struct.iter_unpack("<H", data)]


def value(bits):
    return Fraction.from_float(struct.unpack("<f", struct.pack("<I", bits << 16))[0])


class FixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.helper = Path(os.environ.get("FERRIC_RMSNORM_HELPER",
            Path(__file__).resolve().parents[1] / "tensor-parallel-kernels-v1/probe.py"))
        cls.core = probe.load_helper(cls.helper)
        cls.specs = probe.specifications()
        cls.cases = {spec: probe.make_case(cls.core, spec) for spec in cls.specs}

    def test_exact_eight_case_roster_and_contract(self):
        self.assertEqual(self.specs, tuple((family, rows) for family in ("uniform", "signed")
                                          for rows in (1, 16, 17, 32)))
        self.assertEqual(probe.ROOT, "ferric_qwen3_tp_batch32_wave_rmsnorm_bf16_v15")
        self.assertEqual(probe.EXPECTED_ABI, (14, 96, 96, 352))
        self.assertEqual(probe.WAVE, 64)
        self.assertEqual(struct.unpack("<I", struct.pack("<f", 1e-6))[0], probe.EPSILON_BITS)
        for spec, case in self.cases.items():
            probe.validate_case(self.core, case)
            self.assertEqual(case["groups"], spec[1])
            self.assertEqual(case["scalars"], [spec[1], 4096, 0x358637BD, 0])

    def test_complete_active_buffers_and_empty_auxiliaries(self):
        for (_, rows), case in self.cases.items():
            buffers = case["buffers"]
            self.assertEqual([buffer["name"] for buffer in buffers],
                             ["input", "residual", "weight", "fused_residual", "normalized"])
            self.assertEqual([buffer["access"] for buffer in buffers], ["read", "read", "read", "write", "write"])
            self.assertEqual([len(buffer["data"]) for buffer in buffers], [rows * 8192, 0, 8192, 0, rows * 8192])
            for buffer in buffers:
                self.assertEqual(len(buffer["data"]), len(buffer["expected"]))
            self.assertEqual(buffers[1]["expected"], b"")
            self.assertEqual(buffers[3]["expected"], b"")
        self.assertEqual(sum(len(case["buffers"]) for case in self.cases.values()), 40)
        self.assertEqual(len(self.core.GUARD), 64)
        largest = self.cases[("signed", 32)]
        self.assertEqual(sum(len(buffer["data"]) + 2 * len(self.core.GUARD) for buffer in largest["buffers"]), 533120)

    def test_uniform_exact_bytes_and_output_sentinel(self):
        for rows in (1, 16, 17, 32):
            buffers = self.cases[("uniform", rows)]["buffers"]
            self.assertEqual(buffers[0]["data"], struct.pack("<H", 0x3F80) * (rows * 4096))
            self.assertEqual(buffers[2]["data"], struct.pack("<H", 0x3F80) * 4096)
            self.assertEqual(buffers[4]["expected"], buffers[0]["data"])
            self.assertEqual(buffers[4]["data"], struct.pack("<H", 0x55AA) * (rows * 4096))

    def test_signed_distinct_rows_columns_and_every_output(self):
        buffers = self.cases[("signed", 32)]["buffers"]
        weights = words(buffers[2]["data"])
        self.assertEqual(weights, list(range(0x3780, 0x4780)))
        self.assertEqual(len(set(weights)), 4096)
        self.assertEqual(value(weights[0]), Fraction(1, 65536))
        self.assertEqual(value(weights[-1]), 65280)
        inputs, expected = words(buffers[0]["data"]), words(buffers[4]["expected"])
        self.assertEqual(len({tuple(inputs[row * 4096:(row + 1) * 4096]) for row in range(32)}), 32)
        for index, (actual_input, output) in enumerate(zip(inputs, expected, strict=True)):
            row, column = divmod(index, 4096)
            lane, component = column % 64, column // 64
            parity = (bin((row + 1) & lane).count("1") + bin(component).count("1")) % 2
            self.assertEqual(actual_input, 0xBF80 if parity else 0x3F80)
            self.assertEqual(output, weights[column] | (actual_input & 0x8000))
            self.assertNotEqual(output, 0x55AA)

    def test_unit_magnitude_striped_reduction_is_exact(self):
        inputs = words(self.cases[("signed", 32)]["buffers"][0]["data"])
        for row in range(32):
            partials = []
            for lane in range(64):
                squares = [value(inputs[row * 4096 + lane + component * 64]) ** 2 for component in range(64)]
                self.assertEqual(set(squares), {Fraction(1)})
                partials.append(sum(squares))
            self.assertEqual(partials, [64] * 64)
            for offset in (1, 2, 4, 8, 16, 32):
                previous = partials
                partials = [previous[lane] + previous[lane ^ offset] for lane in range(64)]
            self.assertEqual(partials, [4096] * 64)

    def test_exact_rational_rne_bounds_for_every_weight(self):
        epsilon = Fraction.from_float(struct.unpack("<f", struct.pack("<I", probe.EPSILON_BITS))[0])
        stabilized = 1 + Fraction(1, 2**20)
        self.assertLess(stabilized - Fraction(1, 2**24), 1 + epsilon)
        self.assertLess(1 + epsilon, stabilized + Fraction(1, 2**24))
        denominator = 1 + Fraction(1, 2**21)
        self.assertLess((denominator - Fraction(1, 2**24)) ** 2, stabilized)
        self.assertLess(stabilized, (denominator + Fraction(1, 2**24)) ** 2)
        inverse = 1 - Fraction(1, 2**21)
        self.assertLess(inverse - Fraction(1, 2**25), 1 / denominator)
        self.assertLess(1 / denominator, inverse + Fraction(1, 2**25))
        displacement = Fraction(1, 2**21) + Fraction(1, 2**24)
        self.assertLess(displacement, Fraction(1, 2**20))
        for bits in range(0x3780, 0x4780):
            weight = value(bits)
            lower = (value(bits - 1) + weight) / 2
            upper = (weight + value(bits + 1)) / 2
            self.assertGreaterEqual(min(weight - lower, upper - weight) / weight, Fraction(1, 2**9))
            product = weight * inverse
            rounding_bound = weight * Fraction(1, 2**24)
            self.assertGreater(product - rounding_bound, lower)
            self.assertLess(product + rounding_bound, upper)

    def test_rejects_open_specs_and_semantic_mutations(self):
        for spec in (("uniform", True), ("uniform", 2), ("signed", 0), ("signed", 33),
                     ("other", 1), ["uniform", 1], ("uniform", 1, "extra")):
            with self.subTest(spec=spec), self.assertRaises(ValueError):
                probe.make_case(self.core, spec)
        original = self.cases[("signed", 1)]
        mutations = [lambda c: c.update(extra=True), lambda c: c.update(symbol="old-root"),
            lambda c: c.update(name="unknown"), lambda c: c.update(groups=True),
            lambda c: c.update(groups=2), lambda c: c["buffers"].pop()]
        for index, replacement in ((0, True), (0, 2), (1, 4095), (2, 1), (2, 1e-6), (3, 1)):
            mutations.append(lambda c, i=index, v=replacement: c["scalars"].__setitem__(i, v))
        for index in range(5):
            for field, replacement in (("access", "invalid"), ("name", "wrong"), ("element_bytes", True),
                                       ("element_bytes", 4), ("data", b"\0\0"), ("expected", b"\0\0")):
                mutations.append(lambda c, i=index, f=field, v=replacement: c["buffers"][i].__setitem__(f, v))
        mutations.extend([
            lambda c: c["buffers"][3].update(access="read"),
            lambda c: c["buffers"][0].update(data=b"\x80\x7f" + c["buffers"][0]["data"][2:]),
            lambda c: c["buffers"][4].update(expected=c["buffers"][4]["expected"][:-2] + b"\0\0"),
            lambda c: c["buffers"][4].update(data=bytearray(c["buffers"][4]["data"])),
        ])
        for index, mutation in enumerate(mutations):
            case = copy.deepcopy(original)
            mutation(case)
            with self.subTest(mutation=index), self.assertRaises(ValueError):
                probe.validate_case(self.core, case)

    def test_regeneration_has_exact_active_extent_and_no_mutable_aliases(self):
        for rows in (32, 1, 17):
            case = probe.make_case(self.core, ("signed", rows))
            self.assertEqual(len(case["buffers"][-1]["data"]), rows * 8192)
            self.assertEqual(case, self.cases[("signed", rows)])
            case["scalars"][0] = 0
            case["buffers"][-1]["expected"] = b""
            self.assertEqual(probe.make_case(self.core, ("signed", rows)), self.cases[("signed", rows)])

    def test_helper_pin_and_symlink_fail_closed(self):
        self.assertEqual(hashlib.sha256(self.helper.read_bytes()).hexdigest(), probe.HELPER_SHA256)
        with tempfile.TemporaryDirectory() as directory:
            changed = Path(directory) / "changed.py"
            changed.write_bytes(self.helper.read_bytes() + b"\n")
            with self.assertRaises(ValueError):
                probe.load_helper(changed)
            alias = Path(directory) / "alias.py"
            alias.symlink_to(self.helper)
            with self.assertRaises(OSError):
                probe.load_helper(alias)

    def test_cli_is_self_test_only_and_cannot_launch_worker(self):
        with patch.object(probe, "load_helper", return_value=self.core), patch.object(self.core, "Worker") as worker:
            output = io.StringIO()
            with redirect_stdout(output):
                probe.main(["--self-test", "--helper", str(self.helper)])
            self.assertIn("PASS: 8 analytical V15 cases", output.getvalue())
            with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                probe.main(["--run", "--helper", str(self.helper)])
            worker.assert_not_called()


if __name__ == "__main__":
    unittest.main()
