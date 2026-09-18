#!/usr/bin/env python3
"""Focused CPU tests for the same-input layer-0 residual diagnostic."""

from contextlib import nullcontext
import copy
import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


def load_sibling(module_name: str, filename: str):
    specification = importlib.util.spec_from_file_location(
        module_name, Path(__file__).with_name(filename)
    )
    if specification is None or specification.loader is None:
        raise ImportError(f"cannot load sibling {filename}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[specification.name] = module
    specification.loader.exec_module(module)
    return module


subject = load_sibling(
    "tp1_layer0_residual_ablation",
    "engineering_tp1_layer0_residual_ablation.py",
)
fixtures = load_sibling(
    "tp1_layer0_residual_secure_fixtures",
    "test_engineering_tp1_rmsnorm_ablation.py",
)


def encoded(value) -> bytes:
    return json.dumps(value, indent=2, allow_nan=False).encode()


def boundary_fixture():
    value = fixtures.native_fixture()
    setup = copy.deepcopy(value.manifest["setup"])
    setup.pop("final_stage_capture")
    setup["residual_boundary_capture"] = {
        "positions": value.pins["positions"],
        "layer": 0,
        "operation": "AttentionOutputSum",
        "tensor_parallel_rank": 0,
        "tensor_parallel_world": 1,
        "output_projection_weight_tensor": subject.WEIGHT,
        "output_projection_weight_shape": [subject.WIDTH, subject.WIDTH],
        "projection_input_elements": subject.WIDTH,
        "residual_elements": subject.WIDTH,
        "projection_partial_elements": subject.WIDTH,
        "hidden_after_elements": subject.WIDTH,
        "residual_source": "host_staged_collective_input",
        "output_projection_weight_shard_sha256": "b" * 64,
        "benchmark_comparable": False,
        "timing": "diagnostic readbacks invalidate all performance measurements",
    }
    rows = len(value.pins["positions"])
    data = {
        "projection-input.bf16": bytes(rows * subject.WIDTH * 2),
        "residual-before.bf16": bytes(rows * subject.WIDTH * 2),
        "projection-partial.f32le": bytes(rows * subject.WIDTH * 4),
        "hidden-after-broadcast.bf16": bytes(rows * subject.WIDTH * 2),
    }
    manifest = {
        "schema": subject.CAPTURE_SCHEMA,
        "authority": "none",
        "complete": True,
        "worker_close_confirmed": True,
        "qualification": False,
        "benchmark_comparable": False,
        "numerical_pass_claimed": False,
        "setup": setup,
        "closed": value.manifest["closed"],
        "positions": value.manifest["positions"],
        "epoch": value.manifest["epoch"],
        "input_tokens": value.manifest["input_tokens"],
        "gpu_choices": value.manifest["gpu_choices"],
        "rows": [],
        "payloads": [],
        "maximum_total_bytes": subject.MAX_CAPTURE_BYTES,
        "nonclaim": "raw boundary bytes only",
    }
    for key, filename, dtype, element_bytes in subject.PAYLOADS:
        raw = data[filename]
        manifest["payloads"].append({
            "file": filename,
            "shape": [rows, subject.WIDTH],
            "dtype": dtype,
            "element_bytes": element_bytes,
            "bytes": len(raw),
            "sha256": subject.digest(raw),
        })
    for ordinal, position in enumerate(manifest["positions"]):
        row = {
            "position": position,
            "input_token": manifest["input_tokens"][position],
            "layer": 0,
            "operation": "AttentionOutputSum",
            "gpu_hidden_matches_host_broadcast": True,
        }
        for key, filename, _, element_bytes in subject.PAYLOADS:
            extent = subject.WIDTH * element_bytes
            part = data[filename][ordinal * extent : (ordinal + 1) * extent]
            row[key] = {
                "file": filename,
                "offset_bytes": ordinal * extent,
                "bytes": extent,
                "sha256": subject.digest(part),
            }
        manifest["rows"].append(row)
    intent = {
        "schema": subject.INTENT_SCHEMA,
        "authority": "none",
        "complete": False,
        "setup": setup,
        "positions": manifest["positions"],
        "required_steps": len(value.pins["prompt_tokens"]) + value.pins["new_tokens"] - 1,
        "benchmark_comparable": False,
        "qualification": False,
    }
    data["intent.json"] = encoded(intent)
    data["manifest.json"] = encoded(manifest)
    receipt = {
        "schema": subject.RECEIPT_SCHEMA,
        "authority": "none",
        "qualification": False,
        "benchmark_comparable": False,
        "manifest_sha256": subject.digest(data["manifest.json"]),
        "manifest_bytes": len(data["manifest.json"]),
    }
    witness = dict(value.witness)
    witness["stdout.txt"] = b"".join(
        json.dumps(record).encode() + b"\n"
        for record in (setup, value.measurement, manifest["closed"], receipt)
    )
    return SimpleNamespace(
        pins=value.pins,
        manifest=manifest,
        data=data,
        witness=witness,
    )


def admit(value):
    pin_bytes = encoded(value.pins)
    pins = subject.reference.validate_pins(pin_bytes, subject.digest(pin_bytes))
    return subject.validate_capture(value.data, value.witness, pins)


def native_row(position=4):
    return {
        "position": position,
        "input_token": 374,
        "gpu_hidden_matches_host_broadcast": False,
    }


def native_payloads():
    return {
        "projection_input": bytes(subject.WIDTH * 2),
        "residual_before": bytes(subject.WIDTH * 2),
        "projection_partial": bytes(subject.WIDTH * 4),
        "hidden_after_broadcast": bytes(subject.WIDTH * 2),
    }


def replay_passes():
    bf16 = bytes(subject.WIDTH * 2)
    fp32 = bytes(subject.WIDTH * 4)
    return {
        "torch_bf16_linear": [
            {"projection_bf16": bf16, "partial_f32": fp32,
             "sum_f32": fp32, "hidden_bf16": bf16}
            for _ in range(2)
        ],
        "source_ascending_fp32": [
            {"projection_bf16": bf16, "partial_f32": fp32,
             "sum_f32": fp32, "hidden_bf16": bf16}
            for _ in range(2)
        ],
        "native_partial_host": [
            {"sum_f32": fp32, "hidden_bf16": bf16} for _ in range(2)
        ],
        "native_partial_bf16_first": [
            {"projection_bf16": bf16, "partial_f32": fp32,
             "sum_f32": fp32, "hidden_bf16": bf16}
            for _ in range(2)
        ],
    }


class CaptureAdmissionTests(unittest.TestCase):
    def test_exact_capture_and_tensor_identity_are_admitted(self):
        value = boundary_fixture()
        manifest = admit(value)
        self.assertEqual(manifest["schema"], subject.CAPTURE_SCHEMA)
        self.assertEqual(
            manifest["setup"]["residual_boundary_capture"][
                "output_projection_weight_shard_sha256"
            ],
            "b" * 64,
        )
        self.assertEqual(set(value.data), subject.CAPTURE_FILES)

    def test_capture_roster_terminal_and_descriptors_fail_closed(self):
        mutations = []
        mutations.append(lambda value: value.data.pop("projection-input.bf16"))
        mutations.append(lambda value: value.witness.__setitem__("wrapper.exit", b"1\n"))
        mutations.append(lambda value: value.manifest["rows"][0].__setitem__("layer", 1))
        mutations.append(
            lambda value: value.manifest["setup"]["residual_boundary_capture"].__setitem__(
                "output_projection_weight_tensor", "wrong"
            )
        )
        for mutate in mutations:
            value = boundary_fixture()
            mutate(value)
            if "manifest.json" in value.data:
                value.data["manifest.json"] = encoded(value.manifest)
                receipt = json.loads(value.witness["stdout.txt"].splitlines()[3])
                receipt["manifest_sha256"] = subject.digest(value.data["manifest.json"])
                receipt["manifest_bytes"] = len(value.data["manifest.json"])
                lines = value.witness["stdout.txt"].splitlines()
                value.witness["stdout.txt"] = b"\n".join(lines[:3] + [json.dumps(receipt).encode()]) + b"\n"
            with self.subTest(mutate=mutate), self.assertRaises(subject.Failure):
                admit(value)

    def test_consistent_wrong_tensor_metadata_reaches_exact_selection_guard(self):
        value = boundary_fixture()
        setup = value.manifest["setup"]
        setup["residual_boundary_capture"]["output_projection_weight_tensor"] = (
            "model.layers.1.self_attn.o_proj.weight"
        )
        intent = json.loads(value.data["intent.json"])
        intent["setup"] = setup
        value.data["intent.json"] = encoded(intent)
        value.data["manifest.json"] = encoded(value.manifest)
        records = [json.loads(line) for line in value.witness["stdout.txt"].splitlines()]
        records[0] = setup
        records[3]["manifest_sha256"] = subject.digest(value.data["manifest.json"])
        records[3]["manifest_bytes"] = len(value.data["manifest.json"])
        value.witness["stdout.txt"] = b"".join(
            json.dumps(record).encode() + b"\n" for record in records
        )
        with self.assertRaisesRegex(subject.Failure, "^setup residual_boundary_capture drifted$"):
            admit(value)

    def test_native_rows_slice_all_four_exact_operands(self):
        value = boundary_fixture()
        value.data["projection-input.bf16"] = bytes(range(16)) * (subject.WIDTH * 4 // 16)
        rows = subject.native_rows(value.data, value.manifest)
        self.assertEqual(len(rows), 2)
        self.assertEqual(len(rows[0]["projection_input"]), subject.WIDTH * 2)
        self.assertEqual(
            rows[1]["projection_input"],
            value.data["projection-input.bf16"][subject.WIDTH * 2 :],
        )


class BindingTests(unittest.TestCase):
    def test_weight_binding_is_exact(self):
        subject.parse_weight_binding(
            encoded({"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD}}),
            encoded({"hidden_size": subject.WIDTH, "num_hidden_layers": 36}),
        )
        for index, config in (
            ({"weight_map": {subject.WEIGHT: "wrong"}},
             {"hidden_size": subject.WIDTH, "num_hidden_layers": 36}),
            ({"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD}},
             {"hidden_size": 1, "num_hidden_layers": 36}),
            ({"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD}},
             {"hidden_size": subject.WIDTH, "num_hidden_layers": 0}),
        ):
            with self.subTest(index=index, config=config), self.assertRaises(subject.Failure):
                subject.parse_weight_binding(encoded(index), encoded(config))


class PinnedTorchArithmeticTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.torch, _, _, _, _ = subject.ablation.load_cpu_dependencies()

    def test_actual_torch_linear_and_source_order_are_repeated(self):
        torch = self.torch
        activation = torch.tensor([[1, 1, 1, 1]], dtype=torch.bfloat16, device="cpu")
        weight = torch.tensor(
            [[2**24, 1, -(2**24), 1], [1, 2, 3, 4]],
            dtype=torch.bfloat16,
            device="cpu",
        )
        first = subject.torch_linear_once(activation, weight, torch)
        second = subject.torch_linear_once(activation, weight, torch)
        self.assertEqual(first, second)
        by_inner = weight.transpose(0, 1).to(torch.float32).contiguous()
        source1 = subject.ascending_fp32_once(activation, by_inner, torch)
        source2 = subject.ascending_fp32_once(activation, by_inner, torch)
        self.assertEqual(source1, source2)
        self.assertEqual(struct.unpack("<2f", source1[0]), (1.0, 10.0))

    def test_host_residual_uses_fp32_add_and_manual_bf16_rne(self):
        torch = self.torch
        partial = struct.pack("<f", 1.00390625) + bytes((subject.WIDTH - 1) * 4)
        residual = bytes(subject.WIDTH * 2)
        summed, rounded = subject.host_residual_once(partial, residual, torch)
        self.assertEqual(struct.unpack_from("<I", summed)[0], 0x3F808000)
        self.assertEqual(struct.unpack_from("<H", rounded)[0], 0x3F80)
        partial = struct.pack("<f", struct.unpack("<f", struct.pack("<I", 0x3F818000))[0]) + bytes(
            (subject.WIDTH - 1) * 4
        )
        _, rounded = subject.host_residual_once(partial, residual, torch)
        self.assertEqual(struct.unpack_from("<H", rounded)[0], 0x3F82)

    def test_actual_replay_retains_cpu_overflow_and_nonfinite_native_partial(self):
        torch = self.torch
        weight = torch.zeros((subject.WIDTH, subject.WIDTH), dtype=torch.bfloat16, device="cpu")
        weight[0, 1] = torch.finfo(torch.bfloat16).max
        weight[1, 0] = 3
        self.assertTrue(bool(torch.isfinite(weight).all()))
        projection = struct.pack("<2H", 0x3F80, 0x4000) + bytes((subject.WIDTH - 2) * 2)
        residual = struct.pack("<2H", 0, 0x3F80) + bytes((subject.WIDTH - 2) * 2)
        partial = struct.pack("<2I", 0x7F800000, 0x40400000) + bytes((subject.WIDTH - 2) * 4)
        summed = struct.pack("<2I", 0x7F800000, 0x40800000) + bytes((subject.WIDTH - 2) * 4)
        projected_bf16 = struct.pack("<2H", 0x7F80, 0x4040) + bytes((subject.WIDTH - 2) * 2)
        hidden = struct.pack("<2H", 0x7F80, 0x4080) + bytes((subject.WIDTH - 2) * 2)
        payloads = {
            "projection-input.bf16": projection,
            "residual-before.bf16": residual,
            "projection-partial.f32le": partial,
            "hidden-after-broadcast.bf16": hidden,
        }
        records, retained = subject.replay_rows(
            payloads, {"rows": [native_row()]}, weight, torch
        )
        self.assertEqual(len(records), 1)
        self.assertEqual(len(retained), 32)
        for case in (
            "torch_bf16_linear", "source_ascending_fp32", "native_partial_host",
            "native_partial_bf16_first",
        ):
            with self.subTest(case=case):
                self.assertTrue(all(records[0]["repeat_agreement"][case].values()))
                for number in (1, 2):
                    prefix = f"position-000004.{case}-pass{number}"
                    self.assertEqual(retained[f"{prefix}-sum_f32.f32"], summed)
                    self.assertEqual(retained[f"{prefix}-hidden_bf16.bf16"], hidden)
                    if case != "native_partial_host":
                        self.assertEqual(retained[f"{prefix}-partial_f32.f32"], partial)
                        self.assertEqual(retained[f"{prefix}-projection_bf16.bf16"], projected_bf16)
        for name in (
            "native_partial_vs_torch_widened", "native_partial_vs_source_ordered",
            "native_hidden_vs_torch", "native_hidden_vs_source_ordered",
            "native_hidden_vs_native_partial_host",
            "native_hidden_vs_native_partial_bf16_first",
            "native_partial_host_vs_bf16_first_hidden",
            "native_partial_bf16_first_vs_torch_hidden",
            "native_projection_bf16_first_vs_torch",
        ):
            for metric in records[0]["comparisons"][name]:
                fp32 = "max_fp32_ulp" in metric
                self.assertEqual(metric["actual_nonfinite_count"], 1)
                expected_key = "expected_nonfinite_count" if fp32 else "reference_nonfinite_count"
                self.assertEqual(metric[expected_key], 1)
                self.assertEqual(metric["bit_mismatches"], 0)
                self.assertIsNone(metric["max_abs" if fp32 else "max_absolute_error"])
                self.assertIsNone(metric["rmse"])
                self.assertIsNone(metric["max_fp32_ulp" if fp32 else "max_bf16_ulp"])
        for key, filename, _, _ in subject.PAYLOADS:
            suffix = "f32" if key == "projection_partial" else "bf16"
            name = f"position-000004.native-{key.replace('_', '-')}.{suffix}"
            self.assertEqual(retained[name], payloads[filename])
        json.dumps(records, allow_nan=False)

    def test_projection_first_isolates_double_rounding_at_a_tie(self):
        partial = struct.pack("<f", 1 + 2**-8) + bytes((subject.WIDTH - 1) * 4)
        residual = struct.pack("<H", 0x3B80) + bytes((subject.WIDTH - 1) * 2)
        _, original = subject.host_residual_once(partial, residual, self.torch)
        rounded = subject.projection_rounded_residual_once(partial, residual, self.torch)
        self.assertEqual(struct.unpack_from("<H", original)[0], 0x3F81)
        self.assertEqual(struct.unpack_from("<H", rounded["projection_bf16"])[0], 0x3F80)
        self.assertEqual(struct.unpack_from("<I", rounded["partial_f32"])[0], 0x3F800000)
        self.assertEqual(struct.unpack_from("<I", rounded["sum_f32"])[0], 0x3F808000)
        self.assertEqual(struct.unpack_from("<H", rounded["hidden_bf16"])[0], 0x3F80)
        self.assertEqual(
            rounded, subject.projection_rounded_residual_once(partial, residual, self.torch)
        )

    def test_projection_first_preserves_representable_agreement_and_zero_behavior(self):
        for partial_bits, residual_bits, hidden_bits in (
            (0x3F800000, 0x3B80, 0x3F80),
            (0x3F800000, 0xBF80, 0),
            (0x80000000, 0x8000, 0),
        ):
            with self.subTest(partial=partial_bits, residual=residual_bits):
                partial = struct.pack("<I", partial_bits) + bytes((subject.WIDTH - 1) * 4)
                residual = struct.pack("<H", residual_bits) + bytes((subject.WIDTH - 1) * 2)
                summed, hidden = subject.host_residual_once(partial, residual, self.torch)
                rounded = subject.projection_rounded_residual_once(partial, residual, self.torch)
                self.assertEqual(rounded["partial_f32"], partial)
                self.assertEqual(struct.unpack_from("<H", rounded["projection_bf16"])[0], partial_bits >> 16)
                self.assertEqual(rounded["sum_f32"], summed)
                self.assertEqual(rounded["hidden_bf16"], hidden)
                self.assertEqual(struct.unpack_from("<H", hidden)[0], hidden_bits)

    def test_projection_first_retains_finite_to_bf16_overflow(self):
        partial = struct.pack("<I", 0x7F7FFFFF) + bytes((subject.WIDTH - 1) * 4)
        residual = struct.pack("<H", 0xFF7F) + bytes((subject.WIDTH - 1) * 2)
        summed, _ = subject.host_residual_once(partial, residual, self.torch)
        self.assertNotEqual(struct.unpack_from("<I", summed)[0] & 0x7F800000, 0x7F800000)
        rounded = subject.projection_rounded_residual_once(partial, residual, self.torch)
        self.assertEqual(struct.unpack_from("<H", rounded["projection_bf16"])[0], 0x7F80)
        self.assertEqual(struct.unpack_from("<H", rounded["hidden_bf16"])[0], 0x7F80)
        metrics = subject.f32_metrics(rounded["sum_f32"], summed)
        self.assertEqual(metrics["actual_nonfinite_count"], 1)
        self.assertEqual(metrics["expected_nonfinite_count"], 0)
        self.assertIsNone(metrics["max_abs"])
        self.assertIsNone(metrics["rmse"])

    def test_projection_first_keeps_nonfinite_projection_categories(self):
        partial = struct.pack("<4I", 0x7F800000, 0xFF800000, 0x7FFFFFFF, 0xFFFFFFFF)
        partial += bytes((subject.WIDTH - 4) * 4)
        rounded = subject.projection_rounded_residual_once(partial, bytes(subject.WIDTH * 2), self.torch)
        projection = struct.unpack_from("<4H", rounded["projection_bf16"])
        self.assertEqual(projection[:2], (0x7F80, 0xFF80))
        for bits in projection[2:]:
            self.assertEqual(bits & 0x7F80, 0x7F80)
            self.assertNotEqual(bits & 0x007F, 0)
        metrics = subject.f32_metrics(rounded["partial_f32"], partial)
        self.assertEqual(metrics["actual_nonfinite_count"], 4)
        self.assertEqual(metrics["expected_nonfinite_count"], 4)
        self.assertIsNone(metrics["max_fp32_ulp"])

    def test_projection_first_rejects_operand_geometry_drift(self):
        for partial, residual in ((b"", bytes(subject.WIDTH * 2)), (bytes(subject.WIDTH * 4), b"")):
            with self.subTest(partial_bytes=len(partial), residual_bytes=len(residual)):
                with self.assertRaisesRegex(subject.Failure, "projection-first residual operands"):
                    subject.projection_rounded_residual_once(partial, residual, self.torch)

    def test_nonfinite_f32_metrics_keep_bits_and_null_finite_metrics(self):
        left = struct.pack("<I", 0x7FC00001) + bytes((subject.WIDTH - 1) * 4)
        right = bytes(subject.WIDTH * 4)
        metrics = subject.f32_metrics(left, right)
        self.assertEqual(metrics["actual_nonfinite_count"], 1)
        self.assertEqual(metrics["bit_mismatches"], 1)
        self.assertIsNone(metrics["max_fp32_ulp"])
        self.assertIsNone(metrics["max_abs"])
        self.assertIsNone(metrics["rmse"])


class RetentionTests(unittest.TestCase):
    def test_both_passes_and_all_native_operands_are_retained(self):
        native = native_payloads()
        passes = replay_passes()
        changed = b"\x01" + passes["source_ascending_fp32"][1]["partial_f32"][1:]
        passes["source_ascending_fp32"][1]["partial_f32"] = changed
        retained = {}
        record = subject.record_row(native_row(), native, passes, retained)
        self.assertFalse(record["repeat_agreement"]["source_ascending_fp32"]["partial_f32"])
        self.assertEqual(
            retained["position-000004.source_ascending_fp32-pass2-partial_f32.f32"],
            changed,
        )
        for stem in (
            "native-projection-input.bf16",
            "native-residual-before.bf16",
            "native-projection-partial.f32",
            "native-hidden-after-broadcast.bf16",
        ):
            self.assertIn(f"position-000004.{stem}", retained)

    def test_nonfinite_native_hidden_is_retained_with_null_metrics(self):
        native = native_payloads()
        native["hidden_after_broadcast"] = b"\xc0\x7f" + native["hidden_after_broadcast"][2:]
        retained = {}
        record = subject.record_row(native_row(), native, replay_passes(), retained)
        metric = record["comparisons"]["native_hidden_vs_source_ordered"][0]
        self.assertEqual(metric["actual_nonfinite_count"], 1)
        self.assertIsNone(metric["max_bf16_ulp"])
        self.assertEqual(
            retained["position-000004.native-hidden-after-broadcast.bf16"],
            native["hidden_after_broadcast"],
        )

    def test_result_keeps_nonclaims_and_no_weight_payload(self):
        value = boundary_fixture()
        retained = {}
        record = subject.record_row(native_row(), native_payloads(), replay_passes(), retained)
        result = subject.result_document(
            value.manifest,
            [record],
            retained,
            {"tensor": subject.WEIGHT, "shard": subject.WEIGHT_SHARD,
             "shape": [subject.WIDTH, subject.WIDTH], "dtype": "bf16-little-endian",
             "data_offsets": subject.WEIGHT_OFFSETS, "tensor_bytes": subject.WEIGHT_BYTES,
             "shard_bytes": subject.core.MODEL_FILES[subject.WEIGHT_SHARD][0],
             "shard_sha256": subject.core.MODEL_FILES[subject.WEIGHT_SHARD][1],
             "emitted_tensor_sha256": "b" * 64, "retained_weight_bytes": False},
            {"implementation": "1" * 64},
            b"pins",
            {"manifest.json": b"manifest"},
            {"stdout.txt": b"stdout"},
            {"python": "3.12"},
            {"path": "/held/qwen.py", "sha256": "2" * 64},
        )
        for field in (
            "qualification", "benchmark_comparable", "numerical_pass_claimed",
            "tolerance_reviewed", "cause_established", "device_behavior_established",
            "compiler_behavior_established", "full_model_execution", "gpu_execution",
        ):
            self.assertIs(result[field], False)
        self.assertFalse(any("weight" in name for name in result["payloads"]))
        json.dumps(result, allow_nan=False)


class FileAndRunTests(unittest.TestCase):
    def run_fixture(self, root, *, existing=False, capture_drift=False, source_drift=False,
                    implementation_drift=False, replace_output_parent=False,
                    tensor_digest_mismatch=False):
        value = boundary_fixture()
        base = Path(root)
        for name, data in (("capture", value.data), ("witness", value.witness)):
            directory = base / name
            directory.mkdir()
            for filename, raw in data.items():
                (directory / filename).write_bytes(raw)
        pin_bytes = encoded(value.pins)
        (base / "pins").mkdir()
        (base / "pins" / "pins.json").write_bytes(pin_bytes)
        (base / "model").mkdir()
        for component in ("draft", "target"):
            (base / "model" / component).mkdir()
        publish = base / "publish"
        publish.mkdir()
        output = publish / "output"
        if existing:
            output.mkdir()
            (output / "sentinel").write_bytes(b"preserve")

        target_stat = (base / "model" / "target").stat()
        validations = []

        def validate_source():
            validations.append(True)
            if source_drift:
                raise subject.Failure("model changed during replay")

        model_source = SimpleNamespace(
            validate=validate_source,
            target=SimpleNamespace(
                identity=subject.core.FileIdentity(target_stat.st_dev, target_stat.st_ino)
            ),
        )
        torch = SimpleNamespace(get_num_threads=lambda: 1, get_num_interop_threads=lambda: 1)
        qwen = object()
        qwen_identity = {"path": "/held/qwen.py", "sha256": "2" * 64}
        retained = {"position-000004.native-projection-input.bf16": bytes(subject.WIDTH * 2)}
        records = [{"position": 4, "repeat_agreement": {
            "torch_bf16_linear": {"projection_bf16": True},
            "source_ascending_fp32": {"partial_f32": True},
            "native_partial_host": {"hidden_bf16": True},
            "native_partial_bf16_first": {"hidden_bf16": True},
        }, "comparisons": {}}]
        descriptor = {
            "tensor": subject.WEIGHT, "shard": subject.WEIGHT_SHARD,
            "shape": [subject.WIDTH, subject.WIDTH], "dtype": "bf16-little-endian",
            "data_offsets": subject.WEIGHT_OFFSETS, "tensor_bytes": subject.WEIGHT_BYTES,
            "shard_bytes": subject.core.MODEL_FILES[subject.WEIGHT_SHARD][0],
            "shard_sha256": subject.core.MODEL_FILES[subject.WEIGHT_SHARD][1],
            "emitted_tensor_sha256": "b" * 64, "retained_weight_bytes": False,
        }
        if tensor_digest_mismatch:
            descriptor["emitted_tensor_sha256"] = "c" * 64

        def replay(*_):
            if capture_drift:
                path = base / "capture" / "projection-input.bf16"
                changed = bytearray(path.read_bytes())
                changed[0] = 1
                path.write_bytes(changed)
            if replace_output_parent:
                moved = output.parent.with_name("publish-held")
                output.parent.rename(moved)
                output.parent.mkdir()
            return records, retained

        source_hashes = {"implementation": "1" * 64}
        observations = []

        def observe_implementation():
            if not implementation_drift:
                return source_hashes
            observations.append(True)
            return source_hashes if len(observations) == 1 else {"implementation": "0" * 64}

        arguments = [
            str(base / "capture"), str(base / "witness"),
            str(base / "pins" / "pins.json"), subject.digest(pin_bytes),
            str(base / "model"), str(output),
        ]
        with patch.object(subject.core, "require_isolated_python"), \
                patch.object(subject.core, "require_virtual_environment"), \
                patch.object(subject, "implementation", side_effect=observe_implementation), \
                patch.object(subject.core, "authenticate_model_source",
                             return_value=nullcontext(model_source)), \
                patch.object(subject.ablation, "load_cpu_dependencies",
                             return_value=(torch, object(), qwen, {"python": "3.12"}, qwen_identity)), \
                patch.object(subject, "load_projection_weight", return_value=(object(), descriptor)), \
                patch.object(subject, "replay_rows", side_effect=replay) as replay_mock, \
                patch.object(subject.ablation, "qwen_source_identity", return_value=qwen_identity):
            if tensor_digest_mismatch:
                with self.assertRaisesRegex(
                    subject.Failure,
                    "^authenticated o_proj tensor does not match captured tensor identity$",
                ):
                    subject.run(arguments)
                replay_mock.assert_not_called()
            elif existing or capture_drift or source_drift or implementation_drift or replace_output_parent:
                with self.assertRaises((subject.Failure, FileExistsError)):
                    subject.run(arguments)
            else:
                subject.run(arguments)
        if existing:
            self.assertEqual((output / "sentinel").read_bytes(), b"preserve")
        elif (capture_drift or source_drift or implementation_drift
              or replace_output_parent or tensor_digest_mismatch):
            self.assertFalse(output.exists())
        else:
            self.assertEqual(set(path.name for path in output.iterdir()), set(retained) | {"diagnostic.json"})
            document = json.loads((output / "diagnostic.json").read_bytes())
            self.assertFalse(document["qualification"])

    def test_success_and_existing_output(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root)
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, existing=True)

    def test_input_and_implementation_drift_refuse_publication(self):
        for option in ("capture_drift", "source_drift", "implementation_drift", "replace_output_parent"):
            with tempfile.TemporaryDirectory() as root, self.subTest(option=option):
                self.run_fixture(root, **{option: True})

    def test_captured_tensor_digest_mismatch_refuses_replay_and_publication(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, tensor_digest_mismatch=True)

    def test_usage_is_closed(self):
        for count in (0, 1, 5, 7):
            with self.subTest(count=count), self.assertRaises(subject.Failure):
                subject.run(["unused"] * count)


if __name__ == "__main__":
    unittest.main()
