#!/usr/bin/env python3
"""CPU-only final-RMS protocol and hook-lifecycle regressions; no GPU evidence."""

import copy
import importlib.util
import json
from pathlib import Path
import struct
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch


def load(name, filename):
    specification = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    if specification is None or specification.loader is None:
        raise ImportError("cannot load final-RMS test module")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


diagnostic = load("m5_final_rms_diagnostic", "engineering_m5_final_rms_reference.py")
fixtures = load("m5_final_rms_fixtures", "test_engineering_m5_reference.py")


def fixture():
    capture, logits = fixtures.fixture()
    data = diagnostic.core.canonical_bytes(capture)
    row = struct.pack("<H", 0x3F80) * diagnostic.WIDTH
    base = capture["rows"][0]["offset_bytes"]
    document = {
        "format": "FERRIC-ENGINEERING-S1-K4-FINAL-RMS-V1", "authority": "none",
        "qualification": False, "benchmark_comparable": False,
        "numerical_comparison_performed": False,
        "retained_payload_scope": "all-target-logits-and-final-rms-row3-only",
        "target_segment": 4, "active_index": 3, "position": 131,
        "input_token_id": capture["target_token_ids"][3], "dispatch_generation": capture["dispatch_generation"],
        "capture_sha256": diagnostic.reference.digest(data), "epsilon_f32_bits": 0x358637BD,
        "weight_tensor": "model.norm.weight",
        "scope": "Synthetic opt-in storage fixture; no hardware execution.",
        "nonclaim": "No completed-copy replay or qualification.",
        "completed_readback": {"offset_bytes": base, "bytes": diagnostic.READBACK_BYTES, "sha256": "a" * 64},
    }
    payloads = {"capture.json": data, "target-logits.bf16": logits}
    for index, name in enumerate(("input", "output")):
        filename = f"target-final-rms-{name}.bf16"
        payloads[filename] = row
        document[name] = {
            "path": filename, "shape": [4096], "dtype": "bf16-little-endian", "bytes": 8192,
            "sha256": diagnostic.reference.digest(row),
            "offset_bytes": base + diagnostic.LOGITS_BYTES + (index * 5 + 3) * diagnostic.ROW_BYTES,
        }
    payloads["final-rms-capture.json"] = diagnostic.core.canonical_bytes(document)
    return payloads, document


class HookNorm:
    def __init__(self, fail_output_registration=False):
        self.hooks = {}
        self.fail_output_registration = fail_output_registration

    def register_forward_pre_hook(self, hook):
        self.hooks["input"] = hook
        return SimpleNamespace(remove=lambda: self.hooks.pop("input"))

    def register_forward_hook(self, hook):
        if self.fail_output_registration:
            raise RuntimeError("registration failed")
        self.hooks["output"] = hook
        return SimpleNamespace(remove=lambda: self.hooks.pop("output"))

    def forward(self):
        self.hooks["input"](self, ("input-tensor",))
        self.hooks["output"](self, ("input-tensor",), "output-tensor")


