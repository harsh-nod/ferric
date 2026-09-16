import copy
import json
import math
from pathlib import Path
import tempfile
import unittest

import extract_checkpoint as checkpoint
import reference


class CheckpointTests(unittest.TestCase):
    def layout(self):
        return {checkpoint.TENSOR: {"dtype": "BF16", "shape": [1024, 1024],
                                    "data_offsets": [0, 2 * 1024 * 1024]}}

    def test_exact_fixed_layout(self):
        tensor = checkpoint.inspect_layout(self.layout(), 2 * 1024 * 1024)
        self.assertEqual(tensor["shape"], [1024, 1024])

    def test_duplicate_json_names_aliases_holes_shapes_and_extents_reject(self):
        with self.assertRaises(ValueError):
            json.loads('{"a":1,"a":2}', object_pairs_hook=checkpoint.unique_object)
        for mutation in range(8):
            with self.subTest(mutation=mutation):
                header = copy.deepcopy(self.layout())
                value = header[checkpoint.TENSOR]
                size = 2 * 1024 * 1024
                if mutation == 0:
                    header["alias"] = copy.deepcopy(value)
                elif mutation == 1:
                    value["data_offsets"] = [2, size + 2]
                    size += 2
                elif mutation == 2:
                    value["data_offsets"][1] -= 2
                elif mutation == 3:
                    value["shape"] = [512, 2048]
                elif mutation == 4:
                    value["dtype"] = "F16"
                elif mutation == 5:
                    value["data_offsets"][0] = -1
                elif mutation == 6:
                    value["shape"] = [True, 1024 * 1024]
                else:
                    value["dtype"] = []
                with self.assertRaises(ValueError):
                    checkpoint.inspect_layout(header, size)

    def test_wrong_checkpoint_hash_never_writes_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            header = json.dumps(self.layout()).encode()
            import struct
            source = root / "synthetic.safetensors"
            source.write_bytes(struct.pack("<Q", len(header)) + header + bytes(2 * 1024 * 1024))
            with self.assertRaisesRegex(ValueError, "SHA256"):
                checkpoint.extract(source, root / "must-not-exist")
            self.assertFalse((root / "must-not-exist").exists())


class NumericalPolicyTests(unittest.TestCase):
    def test_bf16_widening_and_nonfinite_rejection(self):
        self.assertEqual(reference.bf16(0x3F80), 1.0)
        self.assertEqual(reference.bf16(0xBF00), -0.5)
        for data in (b"\x00\x7f", b"\x80\x7f", b"\xc0\x7f"):
            if data != b"\x00\x7f":
                with self.assertRaises(ValueError):
                    reference.bf16_words(data, 1)
        with self.assertRaises(ValueError):
            reference.bf16_words(b"\x00", 1)

    def test_predeclared_gamma_is_conservative_without_arbitrary_floor(self):
        self.assertGreaterEqual(reference.GAMMA32, 1 / 16383)
        weights = [0x3F80] * (1024 * 1024)
        expected, bounds = reference.evaluate(weights, [0x3F80] * 1024)
        self.assertEqual(expected, [1024.0] * 1024)
        self.assertTrue(all(0.0625 < bound < 0.0626 for bound in bounds))
        metrics = reference.compare(expected, expected, bounds, [])
        self.assertEqual(metrics["maximum_absolute_error"], 0.0)
        incorrect = list(expected)
        incorrect[0] += bounds[0] * 2
        with self.assertRaisesRegex(ValueError, "predeclared"):
            reference.compare(incorrect, expected, bounds, [])

    def test_zero_basis_and_cancellation_have_exact_rows(self):
        weights = [0x3F80] * (1024 * 1024)
        for name in ("zero", "basis", "cancellation"):
            inputs, exact = reference.case_inputs(name, weights)
            expected, bounds = reference.evaluate(weights, inputs)
            reference.compare(expected, expected, bounds, exact)
            incorrect = list(expected)
            incorrect[exact[0]] = math.nextafter(incorrect[exact[0]], math.inf)
            with self.assertRaisesRegex(ValueError, "exact"):
                reference.compare(incorrect, expected, bounds, exact)
            if name in ("zero", "cancellation"):
                self.assertEqual(expected[0], 0.0)
            if name == "zero":
                self.assertEqual(bounds, [0.0] * 1024)

    def test_denormal_and_overflow_domains_are_explicitly_rejected(self):
        with self.assertRaisesRegex(ValueError, "lattice"):
            reference.evaluate([1] * (1024 * 1024), [1] * 1024)
        with self.assertRaisesRegex(ValueError, "overflow"):
            reference.evaluate([0x7F7F] * (1024 * 1024), [0x7F7F] * 1024)

    def test_output_extent_nonfinite_and_invalid_bounds_reject(self):
        for actual, expected, bounds in (([0.0], [0.0], [0.0]),
                                         ([math.nan] * 1024, [0.0] * 1024, [0.0] * 1024),
                                         ([0.0] * 1024, [0.0] * 1024, [-1.0] * 1024)):
            with self.assertRaises(ValueError):
                reference.compare(actual, expected, bounds, [])


if __name__ == "__main__":
    unittest.main()
