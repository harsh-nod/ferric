#!/usr/bin/env python3
"""Focused CPU-only tests for the same-input TP1 final-RMS diagnostic."""

from contextlib import nullcontext
import importlib.util
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
from unittest.mock import patch
import sys
import unittest


specification = importlib.util.spec_from_file_location(
    "tp1_rmsnorm_ablation", Path(__file__).with_name("engineering_tp1_rmsnorm_ablation.py")
)
if specification is None or specification.loader is None:
    raise ImportError("cannot load TP1 RMSNorm ablation")
subject = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = subject
specification.loader.exec_module(subject)


def row_bytes(bits: int = 0) -> bytes:
    return bits.to_bytes(2, "little") * subject.WIDTH


def replace_bits(data: bytes, index: int, bits: int) -> bytes:
    value = bytearray(data)
    value[index * 2:index * 2 + 2] = bits.to_bytes(2, "little")
    return bytes(value)


def arithmetic_payload(
    residual: bytes, weight: bytes, hf: bytes, source: bytes
) -> dict[str, bytes]:
    payloads = {"input.bf16": residual, "weight.bf16": weight}
    for name in subject.SELECTED_ABLATION_PAYLOADS:
        if name == "hf.output.bf16":
            payloads[name] = hf
        elif name == f"{subject.SOURCE_CASE}.output.bf16":
            payloads[name] = source
        else:
            payloads[name] = bytes(subject.WIDTH * (4 if name.endswith(".f32") else 2))
    return payloads


def encoded(value) -> bytes:
    return json.dumps(value, indent=2, allow_nan=False).encode()


def native_fixture():
    reference = subject.reference
    selected = [4, 5]
    prompt = [785, 6722, 315, 9625, 374]
    length = 6
    choices = [0] * length
    pins = {name: "1" * 64 for name in reference.IDENTITIES}
    pins.update(
        schema="FerricTpFinalStageNativePinsV1",
        authority="none",
        qualification=False,
        benchmark_comparable=False,
        model_bundle_id=subject.core.PINNED_MODEL_IDENTITY,
        device_unique_ids=[16366993098680759275],
        prompt="The capital of France is",
        prompt_tokens=prompt,
        new_tokens=2,
        capacity=128,
        repetitions=1,
        warmup_runs=0,
        positions=selected,
    )
    setup = {
        name: value for name, value in pins.items()
        if name in reference.PIN_FIELDS - {"positions"}
    }
    setup.update(
        schema="FerricQwen3TpEngineeringSetupV1",
        authority="none",
        model=subject.core.PINNED_REPOSITORY,
        dtype="BF16",
        target="gfx950:xnack-",
        tensor_parallel=1,
        worker_pids=[123],
        running_worker_sha256=[pins["worker_sha256"]],
        executable_identity="live_proc_exe_sha256",
        collective="host_staged_fp32_rank_order_reduce_bf16_residual",
        prefill="token_at_a_time_m1",
        decoding="greedy_lowest_id_fixed_length",
        rank_zero_dispatch_budget=length * 544,
        conservative_ring_packet_limit=131072,
        model_intake_seconds=1.25,
        setup_seconds=2.5,
        numerical_status="Contracted; compare emitted token IDs independently",
        timing="monotonic controller clock; includes IPC, host collectives and "
               "per-token progress logging; excludes setup",
        final_stage_capture={
            "positions": selected,
            "benchmark_comparable": False,
            "timing": "diagnostic readbacks invalidate all performance measurements",
        },
    )
    closed = {
        "schema": "FerricQwen3TpEngineeringClosedV1",
        "authority": "none",
        "worker_pids": [123],
        "all_workers_exited": True,
        "whole_seconds": 4.5,
    }
    manifest = {
        "schema": "FerricTpFinalStageCaptureV1",
        "authority": "none",
        "complete": True,
        "worker_close_confirmed": True,
        "qualification": False,
        "benchmark_comparable": False,
        "numerical_pass_claimed": False,
        "setup": setup,
        "closed": closed,
        "positions": selected,
        "epoch": 1,
        "input_tokens": prompt + choices[len(prompt) - 1:-1],
        "gpu_choices": choices,
        "rows": [],
        "payloads": [],
        "maximum_total_bytes": reference.MAX_BYTES,
        "nonclaim": "observed bytes only",
    }
    data = {
        f"{name}.bf16": bytes(len(selected) * width * 2)
        for name, width in reference.WIDTHS.items()
    }
    intent = {
        "schema": "FerricTpFinalStageIntentV1",
        "authority": "none",
        "complete": False,
        "setup": setup,
        "positions": selected,
        "required_steps": length,
        "benchmark_comparable": False,
        "qualification": False,
    }
    measurement = {
        "schema": "FerricQwen3TpEngineeringMeasurementV1",
        "authority": "none",
        "run": 0,
        "warmup": False,
        "world_size": 1,
        "prompt_tokens": prompt,
        "generated_tokens": choices[len(prompt) - 1:],
        "generated_text": "",
        "generated_utf8_bytes": [],
        "ttft_seconds": 1.0,
        "tpot_seconds": 0.5,
        "decode_intervals_seconds": [0.5],
        "generation_seconds": 2.0,
        "rank_dispatch_counts": [length * 544],
        "kv_tokens_processed": length,
        "benchmark_comparable": False,
    }
    witness = {
        name: b"0\n" for name in ("process.exit", "wrapper.exit", "group-probe.exit")
    }
    witness["group-after.txt"] = b"leader and group absent\n"
    value = SimpleNamespace(
        pins=pins,
        manifest=manifest,
        data=data,
        intent=intent,
        measurement=measurement,
        witness=witness,
    )
    refresh_native(value)
    return value


