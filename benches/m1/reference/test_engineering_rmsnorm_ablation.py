#!/usr/bin/env python3
"""Pinned CPU-only RMSNorm regressions; no checkpoint or full-model execution."""

import importlib.util
from contextlib import ExitStack
from pathlib import Path
import struct
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch


spec = importlib.util.spec_from_file_location(
    "rmsnorm_ablation", Path(__file__).with_name("engineering_rmsnorm_ablation.py")
)
if spec is None or spec.loader is None:
    raise ImportError("cannot load RMSNorm ablation")
ablation = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = ablation
spec.loader.exec_module(ablation)


class CpuThreadConfigurationTests(unittest.TestCase):
    def loader(self, count, setter_error=None, silent_setter=False):
        state = {"count": count}

        def set_interop(value):
            if setter_error is not None:
                raise setter_error
            if not silent_setter:
                state["count"] = value

        modules = {
            name: SimpleNamespace(__version__=version)
            for name, version in ablation.core.DEPENDENCY_VERSIONS.items()
        }
        torch = modules["torch"]
        torch.version = SimpleNamespace(hip="7.2.test")
        torch.get_num_interop_threads = Mock(side_effect=lambda: state["count"])
        torch.set_num_interop_threads = Mock(side_effect=set_interop)
        torch.set_num_threads = Mock()
        torch.set_default_device = Mock()
        qwen = SimpleNamespace()
        imports = {"triton" if name == "triton-rocm" else name: module
                   for name, module in modules.items()}
        imports["transformers.models.qwen3.modeling_qwen3"] = qwen
        stack = ExitStack()
        self.addCleanup(stack.close)
        checks = {
            name: stack.enter_context(patch.object(ablation.core, name))
            for name in ("require_isolated_python", "require_virtual_environment",
                         "validate_dependency_provenance")
        }
        checks["qwen"] = stack.enter_context(
            patch.object(ablation, "qwen_source_identity", return_value={}))
        checks["version"] = stack.enter_context(
            patch("importlib.metadata.version",
                  return_value=ablation.core.DEPENDENCY_VERSIONS["triton-rocm"]))
        stack.enter_context(patch.object(ablation.importlib, "import_module",
                                        side_effect=imports.__getitem__))
        return torch, state, checks

    def test_already_one_is_not_reset_on_repeated_loads(self):
        torch, _, checks = self.loader(1)
        for _ in range(2):
            self.assertIs(ablation.load_cpu_dependencies()[0], torch)
        torch.set_num_interop_threads.assert_not_called()
        self.assertEqual(torch.get_num_interop_threads.call_count, 4)
        for check in checks.values():
            self.assertEqual(check.call_count, 2)
        self.assertEqual(torch.set_default_device.call_args_list,
                         [unittest.mock.call("cpu"), unittest.mock.call("cpu")])

    def test_other_count_is_configured_and_verified(self):
        torch, state, _ = self.loader(4)
        ablation.load_cpu_dependencies()
        torch.set_num_interop_threads.assert_called_once_with(1)
        self.assertEqual(state["count"], 1)
        self.assertEqual(torch.get_num_interop_threads.call_count, 2)

    def test_setter_failure_propagates(self):
        error = RuntimeError("interop configuration locked")
        torch, _, checks = self.loader(4, setter_error=error)
        with self.assertRaises(RuntimeError) as caught:
            ablation.load_cpu_dependencies()
        self.assertIs(caught.exception, error)
        torch.set_num_interop_threads.assert_called_once_with(1)
        checks["qwen"].assert_not_called()

    def test_silent_setter_with_wrong_count_is_rejected(self):
        torch, _, checks = self.loader(4, silent_setter=True)
        with self.assertRaisesRegex(ablation.Failure, "interop thread count"):
            ablation.load_cpu_dependencies()
        torch.set_num_interop_threads.assert_called_once_with(1)
        self.assertEqual(torch.get_num_interop_threads.call_count, 2)
        checks["qwen"].assert_not_called()

    def test_repeated_load_rechecks_provenance_and_propagates_drift(self):
        torch, _, checks = self.loader(1)
        error = ablation.Failure("dependency provenance drift")
        checks["validate_dependency_provenance"].side_effect = [None, error]
        ablation.load_cpu_dependencies()
        with self.assertRaises(ablation.Failure) as caught:
            ablation.load_cpu_dependencies()
        self.assertIs(caught.exception, error)
        for name in ("require_isolated_python", "require_virtual_environment",
                     "validate_dependency_provenance", "version"):
            self.assertEqual(checks[name].call_count, 2)
        checks["qwen"].assert_called_once()
        torch.set_num_interop_threads.assert_not_called()


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
