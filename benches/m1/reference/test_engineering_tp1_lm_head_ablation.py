#!/usr/bin/env python3
"""Focused CPU-only tests for the same-input TP1 LM-head diagnostic."""

from contextlib import nullcontext
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
from types import SimpleNamespace
from unittest.mock import patch
import sys
import unittest


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
    "tp1_lm_head_ablation", "engineering_tp1_lm_head_ablation.py"
)
fixtures = load_sibling(
    "tp1_lm_head_secure_fixtures", "test_engineering_tp1_rmsnorm_ablation.py"
)


def encoded(value) -> bytes:
    return json.dumps(value, indent=2, allow_nan=False).encode()


def bf16_row(bits: int, width: int = subject.VOCABULARY) -> bytes:
    return bits.to_bytes(2, "little") * width


def replace_bits(data: bytes, index: int, bits: int) -> bytes:
    value = bytearray(data)
    value[index * 2:index * 2 + 2] = bits.to_bytes(2, "little")
    return bytes(value)


def row(position=4, logits=None):
    return {
        "ordinal": 0,
        "position": position,
        "input_token": 374,
        "gpu_choice": 13,
        "normalized": bf16_row(0, subject.WIDTH),
        "logits": bf16_row(0) if logits is None else logits,
    }


def passes(torch_values=None, source_values=None):
    torch_values = torch_values or [bf16_row(0), bf16_row(0)]
    source_values = source_values or [
        (bytes(subject.VOCABULARY * 4), bf16_row(0)),
        (bytes(subject.VOCABULARY * 4), bf16_row(0)),
    ]
    return {
        "torch_bf16_linear": torch_values,
        "source_ascending_fp32": source_values,
    }