def refresh_native(value) -> None:
    reference = subject.reference
    manifest = value.manifest
    manifest["payloads"] = []
    manifest["rows"] = [
        {
            "position": position,
            "input_token": manifest["input_tokens"][position],
            "gpu_choice": manifest["gpu_choices"][position],
        }
        for position in manifest["positions"]
    ]
    for name, width in reference.WIDTHS.items():
        filename = f"{name}.bf16"
        raw = value.data[filename]
        manifest["payloads"].append({
            "file": filename,
            "shape": [len(manifest["positions"]), width],
            "dtype": "bf16-little-endian",
            "bytes": len(raw),
            "sha256": subject.digest(raw),
        })
        for index, row in enumerate(manifest["rows"]):
            offset = index * width * 2
            part = raw[offset:offset + width * 2]
            row[name] = {
                "file": filename,
                "offset_bytes": offset,
                "bytes": len(part),
                "sha256": subject.digest(part),
            }
            if name == "logits":
                row["logits_diagnostics"] = reference.logits_diagnostics(
                    part, row["gpu_choice"]
                )
    value.data["intent.json"] = encoded(value.intent)
    value.data["manifest.json"] = encoded(manifest)
    receipt = {
        "schema": "FerricTpFinalStageCaptureReceiptV1",
        "authority": "none",
        "qualification": False,
        "benchmark_comparable": False,
        "manifest_sha256": subject.digest(value.data["manifest.json"]),
        "manifest_bytes": len(value.data["manifest.json"]),
    }
    value.witness["stdout.txt"] = b"".join(
        json.dumps(part).encode() + b"\n"
        for part in (manifest["setup"], value.measurement, manifest["closed"], receipt)
    )


