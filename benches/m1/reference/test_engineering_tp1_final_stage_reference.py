#!/usr/bin/env python3
"""CPU-only production-extent tests; no numerical qualification or GPU claims."""

from contextlib import nullcontext
import importlib.util
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


specification = importlib.util.spec_from_file_location(
    "tp1_final_stage_reference_test_subject",
    Path(__file__).with_name("engineering_tp1_final_stage_reference.py"),
)
subject = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = subject
specification.loader.exec_module(subject)
core = subject.core


def encoded(value):
    return json.dumps(value, indent=2, allow_nan=False).encode()


def fixture(selected=None, prompt=None, new_tokens=2, *, arithmetic=None):
    selected = [4, 5] if selected is None else selected
    prompt = [785, 6722, 315, 9625, 374] if prompt is None else prompt
    length = len(prompt) + new_tokens - 1
    choices = [0] * length
    pins = {name: "1" * 64 for name in subject.IDENTITIES}
    pins.update(schema="FerricTpFinalStageNativePinsV1", authority="none", qualification=False,
                benchmark_comparable=False, model_bundle_id=core.PINNED_MODEL_IDENTITY,
                device_unique_ids=[16366993098680759275], prompt="The capital of France is",
                prompt_tokens=prompt, new_tokens=new_tokens, capacity=128, repetitions=1,
                warmup_runs=0, positions=selected)
    setup = {name: value for name, value in pins.items() if name in subject.PIN_FIELDS - {"positions"}}
    setup.update(schema="FerricQwen3TpEngineeringSetupV1", authority="none", model=core.PINNED_REPOSITORY,
                 dtype="BF16", target="gfx950:xnack-", tensor_parallel=1, worker_pids=[123],
                 running_worker_sha256=[pins["worker_sha256"]], executable_identity="live_proc_exe_sha256",
                 collective="host_staged_fp32_rank_order_reduce_bf16_residual", prefill="token_at_a_time_m1",
                 decoding="greedy_lowest_id_fixed_length", rank_zero_dispatch_budget=length * 544,
                 conservative_ring_packet_limit=131072, model_intake_seconds=1.25, setup_seconds=2.5,
                 numerical_status="Contracted; compare emitted token IDs independently",
                 timing="monotonic controller clock; includes IPC, host collectives and per-token progress logging; excludes setup",
                 final_stage_capture={"positions": selected, "benchmark_comparable": False,
                                      "timing": "diagnostic readbacks invalidate all performance measurements"})
    closed = {"schema": "FerricQwen3TpEngineeringClosedV1", "authority": "none",
              "worker_pids": [123], "all_workers_exited": True, "whole_seconds": 4.5}
    manifest = {"schema": "FerricTpFinalStageCaptureV1", "authority": "none", "complete": True,
                "worker_close_confirmed": True, "qualification": False, "benchmark_comparable": False,
                "numerical_pass_claimed": False, "setup": setup, "closed": closed, "positions": selected,
                "epoch": 1, "input_tokens": prompt + choices[len(prompt) - 1:-1], "gpu_choices": choices,
                "rows": [], "payloads": [], "maximum_total_bytes": subject.MAX_BYTES, "nonclaim": "observed bytes only"}
    data = {f"{name}.bf16": bytes(len(selected) * width * 2) for name, width in subject.WIDTHS.items()}
    intent = {"schema": "FerricTpFinalStageIntentV1", "authority": "none", "complete": False,
              "setup": setup, "positions": selected, "required_steps": length,
              "benchmark_comparable": False, "qualification": False}
    if arithmetic is not None:
        pins.update(schema="FerricTpArithmeticFinalStageNativePinsV1", residual_arithmetic=arithmetic)
        setup["residual_arithmetic"] = arithmetic
        setup["arithmetic_final_stage_capture"] = setup.pop("final_stage_capture")
        manifest.update(schema="FerricTpArithmeticFinalStageCaptureV1", residual_arithmetic=arithmetic)
        intent.update(schema="FerricTpArithmeticFinalStageIntentV1", residual_arithmetic=arithmetic)
    measurement = {"schema": "FerricQwen3TpEngineeringMeasurementV1", "authority": "none", "run": 0,
                   "warmup": False, "world_size": 1, "prompt_tokens": prompt,
                   "generated_tokens": choices[len(prompt) - 1:], "generated_text": "", "generated_utf8_bytes": [],
                   "ttft_seconds": 1.0, "tpot_seconds": 0.5, "decode_intervals_seconds": [0.5] * (new_tokens - 1),
                   "generation_seconds": 2.0, "rank_dispatch_counts": [length * 544],
                   "kv_tokens_processed": length, "benchmark_comparable": False}
    witness = {name: b"0\n" for name in ("process.exit", "wrapper.exit", "group-probe.exit")}
    witness["group-after.txt"] = b"leader and group absent\n"
    value = SimpleNamespace(pins=pins, manifest=manifest, data=data, intent=intent,
                            measurement=measurement, witness=witness)
    refresh(value)
    return value


