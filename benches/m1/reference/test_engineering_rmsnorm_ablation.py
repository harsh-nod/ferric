#!/usr/bin/env python3
"""Pinned CPU-only RMSNorm regressions; no checkpoint or full-model execution."""

import importlib.util
from pathlib import Path
import struct
import sys
import unittest


spec = importlib.util.spec_from_file_location(
    "rmsnorm_ablation", Path(__file__).with_name("engineering_rmsnorm_ablation.py")
)
if spec is None or spec.loader is None:
    raise ImportError("cannot load RMSNorm ablation")
ablation = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = ablation
spec.loader.exec_module(ablation)


class AblationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.torch, _, cls.qwen, _, _ = ablation.load_cpu_dependencies()

    def test_existing_serial_reduction_witness(self):
        bits = [
            16295, 16734, 15610, 16917, 16782, 17267, 16001, 15365, 17397, 15627, 16562, 15898, 15809,
            16186, 17041, 16392, 16277, 16725, 15587, 16964, 16853, 17263, 16043, 15361, 17318, 15708,
            16557, 15923, 15850, 16176, 17133, 16399, 16327, 16704, 15535, 16903, 16800, 17182, 16120,
            15441, 17306, 15648, 16523, 15968, 15767, 16218, 17132, 16489, 16317, 16670, 15584, 16991,
            16879, 17154, 16106, 15478, 17362, 15705, 16590, 15906, 15816, 16184, 17040, 16407, 16375,
            16753, 15605, 16970, 16834, 17178, 16127, 15470, 17358, 15622, 16628, 15991, 15869, 16201,
            17112, 16410, 16373, 16696, 15597, 16969, 16793, 17254, 16057, 15418, 17294, 15655, 16639,
            15968, 15798, 16143, 17091, 16496, 16311, 16755, 15562, 16989, 16884, 17254, 16022, 15451,
            17298, 15676, 16621, 15966, 15860, 16137, 17107, 16410, 16317, 16674, 15498, 16900, 16852,
            17177, 16024, 15439, 17370, 15685, 16576, 15983, 15797, 16183, 17030, 16408,
        ]
        x = self.torch.tensor(bits, dtype=self.torch.int16).view(self.torch.bfloat16).reshape(1, 128)
        self.assertEqual(ablation.scalar_bits(ablation.serial_square_sum(x, self.torch), self.torch),
                         "0x49be1c17")

    def test_existing_intermediate_bf16_boundary_witness(self):
        torch = self.torch
        normalized = torch.tensor([-1.5510881], dtype=torch.float32)
        weight = torch.tensor([0.546875], dtype=torch.bfloat16)
        _, _, output = ablation.rounded_weighting(normalized, weight, torch)
        old_boundary = (normalized * weight.float()).to(torch.bfloat16)
        self.assertEqual(output.item(), -0.8515625)
        self.assertEqual(old_boundary.item(), -0.84765625)

    def test_actual_hf_and_three_variants_preserve_inputs_and_raw_intermediates(self):
        torch = self.torch
        x = (((torch.arange(128, dtype=torch.int64) * 17) % 1021 - 510).float() / 64)
        x = x.to(torch.bfloat16).reshape(1, 128)
        weight = torch.ones(128, dtype=torch.bfloat16)
        before = (ablation.tensor_bytes(x, torch), ablation.tensor_bytes(weight, torch))
        cases, comparisons, payloads = ablation.ablate(x, weight, torch, self.qwen)
        self.assertEqual(set(cases), {"hf", "serial-rsqrt", "serial-sqrt-reciprocal"})
        self.assertEqual(len(comparisons), 3)
        self.assertEqual(before, (ablation.tensor_bytes(x, torch), ablation.tensor_bytes(weight, torch)))
        self.assertEqual(len(payloads), 14)
        for name in cases:
            self.assertEqual(len(payloads[f"{name}.normalized.f32"]), 128 * 4)
            self.assertEqual(len(payloads[f"{name}.output.bf16"]), 128 * 2)
        self.assertEqual(cases["serial-rsqrt"]["mean_square_bits"],
                         cases["serial-sqrt-reciprocal"]["mean_square_bits"])
        self.assertEqual(cases["serial-rsqrt"]["stabilized_bits"],
                         cases["serial-sqrt-reciprocal"]["stabilized_bits"])

    def test_zero_row_is_finite_in_all_three_cases(self):
        torch = self.torch
        _, comparisons, payloads = ablation.ablate(torch.zeros((1, 128), dtype=torch.bfloat16),
                                                   torch.ones(128, dtype=torch.bfloat16), torch, self.qwen)
        self.assertTrue(all(item["bit_mismatches"] == 0 for item in comparisons.values()))
        self.assertEqual(payloads["hf.output.bf16"], bytes(256))

    def test_bad_shapes_dtypes_and_nonfinite_inputs_reject(self):
        torch = self.torch
        x = torch.ones((1, 128), dtype=torch.bfloat16)
        weight = torch.ones(128, dtype=torch.bfloat16)
        for bad_x, bad_weight in ((x.float(), weight), (x, weight.float()),
                                  (x[:, :127], weight), (x.repeat(2, 1), weight),
                                  (x * float("inf"), weight), (x, weight * float("nan"))):
            with self.assertRaises(ablation.Failure):
                ablation.validate_inputs(bad_x, bad_weight, torch)

    def test_nonfinite_serial_square_sum_rejects(self):
        torch = self.torch
        maximum = torch.finfo(torch.bfloat16).max
        with self.assertRaises(ablation.Failure):
            ablation.serial_square_sum(torch.full((1, 128), maximum, dtype=torch.bfloat16), torch)

    def test_raw_differences_preserve_bit_identity_including_signed_zero(self):
        result = ablation.compare_outputs(struct.pack("<HH", 0x8000, 0x3F80),
                                           struct.pack("<HH", 0x0000, 0x3F80))
        self.assertEqual(result, {"elements": 2, "bit_mismatches": 1,
                                  "differences": [{"index": 0, "left_bits": "0x8000",
                                                   "right_bits": "0x0000"}]})
        with self.assertRaises(ablation.Failure):
            ablation.compare_outputs(b"\x00", b"\x00")


if __name__ == "__main__":
    unittest.main()