class BindingTests(unittest.TestCase):
    def test_exact_final_weight_binding_and_epsilon(self):
        index = json.dumps({"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD}}).encode()
        config = json.dumps({"rms_norm_eps": subject.EPSILON}).encode()
        subject.parse_weight_binding(index, config)
        failures = (
            (json.dumps({"weight_map": {subject.WEIGHT: "wrong.safetensors"}}).encode(), config),
            (json.dumps({"weight_map": {}}).encode(), config),
            (b'{"weight_map":{},"weight_map":{}}', config),
            (index, json.dumps({"rms_norm_eps": 1e-5}).encode()),
            (index, json.dumps({"rms_norm_eps": True}).encode()),
            (index, b'{"rms_norm_eps":1e-6,"rms_norm_eps":1e-6}'),
        )
        for ordinal, values in enumerate(failures):
            with self.subTest(ordinal=ordinal), self.assertRaises((subject.Failure, ValueError)):
                subject.parse_weight_binding(*values)

    def test_final_weight_comes_from_authenticated_held_shard(self):
        index = json.dumps({"weight_map": {subject.WEIGHT: subject.WEIGHT_SHARD}}).encode()
        config = json.dumps({"rms_norm_eps": subject.EPSILON}).encode()
        reads = []

        class File:
            def __init__(self, data=b"", fd=-1):
                self.data, self.fd = data, fd
            def read(self, *, exact):
                reads.append(exact)
                return self.data

        class Tensor:
            shape, dtype = (subject.WIDTH,), "bf16"
            def clone(self):
                return self

        class Slice:
            def get_shape(self):
                return [subject.WIDTH]

        class Shard:
            def __enter__(self):
                return self
            def __exit__(self, *_):
                return None
            def get_slice(self, name):
                self.test.assertEqual(name, subject.WEIGHT)
                return Slice()
            def get_tensor(self, name):
                self.test.assertEqual(name, subject.WEIGHT)
                return Tensor()

        class Safetensors:
            def safe_open(inner, path, *, framework, device):
                self.assertEqual((path, framework, device), ("/proc/self/fd/47", "pt", "cpu"))
                shard = Shard()
                shard.test = self
                return shard

        validations = []
        source = type("Source", (), {})()
        source.files = {
            subject.INDEX: File(index),
            "config.json": File(config),
            subject.WEIGHT_SHARD: File(fd=47),
        }
        source.validate = lambda: validations.append(True)
        raw = row_bytes(0x3F80)
        torch = type("Torch", (), {"bfloat16": "bf16"})()
        with patch.object(subject.ablation, "tensor_bytes", return_value=raw):
            weight, actual = subject.load_final_weight(source, Safetensors(), torch)
        self.assertIsInstance(weight, Tensor)
        self.assertEqual(actual, raw)
        self.assertEqual(reads, [subject.core.MODEL_FILES[subject.INDEX][0],
                                 subject.core.MODEL_FILES["config.json"][0]])
        self.assertEqual(validations, [True])

    def test_native_rows_keep_exact_selected_bytes(self):
        first_residual = row_bytes(0x3F80)
        second_residual = row_bytes(0xBF80)
        first_normalized = row_bytes(0x3F00)
        second_normalized = row_bytes(0xBF00)
        payloads = {
            "residual.bf16": first_residual + second_residual,
            "normalized.bf16": first_normalized + second_normalized,
        }
        manifest = {
            "positions": [4, 5],
            "input_tokens": [1, 2, 3, 4, 5, 6],
            "gpu_choices": [7, 8, 9, 10, 11, 12],
        }
        rows = subject.native_rows(payloads, manifest)
        self.assertEqual([(row["position"], row["input_token"], row["gpu_choice"])
                          for row in rows], [(4, 5, 11), (5, 6, 12)])
        self.assertEqual(rows[0]["residual"], first_residual)
        self.assertEqual(rows[1]["normalized"], second_normalized)
        for name in ("residual.bf16", "normalized.bf16"):
            with self.subTest(name=name), self.assertRaises(subject.Failure):
                changed = dict(payloads)
                changed[name] = changed[name][:-2]
                subject.native_rows(changed, manifest)

    def test_bf16_row_round_trips_exact_native_bits(self):
        raw = row_bytes(0xBF80)
        calls = []

        class Tensor:
            def reshape(self, *shape):
                calls.append(("reshape", shape))
                return self
            def clone(self):
                calls.append(("clone",))
                return self

        class Torch:
            bfloat16 = "bf16"
            @staticmethod
            def frombuffer(buffer, *, dtype):
                calls.append(("frombuffer", bytes(buffer), dtype, type(buffer)))
                return Tensor()

        with patch.object(subject.ablation, "tensor_bytes", return_value=raw):
            subject.bf16_row(raw, Torch)
        self.assertEqual(calls[0], ("frombuffer", raw, "bf16", bytearray))
        self.assertEqual(calls[1:], [("reshape", (1, subject.WIDTH)), ("clone",)])
        with patch.object(subject.ablation, "tensor_bytes", return_value=row_bytes(0x3F80)), \
                self.assertRaises(subject.Failure):
            subject.bf16_row(raw, Torch)


class ReplayTests(unittest.TestCase):
    def test_record_retains_raw_disagreements_and_distinguishes_cases(self):
        residual = row_bytes(0x3F80)
        native = row_bytes(0x3F00)
        hf = replace_bits(native, 7, 0x3F01)
        source = replace_bits(native, 11, 0x3F02)
        weight = row_bytes(0x3F80)
        row = {"position": 4, "input_token": 5, "gpu_choice": 12095,
               "residual": residual, "normalized": native}
        cases = {"hf": {"inverse_rms_bits": "0x3f800000"},
                 "serial-rsqrt": {},
                 subject.SOURCE_CASE: {"inverse_rms_bits": "0x3f7fffff"}}
        record, retained = subject.record_row(
            row, cases, arithmetic_payload(residual, weight, hf, source)
        )
        self.assertEqual(record["source_case"], "serial-sqrt-reciprocal")
        self.assertEqual(record["comparisons"]["native_normalized_vs_hf"]["bit_mismatches"], 1)
        self.assertEqual(record["comparisons"]["native_normalized_vs_hf"]["metrics"]
                         ["bit_mismatches"], 1)
        self.assertEqual(record["comparisons"]["native_normalized_vs_source_serial_fp32"]
                         ["differences"][0]["index"], 11)
        self.assertEqual(retained["position-000004.native-normalized.bf16"], native)
        self.assertEqual(retained["position-000004.hf-output.bf16"], hf)
        self.assertEqual(retained["position-000004.source-output.bf16"], source)

    def test_nonfinite_native_normalized_is_retained_with_null_finite_metrics(self):
        residual = row_bytes(0x3F80)
        native = replace_bits(row_bytes(0x3F00), 17, 0x7FC1)
        replay = row_bytes(0x3F00)
        weight = row_bytes(0x3F80)
        row = {"position": 4, "input_token": 5, "gpu_choice": 12095,
               "residual": residual, "normalized": native}
        cases = {"hf": {}, "serial-rsqrt": {}, subject.SOURCE_CASE: {}}
        record, retained = subject.record_row(
            row, cases, arithmetic_payload(residual, weight, replay, replay)
        )
        comparison = record["comparisons"]["native_normalized_vs_source_serial_fp32"]
        self.assertEqual(comparison["differences"][0]["index"], 17)
        self.assertFalse(comparison["metrics"]["finite_values"])
        self.assertEqual(comparison["metrics"]["actual_nonfinite_count"], 1)
        for name in ("max_bf16_ulp", "max_absolute_error", "rmse"):
            self.assertIsNone(comparison["metrics"][name])
        self.assertEqual(retained["position-000004.native-normalized.bf16"], native)

    def test_replay_invokes_existing_ablation_once_per_held_native_row(self):
        residuals = [row_bytes(0x3F80), row_bytes(0xBF80)]
        normalized = [row_bytes(0x3F00), row_bytes(0xBF00)]
        native = {"residual.bf16": b"".join(residuals),
                  "normalized.bf16": b"".join(normalized)}
        manifest = {"positions": [4, 5], "input_tokens": [1, 2, 3, 4, 5, 6],
                    "gpu_choices": [7, 8, 9, 10, 12095, 13]}
        weight = object()
        weight_bytes = row_bytes(0x3F80)
        tensors = [object(), object()]
        calls = []

        def fake_ablate(tensor, actual_weight, torch, qwen):
            ordinal = tensors.index(tensor)
            calls.append((ordinal, actual_weight, torch, qwen))
            cases = {"hf": {}, "serial-rsqrt": {}, subject.SOURCE_CASE: {}}
            output = arithmetic_payload(
                residuals[ordinal], weight_bytes, normalized[ordinal], normalized[ordinal]
            )
            return cases, {"unused": True}, output

        with patch.object(subject, "bf16_row", side_effect=tensors) as decoder, \
                patch.object(subject.ablation, "ablate", side_effect=fake_ablate):
            records, retained = subject.replay_rows(
                native, manifest, weight, weight_bytes, "torch", "qwen"
            )
        self.assertEqual(decoder.call_count, 2)
        self.assertEqual(calls, [(0, weight, "torch", "qwen"), (1, weight, "torch", "qwen")])
        self.assertEqual([row["position"] for row in records], [4, 5])
        self.assertEqual(retained["final-rms-weight.bf16"], weight_bytes)
        self.assertEqual(len(retained), 21)

    def test_rejects_arithmetic_input_or_intermediate_drift(self):
        residual = row_bytes(0x3F80)
        weight = row_bytes(0x3F80)
        native = row_bytes(0x3F00)
        row = {"position": 4, "input_token": 5, "gpu_choice": 6,
               "residual": residual, "normalized": native}
        cases = {"hf": {}, "serial-rsqrt": {}, subject.SOURCE_CASE: {}}
        baseline = arithmetic_payload(residual, weight, native, native)
        for mutation in (
            lambda value: value.update({"input.bf16": row_bytes(0xBF80)}),
            lambda value: value.pop("hf.output.bf16"),
            lambda value: value.update({"hf.normalized.f32": value["hf.normalized.f32"][:-4]}),
            lambda value: value.update({"weight.bf16": value["weight.bf16"][:-2]}),
        ):
            value = dict(baseline)
            mutation(value)
            with self.assertRaises(subject.Failure):
                subject.record_row(row, cases, value)


class RealPinnedArithmeticTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.torch, _, cls.qwen, _, _ = subject.ablation.load_cpu_dependencies()

    def test_full_width_native_decode_ablate_and_record_replay(self):
        torch = self.torch
        residual = (
            ((torch.arange(subject.WIDTH, dtype=torch.int64) * 17) % 1021 - 510)
            .to(torch.float32)
            .div(64)
            .to(torch.bfloat16)
            .reshape(1, subject.WIDTH)
        )
        weight = torch.ones(subject.WIDTH, dtype=torch.bfloat16)
        residual_bytes = subject.ablation.tensor_bytes(residual, torch)
        weight_bytes = subject.ablation.tensor_bytes(weight, torch)
        native = {
            "residual.bf16": residual_bytes,
            "normalized.bf16": bytes(subject.WIDTH * 2),
        }
        manifest = {"positions": [0], "input_tokens": [785], "gpu_choices": [12095]}
        records, retained = subject.replay_rows(
            native, manifest, weight, weight_bytes, torch, self.qwen
        )
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]["source_case"], "serial-sqrt-reciprocal")
        self.assertEqual(retained["position-000000.native-residual.bf16"], residual_bytes)
        self.assertEqual(len(retained["position-000000.source-output.bf16"]),
                         subject.WIDTH * 2)
        self.assertEqual(records[0]["source_scalars"]["serial_square_sum_bits"][:2], "0x")