def refresh(value):
    manifest = value.manifest
    manifest["payloads"] = []
    manifest["rows"] = [{"position": p, "input_token": manifest["input_tokens"][p],
                         "gpu_choice": manifest["gpu_choices"][p]} for p in manifest["positions"]]
    for name, width in subject.WIDTHS.items():
        filename = f"{name}.bf16"
        raw = value.data[filename]
        manifest["payloads"].append({"file": filename, "shape": [len(manifest["positions"]), width],
                                     "dtype": "bf16-little-endian", "bytes": len(raw), "sha256": subject.digest(raw)})
        for index, row in enumerate(manifest["rows"]):
            offset = index * width * 2
            part = raw[offset:offset + width * 2]
            row[name] = {"file": filename, "offset_bytes": offset, "bytes": len(part), "sha256": subject.digest(part)}
            if name == "logits":
                row["logits_diagnostics"] = subject.logits_diagnostics(part, row["gpu_choice"])
    seal(value)


def seal(value):
    value.data["intent.json"] = encoded(value.intent)
    value.data["manifest.json"] = encoded(value.manifest)
    receipt = {"schema": "FerricTpFinalStageCaptureReceiptV1", "authority": "none", "qualification": False,
               "benchmark_comparable": False, "manifest_sha256": subject.digest(value.data["manifest.json"]),
               "manifest_bytes": len(value.data["manifest.json"])}
    if value.manifest["schema"] == "FerricTpArithmeticFinalStageCaptureV1":
        receipt.update(schema="FerricTpArithmeticFinalStageCaptureReceiptV1",
                       residual_arithmetic=value.manifest["residual_arithmetic"])
    value.witness["stdout.txt"] = b"".join(json.dumps(part).encode() + b"\n" for part in (
        value.manifest["setup"], value.measurement, value.manifest["closed"], receipt))


def admit(value, *, expected_arithmetic=None):
    pin_bytes = encoded(value.pins)
    pins = subject.validate_pins(pin_bytes, subject.digest(pin_bytes), expected_arithmetic=expected_arithmetic)
    return subject.validate_capture(value.data, value.witness, pins, expected_arithmetic=expected_arithmetic)