class FinalRmsTests(unittest.TestCase):
    def test_valid_extension_preserves_real_sequence_and_exact_row_coordinates(self):
        payloads, document = fixture()
        sequence, validated = diagnostic.validate_capture(payloads)
        self.assertEqual(len(sequence), 133)
        self.assertEqual(sequence[131], 47)
        self.assertEqual(validated, document)
        self.assertEqual(diagnostic.READBACK_BYTES, 1601280)

    def test_transcript_generation_geometry_and_authority_substitutions_reject(self):
        original, document = fixture()
        for path, value in (
            (("qualification",), 0), (("qualification",), True), (("authority",), "protected"),
            (("dispatch_generation",), 18), (("input_token_id",), 13),
            (("position",), 131.0), (("active_index",), 2), (("target_segment",), 3),
            (("epsilon_f32_bits",), 0), (("capture_sha256",), "b" * 64),
            (("weight_tensor",), "model.layers.0.input_layernorm.weight"),
            (("completed_readback", "bytes"), diagnostic.READBACK_BYTES - 2),
            (("completed_readback", "offset_bytes"), 66),
            (("completed_readback", "sha256"), "invalid"),
            (("input", "offset_bytes"), document["output"]["offset_bytes"]),
            (("input", "path"), "../target-final-rms-input.bf16"),
            (("input", "shape"), [4096.0]), (("output", "sha256"), "b" * 64),
            (("output", "dtype"), "float32"), (("retained_payload_scope",), "all-rows"),
        ):
            changed = copy.deepcopy(document)
            node = changed
            for key in path[:-1]:
                node = node[key]
            node[path[-1]] = value
            payloads = dict(original, **{"final-rms-capture.json": diagnostic.core.canonical_bytes(changed)})
            with self.subTest(path=path, value=value), self.assertRaises(diagnostic.Failure):
                diagnostic.validate_capture(payloads)

    def test_capture_byte_binding_is_not_semantic_json_equality(self):
        payloads, _ = fixture()
        payloads["capture.json"] += b"\n"
        with self.assertRaisesRegex(diagnostic.Failure, "capture_sha256"):
            diagnostic.validate_capture(payloads)

    def test_unknown_fields_wrong_object_types_and_nontext_prose_reject(self):
        original, document = fixture()
        for changed in (dict(document, cause_established=True), [], None,
                        dict(document, scope=False), dict(document, nonclaim=""),
                        dict(document, completed_readback=[])):
            payloads = dict(original, **{"final-rms-capture.json": diagnostic.core.canonical_bytes(changed)})
            with self.subTest(changed=type(changed).__name__), self.assertRaises(diagnostic.Failure):
                diagnostic.validate_capture(payloads)

    def test_legacy_geometry_cannot_substitute_bool_or_float_for_integer(self):
        original, document = fixture()
        for path, value in ((("shape", 0), True), (("positions", 3), 131.0),
                            (("rows", 3, "position"), 131.0), (("rows", 1, "active_index"), True),
                            (("rows", 0, "bytes"), float(diagnostic.reference.ROW_BYTES)),
                            (("logits", "bytes"), float(diagnostic.LOGITS_BYTES))):
            capture = json.loads(original["capture.json"])
            node = capture
            for key in path[:-1]:
                node = node[key]
            node[path[-1]] = value
            data = diagnostic.core.canonical_bytes(capture)
            changed = dict(document, capture_sha256=diagnostic.reference.digest(data))
            payloads = dict(original, **{"capture.json": data,
                            "final-rms-capture.json": diagnostic.core.canonical_bytes(changed)})
            with self.subTest(path=path), self.assertRaises(diagnostic.Failure):
                diagnostic.validate_capture(payloads)

    def test_nested_smoke_generation_rejects_float_even_with_matching_capture_sha(self):
        original, document = fixture()
        capture = json.loads(original["capture.json"])
        capture["smoke"]["speculative_k4"]["dispatch_generation"] = float(capture["dispatch_generation"])
        data = diagnostic.core.canonical_bytes(capture)
        changed = dict(document, capture_sha256=diagnostic.reference.digest(data))
        payloads = dict(original, **{"capture.json": data,
                        "final-rms-capture.json": diagnostic.core.canonical_bytes(changed)})
        with self.assertRaisesRegex(diagnostic.Failure, "nested smoke dispatch generation"):
            diagnostic.validate_capture(payloads)

    def test_unexpected_or_missing_file_and_duplicate_json_keys_reject(self):
        original, _ = fixture()
        for payloads in (dict(original, unexpected=b""),
                         {name: data for name, data in original.items() if name != "target-final-rms-input.bf16"}):
            with self.assertRaises(diagnostic.Failure):
                diagnostic.validate_capture(payloads)
        original["final-rms-capture.json"] = original["final-rms-capture.json"].replace(
            b'"position": 131', b'"position": 131, "position": 131')
        with self.assertRaises((diagnostic.Failure, ValueError)):
            diagnostic.validate_capture(original)

    def test_nonfinite_and_wrong_extent_reject_even_with_matching_sha(self):
        original, document = fixture()
        for data in (bytes(8190), bytes(8194), struct.pack("<H", 0x7F80) + bytes(8190),
                     struct.pack("<H", 0xFF80) + bytes(8190), struct.pack("<H", 0x7FC0) + bytes(8190)):
            payloads = dict(original)
            payloads["target-final-rms-input.bf16"] = data
            changed = copy.deepcopy(document)
            changed["input"]["sha256"] = diagnostic.reference.digest(data)
            payloads["final-rms-capture.json"] = diagnostic.core.canonical_bytes(changed)
            with self.subTest(length=len(data)), self.assertRaises(diagnostic.Failure):
                diagnostic.validate_capture(payloads)

    def test_metric_keeps_bit_identity_and_numeric_signed_zero_distinct(self):
        zeros = bytes(8192)
        result = diagnostic.row_metrics(struct.pack("<H", 0x8000) + bytes(8190), zeros)
        self.assertEqual(result["bit_mismatches"], 1)
        self.assertEqual(result["max_bf16_ulp"], 0)
        self.assertEqual(result["rmse"], 0.0)
        result = diagnostic.row_metrics(struct.pack("<H", 0x3F80) + bytes(8190), zeros)
        self.assertEqual(result["max_absolute_error"], 1.0)
        self.assertEqual(result["rmse"], 1 / 64)
        self.assertNotIn("accepted", result)

    def test_hook_capture_records_once_and_removes_both_handles(self):
        norm = HookNorm()
        with patch.object(diagnostic, "tensor_row", side_effect=[b"input", b"output"]) as serializer:
            with diagnostic.capture_hooks(norm, "torch", "device") as captured:
                norm.forward()
            self.assertEqual(captured, {"input": b"input", "output": b"output"})
            self.assertEqual(serializer.call_count, 2)
        self.assertEqual(norm.hooks, {})

    def test_hook_exception_duplicate_or_missing_invocation_always_removes_handles(self):
        for mode in ("body-error", "duplicate", "missing", "bad-arguments"):
            norm = HookNorm()
            with patch.object(diagnostic, "tensor_row", return_value=bytes(8192)):
                with self.subTest(mode=mode), self.assertRaises((RuntimeError, diagnostic.Failure)):
                    with diagnostic.capture_hooks(norm, None, None):
                        if mode == "body-error":
                            raise RuntimeError("forward failed")
                        if mode == "duplicate":
                            norm.forward()
                            norm.forward()
                        if mode == "bad-arguments":
                            norm.hooks["input"](norm, ())
            self.assertEqual(norm.hooks, {})

    def test_failed_second_hook_registration_removes_first(self):
        norm = HookNorm(fail_output_registration=True)
        with self.assertRaises(RuntimeError):
            with diagnostic.capture_hooks(norm, None, None):
                self.fail("registration unexpectedly succeeded")
        self.assertEqual(norm.hooks, {})

    def test_cpu_model_cannot_enter_gpu_reference(self):
        model = SimpleNamespace(parameters=lambda: iter([SimpleNamespace(device=SimpleNamespace(type="cpu"))]))
        with self.assertRaises(diagnostic.Failure):
            diagnostic.execute(model, None, [0] * 133)


if __name__ == "__main__":
    unittest.main()