class RecordTests(unittest.TestCase):
    def test_result_is_explicitly_unqualified_and_binds_inputs(self):
        weight = row_bytes(0x3F80)
        native = {"manifest.json": b"manifest", "residual.bf16": b"residual"}
        witness = {"stdout.txt": b"stdout"}
        pins = b"pins"
        result = subject.result_document(
            {"setup": {"target": "gfx950:xnack-"}, "positions": [4, 5],
             "input_tokens": [1, 2, 3, 4, 5, 6]},
            [{"position": 4}], {"final-rms-weight.bf16": weight},
            {"source": "1" * 64}, pins, native, witness,
            {"python": "3.12"}, {"path": "/held/qwen.py", "sha256": "2" * 64},
        )
        for field in ("qualification", "benchmark_comparable", "numerical_pass_claimed",
                      "tolerance_reviewed", "cause_established", "device_behavior_established",
                      "compiler_behavior_established", "full_model_execution", "gpu_execution"):
            self.assertIs(result[field], False)
        self.assertEqual(result["native_pins_sha256"], subject.digest(pins))
        self.assertEqual(result["native_files_sha256"]["manifest.json"],
                         subject.digest(b"manifest"))
        self.assertEqual(result["weight"]["tensor"], "model.norm.weight")
        self.assertEqual(result["finite_replay_boundary"], {
            "residual_weight_and_cpu_intermediates_required_finite": True,
            "native_normalized_observation_may_be_nonfinite": True,
            "native_capture_is_separate_evidence": True,
        })
        self.assertIn("does not establish", result["nonclaim"])
        json.dumps(result, allow_nan=False)

    def test_usage_is_closed(self):
        for count in (0, 1, 5, 7):
            with self.subTest(count=count), self.assertRaises(subject.Failure):
                subject.run(["unused"] * count)