class AdmissionTests(unittest.TestCase):
    def test_valid_generalized_shapes_and_exact_u64(self):
        for value in (fixture(), fixture([0], [42]), fixture(list(range(8)), list(range(8)))):
            with self.subTest(positions=value.pins["positions"]):
                document = admit(value)
                self.assertEqual(document["positions"], value.pins["positions"])
                self.assertIs(type(document["setup"]["device_unique_ids"][0]), int)

    def test_native_pins_are_independently_hashed_and_exact(self):
        original = fixture().pins
        for key, bad in (("model_bundle_id", "2" * 64), ("controller_sha256", "0" * 64),
                         ("device_unique_ids", [True]), ("device_unique_ids", [0]), ("device_unique_ids", [1.0]),
                         ("device_unique_ids", [2**64]), ("capacity", True), ("new_tokens", 2.0),
                         ("prompt_tokens", [False]), ("positions", [5, 4]), ("positions", [4, 4]),
                         ("positions", []), ("positions", [6]), ("positions", [4.0]),
                         ("positions", list(range(9))), ("qualification", 0)):
            with self.subTest(key=key, bad=bad):
                pins = {**original, key: bad}
                raw = encoded(pins)
                with self.assertRaises(subject.Failure):
                    subject.validate_pins(raw, subject.digest(raw))
        with self.assertRaises(subject.Failure):
            subject.validate_pins(encoded(original), "2" * 64)

    def test_rejects_duplicate_keys_and_nonfinite_json(self):
        for data in (b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}', b'{"a":1e999}', b'[]'):
            with self.subTest(data=data), self.assertRaises((subject.Failure, ValueError)):
                subject.document(data)

    def test_prompt_byte_bound_and_float_only_timing(self):
        pins = fixture().pins
        for prompt, accepted in (("a" * 16384, True), ("a" * 16385, False), ("\u00e9" * 8193, False)):
            raw = encoded({**pins, "prompt": prompt})
            if accepted:
                subject.validate_pins(raw, subject.digest(raw))
            else:
                with self.assertRaises(subject.Failure):
                    subject.validate_pins(raw, subject.digest(raw))
        for value in (0, 1, False, float("inf"), float("nan")):
            with self.assertRaises(subject.Failure):
                subject.duration(value, "fixture time")
        subject.duration(0.0, "fixture time")

    def test_terminal_and_roster_fail_closed(self):
        for name in subject.WITNESS_FILES:
            value = fixture()
            del value.witness[name]
            with self.subTest(missing=name), self.assertRaises(subject.Failure):
                admit(value)
        for name in ("process.exit", "wrapper.exit", "group-probe.exit", "group-after.txt"):
            for data in (b"1\n", b"0", b"00\n", b"0\n0\n", b""):
                value = fixture()
                value.witness[name] = data
                with self.subTest(name=name, data=data), self.assertRaises(subject.Failure):
                    admit(value)
        for missing in subject.CAPTURE_FILES:
            value = fixture()
            del value.data[missing]
            with self.subTest(missing=missing), self.assertRaises(subject.Failure):
                admit(value)
        value = fixture()
        value.data["manifest.incomplete.json"] = b"{}"
        with self.assertRaises(subject.Failure):
            admit(value)

    def test_stdout_crosslinks_and_receipt(self):
        changes = [(0, "controller_sha256", "2" * 64), (1, "run", True), (1, "world_size", 1.0),
                   (1, "generated_tokens", [1, 0]), (1, "rank_dispatch_counts", [1]),
                   (1, "kv_tokens_processed", 7), (1, "benchmark_comparable", True),
                   (1, "generated_text", "different"), (2, "worker_pids", [456]),
                   (2, "all_workers_exited", False), (3, "manifest_sha256", "2" * 64),
                   (3, "manifest_bytes", 1), (3, "qualification", 0)]
        for index, key, bad in changes:
            value = fixture()
            records = [json.loads(line) for line in value.witness["stdout.txt"].splitlines()]
            records[index][key] = bad
            value.witness["stdout.txt"] = b"".join(json.dumps(row).encode() + b"\n" for row in records)
            with self.subTest(index=index, key=key), self.assertRaises(subject.Failure):
                admit(value)
        for change in (lambda b: b + b"\n", lambda b: b.rstrip(), lambda b: b.splitlines()[0] + b"\n",
                       lambda b: b"\n".join(reversed(b.splitlines())) + b"\n"):
            value = fixture()
            value.witness["stdout.txt"] = change(value.witness["stdout.txt"])
            with self.assertRaises(subject.Failure):
                admit(value)

    def test_manifest_causality_descriptors_types_and_setup(self):
        mutations = [lambda v: v.manifest.update(epoch=True), lambda v: v.manifest.update(complete=1),
                     lambda v: v.manifest["input_tokens"].__setitem__(5, 1),
                     lambda v: v.manifest["gpu_choices"].__setitem__(0, False),
                     lambda v: v.manifest["rows"][0]["logits"].update(offset_bytes=2),
                     lambda v: v.manifest["rows"][0]["logits"].update(bytes=303872.0),
                     lambda v: v.manifest["rows"][0].update(gpu_choice=1),
                     lambda v: v.manifest["rows"][0]["logits_diagnostics"].update(cpu_choice=1),
                     lambda v: v.manifest["payloads"][0].update(sha256="2" * 64),
                     lambda v: v.manifest["payloads"][1].update(shape=[2, 4096.0]),
                     lambda v: v.manifest["setup"].update(worker_pids=[True]),
                     lambda v: v.manifest["setup"].update(worker_pids=[2**32]),
                     lambda v: v.manifest["setup"].update(target="gfx942:xnack-"),
                     lambda v: v.manifest["setup"].update(running_worker_sha256=["2" * 64]),
                     lambda v: v.manifest["setup"].update(model_intake_seconds=True),
                     lambda v: v.manifest["setup"].update(rank_zero_dispatch_budget=1),
                     lambda v: v.manifest["setup"].update(conservative_ring_packet_limit=131073),
                     lambda v: v.manifest["setup"].update(timing="changed"),
                     lambda v: v.manifest["closed"].update(whole_seconds=-1),
                     lambda v: v.intent.update(required_steps=7)]
        for index, mutation in enumerate(mutations):
            value = fixture()
            mutation(value)
            seal(value)
            with self.subTest(mutation=index), self.assertRaises(subject.Failure):
                admit(value)
        value = fixture()
        value.data["logits.bf16"] += b"\0\0"
        with self.assertRaises(subject.Failure):
            admit(value)

    def test_raw_nonfinite_and_gpu_disagreement_are_retained(self):
        for bits, index in ((0x7F80, 0), (0xFF80, core.VOCABULARY_SIZE - 1), (0x7FC1, 1), (0x3F80, 42)):
            value = fixture()
            raw = bytearray(value.data["logits.bf16"])
            struct.pack_into("<H", raw, index * 2, bits)
            value.data["logits.bf16"] = bytes(raw)
            refresh(value)
            document = admit(value)
            diagnostics = document["rows"][0]["logits_diagnostics"]
            self.assertEqual(diagnostics["finite"], bits == 0x3F80)
            self.assertEqual(diagnostics["gpu_choice_matches"], False if bits == 0x3F80 else None)