class BindingTests(unittest.TestCase):
    def test_weight_binding_is_exact_and_untied(self):
        subject.parse_weight_binding(
            encoded({"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD}}),
            encoded({"tie_word_embeddings": False}),
        )
        bad = (
            ({"weight_map": {subject.WEIGHT: "wrong"}}, {"tie_word_embeddings": False}),
            ({"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD,
                              "lm_head.bias": subject.WEIGHT_SHARD}},
             {"tie_word_embeddings": False}),
            ({"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD}},
             {"tie_word_embeddings": True}),
        )
        for index, config in bad:
            with self.subTest(index=index, config=config), self.assertRaises(subject.Failure):
                subject.parse_weight_binding(encoded(index), encoded(config))

    def test_load_weight_retains_only_enclosing_shard_descriptor(self):
        bf16_dtype = object()
        validated = []

        class Tensor:
            shape = (subject.VOCABULARY, subject.WIDTH)
            device = SimpleNamespace(type="cpu")
            dtype = bf16_dtype

            def clone(self):
                return self

            def is_contiguous(self):
                return True

            def stride(self):
                return (subject.WIDTH, 1)

            def numel(self):
                return subject.VOCABULARY * subject.WIDTH

            def element_size(self):
                return 2

        tensor = Tensor()

        class Slice:
            @staticmethod
            def get_shape():
                return [subject.VOCABULARY, subject.WIDTH]

        class Shard:
            def __enter__(self):
                return self

            def __exit__(self, *_):
                return None

            @staticmethod
            def get_slice(name):
                self.assertEqual(name, subject.WEIGHT)
                return Slice()

            @staticmethod
            def get_tensor(name):
                self.assertEqual(name, subject.WEIGHT)
                return tensor

        files = {
            subject.INDEX: SimpleNamespace(
                read=lambda **_: encoded(
                    {"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD}}
                )
            ),
            subject.CONFIG: SimpleNamespace(
                read=lambda **_: encoded({"tie_word_embeddings": False})
            ),
            subject.WEIGHT_SHARD: SimpleNamespace(fd=91),
        }
        source = SimpleNamespace(files=files, validate=lambda: validated.append(True))
        torch = SimpleNamespace(
            bfloat16=bf16_dtype,
            isfinite=lambda _: SimpleNamespace(all=lambda: True),
        )
        safetensors = SimpleNamespace(safe_open=lambda *_, **__: Shard())
        actual, descriptor = subject.load_head_weight(source, safetensors, torch)
        self.assertIs(actual, tensor)
        self.assertEqual(validated, [True])
        self.assertEqual(descriptor["tensor"], subject.WEIGHT)
        self.assertEqual(descriptor["data_offsets"], [0, subject.WEIGHT_BYTES])
        self.assertEqual(descriptor["shard_sha256"],
                         subject.core.MODEL_FILES[subject.WEIGHT_SHARD][1])
        self.assertNotIn("weight_bytes", descriptor)

    def test_native_rows_use_held_normalized_and_logits_bytes(self):
        normalized = bytes(range(16)) * (subject.WIDTH * 4 // 16)
        logits = bytes(range(16)) * (subject.VOCABULARY * 4 // 16)
        rows = subject.native_rows(
            {"normalized.bf16": normalized, "logits.bf16": logits},
            {
                "positions": [4, 5],
                "input_tokens": [1, 2, 3, 4, 5, 6],
                "gpu_choices": [7, 8, 9, 10, 11, 12],
            },
        )
        self.assertEqual([value["position"] for value in rows], [4, 5])
        self.assertEqual(rows[0]["normalized"], normalized[:subject.WIDTH * 2])
        self.assertEqual(rows[1]["logits"], logits[subject.VOCABULARY * 2:])


class PinnedTorchArithmeticTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.torch, _, _, _, _ = subject.ablation.load_cpu_dependencies()

    def test_actual_torch_bf16_linear_is_repeatable(self):
        torch = self.torch
        normalized = torch.tensor([[1, 2, 3, 4]], dtype=torch.bfloat16, device="cpu")
        weight = torch.tensor(
            [[1, 2, 3, 4], [2, 0, -2, 1]],
            dtype=torch.bfloat16,
            device="cpu",
        )
        first = subject.torch_linear_once(normalized, weight, torch)
        second = subject.torch_linear_once(normalized, weight, torch)
        expected = subject.tensor_bytes(
            torch.tensor([[30, 0]], dtype=torch.bfloat16, device="cpu"), torch
        )
        self.assertEqual(first, expected)
        self.assertEqual(second, expected)

    def test_actual_torch_ascending_fp32_recurrence(self):
        torch = self.torch
        normalized = torch.tensor([[1, 1, 1, 1]], dtype=torch.bfloat16, device="cpu")
        by_inner = torch.tensor(
            [[2**24, 1], [1, 2], [-(2**24), 3], [1, 4]],
            dtype=torch.float32,
            device="cpu",
        )
        fp32, rounded = subject.ascending_fp32_once(normalized, by_inner, torch)
        self.assertEqual(struct.unpack("<2f", fp32), (1.0, 10.0))
        self.assertEqual(
            rounded,
            subject.tensor_bytes(
                torch.tensor([1, 10], dtype=torch.bfloat16, device="cpu"), torch
            ),
        )

    def test_nonfinite_replay_input_is_rejected(self):
        torch = self.torch
        normalized = torch.tensor(
            [[float("nan"), 1]], dtype=torch.bfloat16, device="cpu"
        )
        weight = torch.ones((2, 2), dtype=torch.bfloat16, device="cpu")
        with self.assertRaises(subject.Failure):
            subject.torch_linear_once(normalized, weight, torch)

    def test_finite_inputs_can_overflow_and_raw_outputs_are_retained(self):
        torch = self.torch
        maximum = torch.finfo(torch.bfloat16).max
        normalized = torch.full(
            (1, 2), maximum, dtype=torch.bfloat16, device="cpu"
        )
        weight = torch.full(
            (subject.VOCABULARY, 2),
            maximum,
            dtype=torch.bfloat16,
            device="cpu",
        )
        by_inner = torch.full(
            (2, subject.VOCABULARY),
            maximum,
            dtype=torch.float32,
            device="cpu",
        )
        torch_raw = subject.torch_linear_once(normalized, weight, torch)
        source_fp32, source_bf16 = subject.ascending_fp32_once(
            normalized, by_inner, torch
        )
        self.assertFalse(
            bool(
                torch.isfinite(
                    torch.frombuffer(bytearray(torch_raw), dtype=torch.bfloat16)
                ).all()
            )
        )
        self.assertFalse(
            bool(
                torch.isfinite(
                    torch.frombuffer(bytearray(source_fp32), dtype=torch.float32)
                ).all()
            )
        )
        retained = {}
        record = subject.record_row(
            row(),
            {
                "torch_bf16_linear": [torch_raw, torch_raw],
                "source_ascending_fp32": [
                    (source_fp32, source_bf16),
                    (source_fp32, source_bf16),
                ],
            },
            retained,
        )
        for comparison in (
            "native_vs_torch_pass1",
            "native_vs_source_pass1",
            "torch_pass1_vs_source_pass1",
        ):
            metric = record["comparisons"][comparison]
            self.assertFalse(metric["finite_values"])
            self.assertIsNone(metric["max_bf16_ulp"])
            self.assertIsNone(metric["max_absolute_error"])
            self.assertIsNone(metric["rmse"])
        self.assertEqual(
            retained["position-000004.torch-pass1-logits.bf16"], torch_raw
        )
        self.assertEqual(
            retained["position-000004.torch-pass2-logits.bf16"], torch_raw
        )
        self.assertEqual(
            retained["position-000004.source-pass1-logits.bf16"], source_bf16
        )
        self.assertEqual(
            retained["position-000004.source-pass2-logits.f32"], source_fp32
        )


class RetentionTests(unittest.TestCase):
    def test_record_retains_both_disagreeing_passes(self):
        torch1 = bf16_row(0)
        torch2 = replace_bits(torch1, 7, 0x3F80)
        source1 = (bytes(subject.VOCABULARY * 4), bf16_row(0))
        source2 = (b"\x01" + bytes(subject.VOCABULARY * 4 - 1), bf16_row(0))
        retained = {}
        record = subject.record_row(
            row(), passes([torch1, torch2], [source1, source2]), retained
        )
        self.assertFalse(record["repeat_agreement"]["torch_bf16_logits_byte_identical"])
        self.assertFalse(
            record["repeat_agreement"]["source_fp32_accumulators_byte_identical"]
        )
        self.assertEqual(
            retained["position-000004.torch-pass2-logits.bf16"], torch2
        )
        self.assertEqual(
            retained["position-000004.source-pass2-logits.f32"], source2[0]
        )

    def test_nonfinite_native_logits_are_retained_with_null_finite_metrics(self):
        native = replace_bits(bf16_row(0), 3, 0x7FC0)
        retained = {}
        record = subject.record_row(row(logits=native), passes(), retained)
        metric = record["comparisons"]["native_vs_torch_pass1"]
        self.assertFalse(metric["finite_values"])
        self.assertEqual(metric["actual_nonfinite_count"], 1)
        self.assertIsNone(metric["max_bf16_ulp"])
        self.assertIsNone(metric["max_absolute_error"])
        self.assertIsNone(metric["rmse"])
        self.assertEqual(
            retained["position-000004.native-logits.bf16"], native
        )

    def test_replay_invokes_each_case_twice_per_held_row(self):
        normalized = bytes(2 * subject.WIDTH * 2)
        logits = bytes(2 * subject.VOCABULARY * 2)
        manifest = {
            "positions": [4, 5],
            "input_tokens": [0] * 6,
            "gpu_choices": [0] * 6,
        }
        torch_outputs = [bf16_row(value) for value in (0, 0, 0, 0)]
        source_outputs = [
            (bytes(subject.VOCABULARY * 4), bf16_row(0)) for _ in range(4)
        ]
        with patch.object(subject, "prepare_source_weights", return_value=object()), \
                patch.object(subject, "bf16_row", return_value=object()), \
                patch.object(subject, "require_finite"), \
                patch.object(subject, "torch_linear_once", side_effect=torch_outputs) as linear, \
                patch.object(subject, "ascending_fp32_once", side_effect=source_outputs) as serial:
            records, retained = subject.replay_rows(
                {"normalized.bf16": normalized, "logits.bf16": logits},
                manifest,
                object(),
                object(),
            )
        self.assertEqual(len(records), 2)
        self.assertEqual(linear.call_count, 4)
        self.assertEqual(serial.call_count, 4)
        self.assertIn("position-000005.source-pass2-logits.f32", retained)

    def test_result_has_no_weight_payload_or_acceptance_claim(self):
        value = fixtures.native_fixture()
        retained = {}
        record = subject.record_row(row(), passes(), retained)
        result = subject.result_document(
            value.manifest,
            [record],
            retained,
            {
                "tensor": subject.WEIGHT,
                "shard": subject.WEIGHT_SHARD,
                "shape": [subject.VOCABULARY, subject.WIDTH],
                "dtype": "bf16-little-endian",
                "data_offsets": subject.WEIGHT_OFFSETS,
                "tensor_bytes": subject.WEIGHT_BYTES,
                "shard_bytes": subject.core.MODEL_FILES[subject.WEIGHT_SHARD][0],
                "shard_sha256": subject.core.MODEL_FILES[subject.WEIGHT_SHARD][1],
            },
            {"implementation": "1" * 64},
            b"pins",
            {"manifest.json": b"manifest"},
            {"stdout.txt": b"stdout"},
            {"python": "3.12"},
            {"path": "/held/qwen.py", "sha256": "2" * 64},
        )
        for field in (
            "qualification",
            "benchmark_comparable",
            "numerical_pass_claimed",
            "tolerance_reviewed",
            "cause_established",
            "device_behavior_established",
            "compiler_behavior_established",
            "full_model_execution",
            "gpu_execution",
        ):
            self.assertIs(result[field], False)
        self.assertFalse(any("weight" in name for name in result["payloads"]))
        self.assertEqual(result["weight"]["shard_sha256"],
                         subject.core.MODEL_FILES[subject.WEIGHT_SHARD][1])
        self.assertIn("does not establish", result["nonclaim"])
        json.dumps(result, allow_nan=False)

    def test_usage_is_closed(self):
        for count in (0, 1, 5, 7):
            with self.subTest(count=count), self.assertRaises(subject.Failure):
                subject.run(["unused"] * count)


class FileAndRunTests(unittest.TestCase):
    def run_fixture(
        self,
        root,
        *,
        existing_output=False,
        drift=None,
        alias=None,
        implementation_drift=False,
        replace_output_parent=False,
    ):
        value = fixtures.native_fixture()
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
        if alias == "capture":
            output = base / "capture" / "output"
        elif alias == "target":
            output = base / "model" / "target" / "output"
        if existing_output:
            output.mkdir()
            (output / "sentinel").write_bytes(b"preserve")

        validations = []

        def validate_source():
            validations.append(True)
            if drift == "source":
                raise subject.Failure("model changed during replay")

        target_stat = (base / "model" / "target").stat()
        model_source = SimpleNamespace(
            validate=validate_source,
            target=SimpleNamespace(
                identity=subject.core.FileIdentity(target_stat.st_dev, target_stat.st_ino)
            ),
        )
        torch = SimpleNamespace(
            get_num_threads=lambda: 1,
            get_num_interop_threads=lambda: 1,
        )
        qwen = object()
        qwen_identity = {"path": "/held/qwen.py", "sha256": "2" * 64}
        retained = {
            "position-000004.native-normalized.bf16": bytes(subject.WIDTH * 2),
            "position-000004.native-logits.bf16": bytes(subject.VOCABULARY * 2),
        }
        records = [{
            "position": 4,
            "repeat_agreement": {
                "torch_bf16_logits_byte_identical": False,
                "source_bf16_logits_byte_identical": True,
                "source_fp32_accumulators_byte_identical": True,
            },
            "comparisons": {},
        }]
        descriptor = {
            "tensor": subject.WEIGHT,
            "shard": subject.WEIGHT_SHARD,
            "shape": [subject.VOCABULARY, subject.WIDTH],
            "dtype": "bf16-little-endian",
            "data_offsets": subject.WEIGHT_OFFSETS,
            "tensor_bytes": subject.WEIGHT_BYTES,
            "shard_bytes": subject.core.MODEL_FILES[subject.WEIGHT_SHARD][0],
            "shard_sha256": subject.core.MODEL_FILES[subject.WEIGHT_SHARD][1],
        }

        def replay(*_):
            if drift == "capture":
                path = base / "capture" / "normalized.bf16"
                changed = bytearray(path.read_bytes())
                changed[0:2] = b"\x80\x3f"
                path.write_bytes(changed)
            if replace_output_parent:
                moved = output.parent.with_name("publish-held")
                output.parent.rename(moved)
                output.parent.mkdir()
            return records, retained

        source_hashes = {"implementation": "1" * 64}
        real_implementation = subject.implementation
        implementation_observations = []

        def observe_implementation():
            if not implementation_drift:
                return source_hashes
            observed = real_implementation()
            implementation_observations.append(observed)
            if len(implementation_observations) == 2:
                observed = dict(observed)
                observed[subject.GEMM_SOURCE] = "0" * 64
            return observed

        arguments = [
            str(base / "capture"),
            str(base / "witness"),
            str(base / "pins" / "pins.json"),
            subject.digest(pin_bytes),
            str(base / "model"),
            str(output),
        ]
        with patch.object(subject.core, "require_isolated_python"), \
                patch.object(subject.core, "require_virtual_environment"), \
                patch.object(subject, "implementation", side_effect=observe_implementation), \
                patch.object(
                    subject.core,
                    "authenticate_model_source",
                    return_value=nullcontext(model_source),
                ), \
                patch.object(
                    subject.ablation,
                    "load_cpu_dependencies",
                    return_value=(torch, object(), qwen, {"python": "3.12"}, qwen_identity),
                ), \
                patch.object(subject, "load_head_weight", return_value=(object(), descriptor)), \
                patch.object(subject, "replay_rows", side_effect=replay), \
                patch.object(subject.ablation, "qwen_source_identity", return_value=qwen_identity):
            if (existing_output or drift is not None or alias is not None
                    or implementation_drift or replace_output_parent):
                with self.assertRaises((subject.Failure, FileExistsError)):
                    subject.run(arguments)
            else:
                subject.run(arguments)

        if existing_output:
            self.assertEqual((output / "sentinel").read_bytes(), b"preserve")
            self.assertEqual(set(path.name for path in output.iterdir()), {"sentinel"})
        elif (drift is not None or alias is not None or implementation_drift
              or replace_output_parent):
            self.assertFalse(output.exists())
            if replace_output_parent:
                self.assertFalse((base / "publish-held" / "output").exists())
        else:
            self.assertEqual(
                set(path.name for path in output.iterdir()),
                set(retained) | {"diagnostic.json"},
            )
            for name, raw in retained.items():
                self.assertEqual((output / name).read_bytes(), raw)
                self.assertEqual((output / name).stat().st_mode & 0o777, 0o600)
            document = json.loads((output / "diagnostic.json").read_bytes())
            self.assertFalse(document["numerical_pass_claimed"])
            self.assertFalse(document["repeat_agreement_all"])
            self.assertEqual((output / "diagnostic.json").stat().st_mode & 0o777, 0o600)
            self.assertEqual(output.stat().st_mode & 0o777, 0o700)
        if alias is None:
            self.assertEqual(validations, [True])
        else:
            self.assertEqual(validations, [])
        if implementation_drift:
            self.assertEqual(len(implementation_observations), 2)
            self.assertEqual(
                implementation_observations[0][subject.GEMM_SOURCE],
                subject.GEMM_SOURCE_SHA256,
            )

    def test_valid_run_publishes_exact_new_output_including_disagreement(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root)

    def test_existing_output_is_preserved(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, existing_output=True)

    def test_capture_or_source_drift_refuses_publication(self):
        for drift in ("capture", "source"):
            with self.subTest(drift=drift), tempfile.TemporaryDirectory() as root:
                self.run_fixture(root, drift=drift)

    def test_output_parent_must_not_alias_inputs(self):
        for alias in ("capture", "target"):
            with self.subTest(alias=alias), tempfile.TemporaryDirectory() as root:
                self.run_fixture(root, alias=alias)

    def test_actual_implementation_hash_drift_refuses_publication(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, implementation_drift=True)

    def test_output_parent_replacement_refuses_publication(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, replace_output_parent=True)


if __name__ == "__main__":
    unittest.main()