class FileAndRunTests(unittest.TestCase):
    def run_fixture(self, root, *, existing_output=False, drift=None):
        value = native_fixture()
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
        output = base / "output"
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
        weight = row_bytes(0x3F80)
        retained = {
            "final-rms-weight.bf16": weight,
            "position-000004.native-normalized.bf16": row_bytes(0x3F00),
        }
        records = [{"position": 4, "comparisons": {}}]

        def replay(*_):
            if drift == "capture":
                path = base / "capture" / "residual.bf16"
                changed = bytearray(path.read_bytes())
                changed[0:2] = b"\x80\x3f"
                path.write_bytes(changed)
            return records, retained

        source_hashes = {"implementation": "1" * 64}
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
                patch.object(subject, "implementation", return_value=source_hashes), \
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
                patch.object(subject, "load_final_weight", return_value=(object(), weight)), \
                patch.object(subject, "replay_rows", side_effect=replay), \
                patch.object(subject.ablation, "qwen_source_identity", return_value=qwen_identity):
            if existing_output or drift is not None:
                with self.assertRaises((subject.Failure, FileExistsError)):
                    subject.run(arguments)
            else:
                subject.run(arguments)

        if existing_output:
            self.assertEqual((output / "sentinel").read_bytes(), b"preserve")
            self.assertEqual(set(path.name for path in output.iterdir()), {"sentinel"})
        elif drift is not None:
            self.assertFalse(output.exists())
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
            self.assertEqual((output / "diagnostic.json").stat().st_mode & 0o777, 0o600)
            self.assertEqual(output.stat().st_mode & 0o777, 0o700)
        self.assertEqual(validations, [True])

    def test_valid_run_publishes_exact_new_output(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root)

    def test_existing_output_is_preserved(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, existing_output=True)

    def test_capture_or_source_drift_refuses_publication(self):
        for drift in ("capture", "source"):
            with self.subTest(drift=drift), tempfile.TemporaryDirectory() as root:
                self.run_fixture(root, drift=drift)


if __name__ == "__main__":
    unittest.main()