class ArithmeticAdmissionTests(unittest.TestCase):
    def test_both_explicit_modes_admit_only_with_exact_external_expectation(self):
        for mode in subject.ARITHMETIC_MODES:
            value = fixture(arithmetic=mode)
            manifest = admit(value, expected_arithmetic=mode)
            self.assertEqual(manifest["residual_arithmetic"], mode)
            self.assertEqual(manifest["setup"]["residual_arithmetic"], mode)
            self.assertNotIn("final_stage_capture", manifest["setup"])
            for expected in (None, "auto", True, "", *[m for m in subject.ARITHMETIC_MODES if m != mode]):
                with self.subTest(mode=mode, expected=expected), self.assertRaises(subject.Failure):
                    admit(value, expected_arithmetic=expected)
            pins = encoded(value.pins)
            with self.assertRaises(subject.Failure):
                subject.validate_pins(pins, "2" * 64, expected_arithmetic=mode)

    def test_legacy_records_are_not_upgraded_or_mode_inferred(self):
        for mode in subject.ARITHMETIC_MODES:
            value = fixture()
            with self.assertRaises(subject.Failure):
                admit(value, expected_arithmetic=mode)
            value.manifest["setup"]["residual_arithmetic"] = mode
            seal(value)
            with self.assertRaises(subject.Failure):
                admit(value)
            value = fixture(arithmetic=mode)
            with self.assertRaises(subject.Failure):
                subject.validate_capture(value.data, value.witness, value.pins)

    def test_all_mode_and_schema_crosslinks_fail_closed(self):
        mode, other = subject.ARITHMETIC_MODES
        changes = [lambda v: v.manifest.update(residual_arithmetic=other),
                   lambda v: v.manifest["setup"].update(residual_arithmetic=other),
                   lambda v: v.intent.update(residual_arithmetic=other),
                   lambda v: v.manifest.update(schema="FerricTpFinalStageCaptureV1"),
                   lambda v: v.intent.update(schema="FerricTpFinalStageIntentV1"),
                   lambda v: v.pins.update(schema="FerricTpFinalStageNativePinsV1"),
                   lambda v: v.pins.pop("residual_arithmetic"),
                   lambda v: v.manifest["setup"].update(final_stage_capture={}),
                   lambda v: v.manifest["setup"].pop("arithmetic_final_stage_capture"),
                   lambda v: v.manifest.update(numerical_pass_claimed=True),
                   lambda v: v.manifest["setup"].update(controller_sha256="2" * 64),
                   lambda v: v.manifest["setup"].update(artifact_hsaco_id="2" * 64)]
        for index, change in enumerate(changes):
            value = fixture(arithmetic=mode)
            change(value)
            seal(value)
            with self.subTest(change=index), self.assertRaises(subject.Failure):
                admit(value, expected_arithmetic=mode)
        for key, bad in (("residual_arithmetic", other), ("schema", "FerricTpFinalStageCaptureReceiptV1")):
            value = fixture(arithmetic=mode)
            records = [json.loads(line) for line in value.witness["stdout.txt"].splitlines()]
            records[3][key] = bad
            value.witness["stdout.txt"] = b"".join(json.dumps(row).encode() + b"\n" for row in records)
            with self.assertRaises(subject.Failure):
                admit(value, expected_arithmetic=mode)

    def test_arithmetic_capture_keeps_terminal_payload_and_causality_guards(self):
        mode = subject.ARITHMETIC_MODES[1]
        changes = [lambda v: v.witness.update({"process.exit": b"1\n"}),
                   lambda v: v.manifest["closed"].update(all_workers_exited=False),
                   lambda v: v.manifest["input_tokens"].__setitem__(5, 1),
                   lambda v: v.manifest["payloads"][0].update(sha256="2" * 64),
                   lambda v: v.manifest["rows"][0]["normalized"].update(offset_bytes=2),
                   lambda v: v.manifest["setup"].update(tensor_parallel=2)]
        for index, change in enumerate(changes):
            value = fixture(arithmetic=mode)
            change(value)
            seal(value)
            with self.subTest(change=index), self.assertRaises(subject.Failure):
                admit(value, expected_arithmetic=mode)
        value = fixture(arithmetic=mode)
        value.data["normalized.bf16"] = b"\x80\x3f" + value.data["normalized.bf16"][2:]
        with self.assertRaises(subject.Failure):
            admit(value, expected_arithmetic=mode)

    def test_nonfinite_disagreement_retention_and_all_three_comparisons(self):
        for mode in subject.ARITHMETIC_MODES:
            value = fixture(arithmetic=mode)
            value.data["logits.bf16"] = b"\xc1\x7f" + value.data["logits.bf16"][2:]
            refresh(value)
            manifest = admit(value, expected_arithmetic=mode)
            first = {name: data for name, data in value.data.items() if name.endswith(".bf16")}
            second = {name: b"\x80\x3f" + data[2:] for name, data in first.items()}
            result = subject.compare(value.data, [first, second], manifest, expected_arithmetic=mode)
            self.assertEqual(result["schema"], "FerricTpArithmeticFinalStageReferenceComparisonV1")
            self.assertEqual(result["residual_arithmetic"], mode)
            self.assertEqual(result["reference_arithmetic"], "unmodified-pinned-transformers-bf16-full-sequence")
            self.assertFalse(result["reference_byte_identical"])
            for key in ("qualification", "benchmark_comparable", "numerical_pass_claimed", "tolerance_reviewed", "cause_established"):
                self.assertIs(result[key], False)
            self.assertEqual(set(result["rows"][0]["stages"]), set(subject.WIDTHS))
            self.assertIsNone(result["rows"][0]["stages"]["logits"]["native_vs_pass2"]["rmse"])
            self.assertEqual(result["rows"][0]["stages"]["normalized"]["pass1_vs_pass2"]["bit_mismatches"], 1)
            with self.assertRaises(subject.Failure):
                subject.compare(value.data, [first, second], manifest)
            with self.assertRaises(subject.Failure):
                subject.compare(value.data, [first, second], manifest,
                                expected_arithmetic=next(m for m in subject.ARITHMETIC_MODES if m != mode))

    def test_arithmetic_entrypoint_requires_external_mode_before_execution(self):
        specification = importlib.util.spec_from_file_location(
            "arithmetic_final_stage_entry_test",
            Path(__file__).with_name("engineering_tp1_arithmetic_final_stage_reference.py"),
        )
        entry = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(entry)
        with patch.object(entry.reference, "run") as run:
            for mode in subject.ARITHMETIC_MODES:
                entry.run([mode, "capture", "witness", "pins", "sha", "model", "output"])
                run.assert_called_with(["capture", "witness", "pins", "sha", "model", "output"],
                                       expected_arithmetic=mode)
            run.reset_mock()
            for arguments in (["capture", "witness", "pins", "sha", "model", "output"],
                              ["auto", "capture", "witness", "pins", "sha", "model", "output"]):
                with self.assertRaises(entry.reference.Failure):
                    entry.run(arguments)
            run.assert_not_called()


class MetricsTests(unittest.TestCase):
    def test_signed_zero_ties_and_finite_metrics(self):
        zero = bytes(4096 * 2)
        negative_zero = b"\0\x80" + zero[2:]
        result = subject.row_metrics(negative_zero, zero, 4096)
        self.assertEqual((result["bit_mismatches"], result["max_bf16_ulp"], result["rmse"]), (1, 0, 0))
        logits = b"\0\x80" + bytes(core.VOCABULARY_SIZE * 2 - 2)
        self.assertEqual(subject.logits_diagnostics(logits, 42),
                         {"finite": True, "cpu_choice": 0, "gpu_choice_matches": False})
        with self.assertRaises(subject.Failure):
            subject.row_metrics(zero[:-2], zero, 4096)

    def test_two_differing_or_nonfinite_passes_keep_both_without_claims(self):
        value = fixture()
        first = {name: raw for name, raw in value.data.items() if name.endswith(".bf16")}
        second = dict(first)
        second["residual.bf16"] = b"\x80\x7f" + second["residual.bf16"][2:]
        result = subject.compare(value.data, [first, second], value.manifest)
        self.assertFalse(result["reference_byte_identical"])
        for name in ("qualification", "benchmark_comparable", "numerical_pass_claimed", "tolerance_reviewed", "cause_established"):
            self.assertIs(result[name], False)
        metrics = result["rows"][0]["stages"]["residual"]["pass1_vs_pass2"]
        self.assertEqual(metrics["reference_nonfinite_count"], 1)
        self.assertIsNone(metrics["rmse"])
        json.dumps(result, allow_nan=False)


class HookNorm:
    def __init__(self, fail_registration=False):
        self.hooks = {}
        self.fail_registration = fail_registration

    def register_forward_pre_hook(self, hook):
        self.hooks["before"] = hook
        return SimpleNamespace(remove=lambda: self.hooks.pop("before"))

    def register_forward_hook(self, hook):
        if self.fail_registration:
            raise RuntimeError("second hook registration failed")
        self.hooks["after"] = hook
        return SimpleNamespace(remove=lambda: self.hooks.pop("after"))

    def forward(self, tensor):
        self.hooks["before"](self, (tensor,))
        self.hooks["after"](self, (tensor,), tensor)


class FakeTensor:
    def __init__(self, shape, device, dtype="bf16"):
        self.shape, self.device, self.dtype = shape, device, dtype
        self.selection = None

    def __getitem__(self, selection):
        self.selection = selection
        return FakeTensor((1, len(selection[1]), self.shape[2]), self.device)

    def to(self, *, dtype):
        if dtype != self.dtype:
            raise AssertionError("unexpected dtype conversion")
        return self


class ExecutorTests(unittest.TestCase):
    def test_hooks_always_remove_handles(self):
        for mode in ("normal", "body", "duplicate", "missing", "bad-args", "wrong-module", "registration"):
            norm = HookNorm(mode == "registration")
            def invoke():
                with patch.object(subject, "raw_rows", return_value=b"raw"), \
                        subject.capture_hooks(norm, None, None, 6, [4, 5]):
                    if mode == "body":
                        raise RuntimeError("body failed")
                    if mode == "missing":
                        return
                    if mode == "bad-args":
                        norm.hooks["before"](norm, ())
                    if mode == "wrong-module":
                        norm.hooks["before"](object(), ("tensor",))
                    norm.forward("tensor")
                    if mode == "duplicate":
                        norm.forward("tensor")
            with self.subTest(mode=mode):
                if mode == "normal":
                    invoke()
                else:
                    with self.assertRaises((subject.Failure, RuntimeError)):
                        invoke()
                self.assertEqual(norm.hooks, {})

    def test_full_consumed_sequence_no_cache_selected_lm_head_and_sync(self):
        for sequence, selected in (([1, 2, 3, 4, 5, 6], [4, 5]), ([7, 8, 9], [0]), ([7, 8, 9, 10], [0, 2])):
            with self.subTest(sequence=sequence):
                calls = []
                device = SimpleNamespace(type="cuda")
                hidden = FakeTensor((1, len(sequence), 4096), device)
                norm = HookNorm()
                class Body:
                    def __init__(self):
                        self.norm = norm
                    def __call__(self, **kwargs):
                        self_kwargs = {"input_ids": "ids", "attention_mask": "mask", "return_dict": True, "use_cache": False}
                        if kwargs != self_kwargs:
                            raise AssertionError(kwargs)
                        norm.forward(hidden)
                        return SimpleNamespace(last_hidden_state=hidden)
                def tensor(value, *, dtype, device):
                    self.assertEqual(value, [sequence])
                    self.assertEqual(dtype, "long")
                    return "ids"
                def head(value):
                    self.assertEqual(value.shape, (1, len(selected), 4096))
                    return FakeTensor((1, len(selected), core.VOCABULARY_SIZE), device)
                def serialize(value, torch, received_device, length, rows, width):
                    subject.tensor_geometry(value, torch, device, (1, length, width))
                    self.assertEqual(received_device, device)
                    calls.append((length, rows, width))
                    return bytes(len(rows) * width * 2)
                torch = SimpleNamespace(bfloat16="bf16", long="long", tensor=tensor,
                                        ones_like=lambda ids: "mask", inference_mode=nullcontext,
                                        cuda=SimpleNamespace(synchronize=lambda d: calls.append(("sync", d))))
                model = SimpleNamespace(parameters=lambda: iter([hidden]), model=Body(), lm_head=head)
                with patch.object(subject, "raw_rows", side_effect=serialize):
                    output = subject.execute(model, torch, sequence, selected)
                self.assertEqual(hidden.selection[1], selected)
                self.assertEqual(calls[:3], [(len(sequence), selected, 4096)] * 3)
                self.assertEqual(calls[3], (len(selected), list(range(len(selected))), core.VOCABULARY_SIZE))
                self.assertEqual(calls[-1], ("sync", device))
                self.assertEqual(norm.hooks, {})
                self.assertEqual(set(output), {f"{name}.bf16" for name in subject.WIDTHS})
                with patch.object(subject, "raw_rows", side_effect=[b"input", b"normalized", b"different"]), \
                        self.assertRaisesRegex(subject.Failure, "last hidden state"):
                    subject.execute(model, torch, sequence, selected)
                self.assertEqual(norm.hooks, {})

    def test_raw_serializer_keeps_nonfinite_bytes_and_selection(self):
        device = SimpleNamespace(type="cuda")
        raw = b"\x80\x7f\xc1\x7f\0\x80" + bytes(2 * 4096 * 2 - 6)
        calls = []
        class RawTensor:
            dtype, shape = "bf16", (1, 6, 4096)
            def __init__(self):
                self.device = device
            def __getitem__(self, selected):
                calls.append(selected)
                return self
            def detach(self):
                return self
            def contiguous(self):
                return self
            def cpu(self):
                return self
            def view(self, dtype):
                calls.append(dtype)
                return self
            def flatten(self):
                return self
            def tolist(self):
                return list(raw)
        torch = SimpleNamespace(bfloat16="bf16", uint8="uint8")
        actual = subject.raw_rows(RawTensor(), torch, device, 6, [1, 4], 4096)
        self.assertEqual(actual, raw)
        self.assertEqual(calls, [(0, [1, 4], slice(None)), "uint8"])

    def test_rejects_cpu_dtype_and_geometry(self):
        torch = SimpleNamespace(bfloat16="bf16")
        for parameter in (None, FakeTensor((1, 6, 4096), SimpleNamespace(type="cpu")),
                          FakeTensor((1, 6, 4096), SimpleNamespace(type="cuda"), "fp32")):
            model = SimpleNamespace(parameters=lambda: iter([] if parameter is None else [parameter]))
            with self.assertRaises(subject.Failure):
                subject.execute(model, torch, [1, 2], [1])
        for tensor in (FakeTensor((1, 6, 4095), "gpu"), FakeTensor((1, 6, 4096), "cpu"),
                       FakeTensor((1, 6, 4096), "gpu", "fp32")):
            with self.assertRaises(subject.Failure):
                subject.raw_rows(tensor, torch, "gpu", 6, [4], 4096)


class FileAndRunTests(unittest.TestCase):
    def test_arithmetic_implementation_binds_entrypoint_and_shared_sources(self):
        legacy = {"engineering_tp1_final_stage_reference.py", "run.py", "pyproject.toml", "uv.lock"}
        entrypoint = "engineering_tp1_arithmetic_final_stage_reference.py"
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payloads = {name: name.encode("ascii") for name in legacy | {entrypoint}}
            for name, data in payloads.items():
                (root / name).write_bytes(data)
            with patch.object(subject, "__file__", str(root / "engineering_tp1_final_stage_reference.py")):
                expected = {name: subject.digest(payloads[name]) for name in legacy}
                self.assertEqual(subject.implementation(), expected)
                for mode in subject.ARITHMETIC_MODES:
                    self.assertEqual(subject.implementation(expected_arithmetic=mode), {
                        **expected, entrypoint: subject.digest(payloads[entrypoint]),
                    })
                (root / entrypoint).write_bytes(b"changed entrypoint")
                self.assertEqual(subject.implementation(), expected)
                for mode in subject.ARITHMETIC_MODES:
                    self.assertEqual(subject.implementation(expected_arithmetic=mode)[entrypoint],
                                     subject.digest(b"changed entrypoint"))

    def test_held_files_reject_links_mutations_and_directory_replacement(self):
        for mode in ("symlink", "hardlink", "contents", "inode", "directory", "extra"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as root:
                path = Path(root) / "input"
                path.mkdir()
                file = path / "file"
                file.write_bytes(b"before")
                if mode in ("symlink", "hardlink"):
                    other = Path(root) / "original"
                    file.rename(other)
                    if mode == "symlink":
                        file.symlink_to(other)
                    else:
                        os.link(other, file)
                with self.assertRaises(subject.Failure):
                    with subject.held_directory(path, {"file"}, "fixture"):
                        if mode == "contents":
                            file.write_bytes(b"changed")
                        elif mode == "inode":
                            file.unlink()
                            file.write_bytes(b"before")
                        elif mode == "directory":
                            path.rename(Path(root) / "old")
                            path.mkdir()
                            (path / "file").write_bytes(b"before")
                        elif mode == "extra":
                            (path / "extra").write_bytes(b"extra")

    def run_fixture(self, root, fail_source=False, existing_output=False, alias=None, arithmetic=None):
        value = fixture(arithmetic=arithmetic)
        base = Path(root)
        for name, data in (("capture", value.data), ("witness", value.witness)):
            directory = base / name
            directory.mkdir()
            for filename, raw in data.items():
                (directory / filename).write_bytes(raw)
        raw_pins = encoded(value.pins)
        (base / "pins").mkdir()
        (base / "pins" / "pins.json").write_bytes(raw_pins)
        (base / "model").mkdir()
        for component in ("draft", "target"):
            (base / "model" / component).mkdir()
        first = {name: data for name, data in value.data.items() if name.endswith(".bf16")}
        second = {**first, "logits.bf16": b"\xc1\x7f" + first["logits.bf16"][2:]}
        validations = []
        def validate():
            validations.append(1)
            if fail_source:
                raise subject.Failure("model changed after pass")
        target_stat = (base / "model" / "target").stat()
        source = SimpleNamespace(validate=validate, target=SimpleNamespace(
            identity=core.FileIdentity(target_stat.st_dev, target_stat.st_ino)))
        output = base / "output" if alias is None else base / alias / "output"
        if existing_output:
            output.mkdir()
            (output / "sentinel").write_bytes(b"preserve")
        with patch.object(core, "require_isolated_python"), patch.object(core, "require_virtual_environment"), \
                patch.object(subject, "implementation", return_value={"test": "1" * 64}), \
                patch.object(core, "authenticate_model_source", return_value=nullcontext(source)), \
                patch.object(core, "load_dependencies", return_value=SimpleNamespace(torch=None)), \
                patch.object(core, "load_model", return_value=object()), \
                patch.object(subject, "execute", side_effect=[first, second]) as executor:
            arguments = [str(base / "capture"), str(base / "witness"), str(base / "pins" / "pins.json"),
                         subject.digest(raw_pins), str(base / "model"), str(output)]
            if fail_source or existing_output or alias is not None:
                with self.assertRaises((subject.Failure, FileExistsError)):
                    subject.run(arguments, expected_arithmetic=arithmetic)
                if alias is not None:
                    self.assertEqual(executor.call_count, 0)
                    self.assertFalse(output.exists())
                elif fail_source:
                    self.assertEqual(executor.call_count, 1)
                    self.assertFalse(output.exists())
                else:
                    self.assertEqual((output / "sentinel").read_bytes(), b"preserve")
                return
            subject.run(arguments, expected_arithmetic=arithmetic)
        self.assertEqual(len(validations), 2)
        result = json.loads((output / "comparison.json").read_bytes())
        self.assertFalse(result["reference_byte_identical"])
        self.assertFalse(result["numerical_pass_claimed"])
        if arithmetic is not None:
            self.assertEqual(result["residual_arithmetic"], arithmetic)
            self.assertEqual(result["schema"], "FerricTpArithmeticFinalStageReferenceComparisonV1")
        for ordinal, data in enumerate((first, second), 1):
            for name, raw in data.items():
                self.assertEqual((output / f"pass{ordinal}-{name}").read_bytes(), raw)
                self.assertEqual((output / f"pass{ordinal}-{name}").stat().st_mode & 0o777, 0o600)
        self.assertEqual(output.stat().st_mode & 0o777, 0o700)

    def test_complete_run_retains_both_nonidentical_raw_passes(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root)

    def test_arithmetic_run_preserves_both_raw_passes_and_publication_guards(self):
        for mode in subject.ARITHMETIC_MODES:
            for options in ({}, {"fail_source": True}, {"existing_output": True}, {"alias": "capture"}):
                with self.subTest(mode=mode, options=options), tempfile.TemporaryDirectory() as root:
                    self.run_fixture(root, arithmetic=mode, **options)

    def test_model_revalidated_after_each_pass(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, fail_source=True)

    def test_output_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as root:
            self.run_fixture(root, existing_output=True)

    def test_output_cannot_modify_authenticated_input_directories(self):
        for alias in ("capture", "witness", "pins", "model", "model/target", "model/draft"):
            with self.subTest(alias=alias), tempfile.TemporaryDirectory() as root:
                self.run_fixture(root, alias=alias)
        with core.SecureDirectory.open(Path(subject.__file__).parent, "reference source") as source:
            with self.assertRaisesRegex(subject.Failure, "aliases"):
                subject.reject_output_alias(source, {source.identity})


if __name__ == "__main__":
    unittest.main()
