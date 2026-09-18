#!/usr/bin/env python3
"""Unqualified, two-pass full-sequence reference for TP1 final-stage readbacks."""

from __future__ import annotations

from contextlib import ExitStack, contextmanager
import importlib.util
import json
import math
import os
from pathlib import Path
import struct
import sys


specification = importlib.util.spec_from_file_location(
    "ferric_tp1_final_stage_core", Path(__file__).with_name("run.py")
)
if specification is None or specification.loader is None:
    raise ImportError("cannot load sibling reference core")
core = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = core
specification.loader.exec_module(core)
Failure = core.ReferenceFailure
digest = core.sha256_bytes
WIDTHS = {"residual": 4096, "normalized": 4096, "logits": core.VOCABULARY_SIZE}
MAX_BYTES = 4 * 1024 * 1024
CAPTURE_FILES = {"intent.json", "manifest.json"} | {f"{name}.bf16" for name in WIDTHS}
WITNESS_FILES = {"stdout.txt", "process.exit", "wrapper.exit", "group-probe.exit", "group-after.txt"}
IDENTITIES = {"controller_sha256", "worker_sha256", "model_bundle_id",
              "artifact_hsaco_id", "artifact_manifest_id", "artifact_handoff_id"}
PIN_FIELDS = IDENTITIES | {"device_unique_ids", "prompt", "prompt_tokens", "new_tokens",
                           "capacity", "repetitions", "warmup_runs", "positions"}
ARITHMETIC_MODES = (
    "fp32-rank-sum-plus-residual-then-bf16-v1",
    "fp32-rank-sum-then-bf16-plus-residual-then-bf16-v1",
)
NONCLAIM = ("Diagnostic cross-host comparison only: gfx950 Ferric readbacks and gfx942 "
            "independent full-sequence reference. No tolerance acceptance, numerical pass, "
            "causal attribution, protected proof, performance claim, or M1 gate closure.")


def equal(actual, expected, label: str) -> None:
    # Canonical encoding distinguishes bool/int/float; input JSON itself is not canonical.
    if core.canonical_bytes(actual) != core.canonical_bytes(expected):
        raise Failure(f"{label} drifted")


def object_keys(value, keys: set[str], label: str) -> None:
    if type(value) is not dict or set(value) != keys:
        raise Failure(f"{label} field roster drifted")


def integer(value, lower: int, upper: int, label: str) -> int:
    if type(value) is not int or not lower <= value <= upper:
        raise Failure(f"{label} is outside its integer bound")
    return value


def tokens(value, label: str) -> list[int]:
    if type(value) is not list or not 1 <= len(value) <= 8192:
        raise Failure(f"{label} must contain 1..8192 token IDs")
    for token in value:
        integer(token, 0, core.VOCABULARY_SIZE - 1, label)
    return value


def positions(value, length: int) -> list[int]:
    if type(value) is not list or not 1 <= len(value) <= 8:
        raise Failure("capture must select 1..8 positions")
    for position in value:
        integer(position, 0, length - 1, "capture position")
    if value != sorted(set(value)):
        raise Failure("capture positions must be strictly increasing")
    return value


def duration(value, label: str) -> None:
    if type(value) is not float or not math.isfinite(value) or value < 0:
        raise Failure(f"{label} must be a finite nonnegative JSON float")


def document(data: bytes) -> dict:
    if not 0 < len(data) <= MAX_BYTES:
        raise Failure("JSON exceeds diagnostic bound")
    value = json.loads(data, object_pairs_hook=core._unique_object,
                       parse_constant=core._reject_constant)
    if type(value) is not dict:
        raise Failure("JSON must be an object")
    # Also reject overflowing finite-number syntax such as 1e999.
    core.canonical_bytes(value)
    return value


def capture_profile(expected_arithmetic: str | None) -> tuple[str, str, set[str]]:
    if expected_arithmetic is None:
        return "FerricTpFinalStage", "final_stage_capture", PIN_FIELDS
    if type(expected_arithmetic) is not str or expected_arithmetic not in ARITHMETIC_MODES:
        raise Failure("explicit external residual arithmetic expectation is invalid")
    return "FerricTpArithmeticFinalStage", "arithmetic_final_stage_capture", PIN_FIELDS | {"residual_arithmetic"}


def bound_record(record: dict, expected_arithmetic: str | None) -> dict:
    if expected_arithmetic is not None:
        record["residual_arithmetic"] = expected_arithmetic
    return record


def validate_pins(data: bytes, expected_sha256: str, *, expected_arithmetic: str | None = None) -> dict:
    prefix, _, fields = capture_profile(expected_arithmetic)
    core.require_sha256(expected_sha256, "externally retained native pins SHA256")
    equal(digest(data), expected_sha256, "native pins SHA256")
    pins = document(data)
    object_keys(pins, fields | {"schema", "authority", "qualification", "benchmark_comparable"}, "native pins")
    for key, value in bound_record({"schema": f"{prefix}NativePinsV1", "authority": "none",
                       "qualification": False, "benchmark_comparable": False,
                       "repetitions": 1, "warmup_runs": 0}, expected_arithmetic).items():
        equal(pins[key], value, f"pins {key}")
    for key in IDENTITIES:
        core.require_sha256(pins[key], key)
        if pins[key] == "0" * 64:
            raise Failure(f"{key} must be nonzero")
    equal(pins["model_bundle_id"], core.PINNED_MODEL_IDENTITY, "canonical model identity")
    devices = pins["device_unique_ids"]
    if type(devices) is not list or len(devices) != 1:
        raise Failure("pins require one physical device")
    integer(devices[0], 1, 2**64 - 1, "physical device ID")
    prompt = tokens(pins["prompt_tokens"], "prompt")
    if type(pins["prompt"]) is not str or not 0 < len(pins["prompt"].encode("utf-8")) <= 16384:
        raise Failure("prompt text extent drifted")
    required = len(prompt) + integer(pins["new_tokens"], 2, 256, "new tokens") - 1
    if required * 544 > 131072:
        raise Failure("native sequence exceeds the no-ring-rollover dispatch budget")
    integer(pins["capacity"], required, 8192, "capacity")
    positions(pins["positions"], required)
    return pins


def logits_diagnostics(data: bytes, choice: int) -> dict:
    if len(data) != core.VOCABULARY_SIZE * 2:
        raise Failure("logit row extent drifted")
    for index, (bits,) in enumerate(struct.iter_unpack("<H", data)):
        if bits & 0x7F80 == 0x7F80:
            return {"finite": False, "first_nonfinite_token": index,
                    "cpu_choice": None, "gpu_choice_matches": None}
    cpu = core.bf16_argmax(data)
    return {"finite": True, "cpu_choice": cpu, "gpu_choice_matches": cpu == choice}


def validate_capture(payloads: dict[str, bytes], witness: dict[str, bytes], pins: dict,
                     *, expected_arithmetic: str | None = None) -> dict:
    prefix, capture_key, fields = capture_profile(expected_arithmetic)
    # Even direct parser callers must supply the matching, closed pin schema.
    pin_bytes = core.canonical_bytes(pins)
    validate_pins(pin_bytes, digest(pin_bytes), expected_arithmetic=expected_arithmetic)
    if set(payloads) != CAPTURE_FILES or sum(map(len, payloads.values())) > MAX_BYTES:
        raise Failure("capture file roster or total extent drifted")
    if set(witness) != WITNESS_FILES:
        raise Failure("native witness roster drifted")
    for name in ("process.exit", "wrapper.exit", "group-probe.exit"):
        if witness[name] != b"0\n":
            raise Failure(f"{name} must record successful native termination")
    if witness["group-after.txt"] != b"leader and group absent\n":
        raise Failure("native process leader/group absence not confirmed")
    stdout = witness["stdout.txt"]
    if not 0 < len(stdout) <= MAX_BYTES or not stdout.endswith(b"\n"):
        raise Failure("native stdout extent/termination drifted")
    lines = stdout.splitlines()
    if len(lines) != 4 or not all(lines):
        raise Failure("native stdout must contain Setup, Measurement, Closed, Receipt")
    setup, measured, closed, receipt = map(document, lines)
    manifest = document(payloads["manifest.json"])
    intent = document(payloads["intent.json"])
    expected = bound_record({"schema": f"{prefix}CaptureV1", "authority": "none", "complete": True,
                "worker_close_confirmed": True, "qualification": False,
                "benchmark_comparable": False, "numerical_pass_claimed": False,
                "maximum_total_bytes": MAX_BYTES}, expected_arithmetic)
    object_keys(manifest, set(expected) | {"setup", "closed", "positions", "epoch", "input_tokens",
                                          "gpu_choices", "rows", "payloads", "nonclaim"}, "manifest")
    for key, value in expected.items():
        equal(manifest[key], value, f"manifest {key}")
    if type(manifest["nonclaim"]) is not str or not 0 < len(manifest["nonclaim"]) <= 4096:
        raise Failure("native nonclaim missing")
    equal(manifest["setup"], setup, "stdout/manifest setup")
    equal(manifest["closed"], closed, "stdout/manifest close")
    integer(manifest["epoch"], 0, 2**64 - 1, "capture epoch")
    for key in fields - {"positions"}:
        equal(setup[key], pins[key], f"setup frozen {key}")
    setup_expected = {"schema": "FerricQwen3TpEngineeringSetupV1", "authority": "none",
                      "model": core.PINNED_REPOSITORY, "dtype": "BF16", "target": "gfx950:xnack-",
                      "tensor_parallel": 1, "running_worker_sha256": [pins["worker_sha256"]],
                      "executable_identity": "live_proc_exe_sha256",
                      "collective": "host_staged_fp32_rank_order_reduce_bf16_residual",
                      "prefill": "token_at_a_time_m1", "decoding": "greedy_lowest_id_fixed_length",
                      "numerical_status": "Contracted; compare emitted token IDs independently",
                      "timing": "monotonic controller clock; includes IPC, host collectives and per-token progress logging; excludes setup",
                      "conservative_ring_packet_limit": 131072,
                      capture_key: {"positions": pins["positions"], "benchmark_comparable": False,
                                              "timing": "diagnostic readbacks invalidate all performance measurements"}}
    object_keys(setup, (fields - {"positions"}) | set(setup_expected) | {
        "worker_pids", "rank_zero_dispatch_budget", "conservative_ring_packet_limit",
        "model_intake_seconds", "setup_seconds", "numerical_status", "timing"}, "setup")
    for key, value in setup_expected.items():
        equal(setup[key], value, f"setup {key}")
    pids = setup["worker_pids"]
    if type(pids) is not list or len(pids) != 1:
        raise Failure("setup requires one worker PID")
    integer(pids[0], 2, 2**32 - 1, "worker PID")
    for key in ("rank_zero_dispatch_budget", "conservative_ring_packet_limit"):
        integer(setup[key], 1, 2**64 - 1, key)
    for key in ("model_intake_seconds", "setup_seconds"):
        duration(setup[key], key)
    for key in ("numerical_status", "timing"):
        if type(setup[key]) is not str or not setup[key]:
            raise Failure(f"setup {key} missing")
    if len(json.dumps(setup, separators=(",", ":"), ensure_ascii=False).encode("utf-8")) > 65536:
        raise Failure("setup exceeds producer bound")
    prompt = pins["prompt_tokens"]
    required = len(prompt) + pins["new_tokens"] - 1
    equal(setup["rank_zero_dispatch_budget"], required * 544, "rank zero dispatch budget")
    selected = positions(manifest["positions"], required)
    equal(selected, pins["positions"], "selected positions")
    sequence = tokens(manifest["input_tokens"], "consumed sequence")
    choices = tokens(manifest["gpu_choices"], "actual GPU choices")
    equal(len(choices), required, "choice count")
    equal(sequence, prompt + choices[len(prompt) - 1:-1], "prompt/GPU-choice continuation")
    equal(intent, bound_record({"schema": f"{prefix}IntentV1", "authority": "none", "complete": False,
                   "setup": setup, "positions": selected, "required_steps": required,
                   "benchmark_comparable": False, "qualification": False}, expected_arithmetic), "capture intent")
    equal(closed, {"schema": "FerricQwen3TpEngineeringClosedV1", "authority": "none",
                   "worker_pids": pids, "all_workers_exited": True,
                   "whole_seconds": closed["whole_seconds"]}, "successful worker close")
    duration(closed["whole_seconds"], "whole seconds")
    equal(receipt, bound_record({"schema": f"{prefix}CaptureReceiptV1", "authority": "none",
                    "qualification": False, "benchmark_comparable": False,
                    "manifest_sha256": digest(payloads["manifest.json"]),
                    "manifest_bytes": len(payloads["manifest.json"])}, expected_arithmetic), "manifest receipt")
    measurement = {"schema": "FerricQwen3TpEngineeringMeasurementV1", "authority": "none",
                   "run": 0, "warmup": False, "world_size": 1, "prompt_tokens": prompt,
                   "generated_tokens": choices[len(prompt) - 1:], "kv_tokens_processed": required,
                   "rank_dispatch_counts": [required * 544], "benchmark_comparable": False}
    object_keys(measured, set(measurement) | {"generated_text", "generated_utf8_bytes", "ttft_seconds",
                                             "tpot_seconds", "decode_intervals_seconds", "generation_seconds"}, "measurement")
    for key, value in measurement.items():
        equal(measured[key], value, f"measurement {key}")
    for key in ("ttft_seconds", "tpot_seconds", "generation_seconds"):
        duration(measured[key], key)
    intervals = measured["decode_intervals_seconds"]
    if type(intervals) is not list or len(intervals) != pins["new_tokens"] - 1:
        raise Failure("decode interval count drifted")
    for value in intervals:
        duration(value, "decode interval")
    encoded = measured["generated_utf8_bytes"]
    if type(encoded) is not list or len(encoded) > 65536:
        raise Failure("generated byte extent drifted")
    for value in encoded:
        integer(value, 0, 255, "generated byte")
    try:
        text = bytes(encoded).decode("utf-8")
    except UnicodeDecodeError:
        text = None
    equal(measured["generated_text"], text, "generated text/bytes")
    descriptors = []
    rows = manifest["rows"]
    if type(rows) is not list or len(rows) != len(selected):
        raise Failure("selected row count drifted")
    expected_rows = [{"position": p, "input_token": sequence[p], "gpu_choice": choices[p]} for p in selected]
    for name, width in WIDTHS.items():
        filename = f"{name}.bf16"
        data = payloads[filename]
        if len(data) != len(selected) * width * 2:
            raise Failure(f"{name} payload extent drifted")
        descriptors.append({"file": filename, "shape": [len(selected), width], "dtype": "bf16-little-endian",
                            "bytes": len(data), "sha256": digest(data)})
        for index, row in enumerate(expected_rows):
            offset = index * width * 2
            raw = data[offset:offset + width * 2]
            row[name] = {"file": filename, "offset_bytes": offset, "bytes": len(raw), "sha256": digest(raw)}
            if name == "logits":
                row["logits_diagnostics"] = logits_diagnostics(raw, row["gpu_choice"])
    equal(manifest["payloads"], descriptors, "payload descriptors")
    equal(rows, expected_rows, "row descriptors and raw logit diagnostics")
    return manifest


def row_metrics(actual: bytes, expected: bytes, width: int) -> dict:
    if len(actual) != width * 2 or len(expected) != width * 2 or width not in WIDTHS.values():
        raise Failure("comparison row extent drifted")
    left = [v[0] for v in struct.iter_unpack("<H", actual)]
    right = [v[0] for v in struct.iter_unpack("<H", expected)]
    counts = [sum(bits & 0x7F80 == 0x7F80 for bits in row) for row in (left, right)]
    result = {"elements": width, "bit_mismatches": sum(a != b for a, b in zip(left, right, strict=True)),
              "actual_nonfinite_count": counts[0], "reference_nonfinite_count": counts[1],
              "finite_values": not any(counts), "max_bf16_ulp": None, "max_absolute_error": None, "rmse": None}
    if not any(counts):
        def number(bits):
            return struct.unpack("<f", struct.pack("<I", bits << 16))[0]
        def ordered(bits):
            return 0x8000 - (bits & 0x7FFF) if bits & 0x8000 else 0x8000 + bits
        differences = [number(a) - number(b) for a, b in zip(left, right, strict=True)]
        result.update(max_bf16_ulp=max(abs(ordered(a) - ordered(b)) for a, b in zip(left, right, strict=True)),
                      max_absolute_error=max(map(abs, differences)),
                      rmse=math.sqrt(math.fsum(value * value for value in differences) / width))
    if width == core.VOCABULARY_SIZE:
        a = None if counts[0] else core.bf16_argmax(actual)
        b = None if counts[1] else core.bf16_argmax(expected)
        result.update(actual_argmax=a, reference_argmax=b,
                      token_mismatch=None if a is None or b is None else a != b)
    return result


def tensor_geometry(tensor, torch, device, shape: tuple[int, ...]) -> None:
    if tensor.dtype != torch.bfloat16 or tuple(tensor.shape) != shape or tensor.device != device:
        raise Failure("reference tensor shape, dtype, or device drifted")


def raw_rows(tensor, torch, device, length: int, selected: list[int], width: int) -> bytes:
    tensor_geometry(tensor, torch, device, (1, length, width))
    if sys.byteorder != "little":
        raise Failure("reference requires little-endian BF16 storage")
    data = bytes(tensor[0, selected, :].detach().contiguous().cpu().view(torch.uint8).flatten().tolist())
    if len(data) != len(selected) * width * 2:
        raise Failure("raw BF16 serialization extent drifted")
    return data


@contextmanager
def capture_hooks(norm, torch, device, length: int, selected: list[int]):
    captured = {}
    def before(module, arguments):
        if module is not norm or len(arguments) != 1 or "residual.bf16" in captured:
            raise Failure("final norm pre-hook invocation drifted")
        captured["residual.bf16"] = raw_rows(arguments[0], torch, device, length, selected, 4096)

    def after(module, arguments, output):
        if module is not norm or len(arguments) != 1 or "residual.bf16" not in captured or "normalized.bf16" in captured:
            raise Failure("final norm post-hook invocation drifted")
        captured["normalized.bf16"] = raw_rows(output, torch, device, length, selected, 4096)

    with ExitStack() as hooks:
        hooks.callback(norm.register_forward_pre_hook(before).remove)
        hooks.callback(norm.register_forward_hook(after).remove)
        yield captured
        if set(captured) != {"residual.bf16", "normalized.bf16"}:
            raise Failure("reference must invoke final norm exactly once")


def execute(model, torch, sequence: list[int], selected: list[int]) -> dict[str, bytes]:
    tokens(sequence, "reference consumed sequence")
    positions(selected, len(sequence))
    parameter = next(model.parameters(), None)
    if parameter is None or parameter.device.type != "cuda" or parameter.dtype != torch.bfloat16:
        raise Failure("reference model must be BF16 on the guarded GPU")
    device = parameter.device
    with torch.inference_mode(), capture_hooks(model.model.norm, torch, device, len(sequence), selected) as captured:
        ids = torch.tensor([sequence], dtype=torch.long, device=device)
        mask = torch.ones_like(ids)
        hidden = model.model(input_ids=ids, attention_mask=mask, return_dict=True, use_cache=False).last_hidden_state
        tensor_geometry(hidden, torch, device, (1, len(sequence), 4096))
        if raw_rows(hidden, torch, device, len(sequence), selected, 4096) != captured.get("normalized.bf16"):
            raise Failure("last hidden state differs from captured final norm output")
        logits = model.lm_head(hidden[:, selected, :]).to(dtype=torch.bfloat16)
        raw_logits = raw_rows(logits, torch, device, len(selected), list(range(len(selected))), core.VOCABULARY_SIZE)
        torch.cuda.synchronize(device)
    return {**captured, "logits.bf16": raw_logits}


def implementation(*, expected_arithmetic: str | None = None) -> dict[str, str]:
    capture_profile(expected_arithmetic)
    names = (Path(__file__).name, "run.py", "pyproject.toml", "uv.lock")
    if expected_arithmetic is not None:
        names += ("engineering_tp1_arithmetic_final_stage_reference.py",)
    with core.SecureDirectory.open(Path(__file__).parent, "TP1 reference source") as directory:
        return {name: digest(directory.read(name, "reference implementation", maximum=2 * 1024 * 1024))
                for name in names}


@contextmanager
def held_directory(path: Path, names: set[str], label: str, identities=None):
    with core.SecureDirectory.open(path, label) as directory, ExitStack() as held:
        if identities is not None:
            identities.add(directory.identity)
        if directory.entries() != names:
            raise Failure(f"{label} roster drifted")
        files = {name: held.enter_context(directory.open_file(name, label)) for name in sorted(names)}
        payloads = {name: file.read(maximum=MAX_BYTES) for name, file in files.items()}
        yield payloads
        core.validate_bound_files(directory, files, label)
        with core.SecureDirectory.open(path, label) as reopened:
            if reopened.identity != directory.identity:
                raise Failure(f"{label} directory identity drifted")


def compare(payloads: dict[str, bytes], passes: list[dict[str, bytes]], manifest: dict,
            *, expected_arithmetic: str | None = None) -> dict:
    prefix, _, _ = capture_profile(expected_arithmetic)
    equal(manifest["schema"], f"{prefix}CaptureV1", "comparison capture schema")
    if expected_arithmetic is not None:
        equal(manifest["residual_arithmetic"], expected_arithmetic, "comparison arithmetic")
        equal(manifest["setup"]["residual_arithmetic"], expected_arithmetic, "comparison setup arithmetic")
    elif "residual_arithmetic" in manifest or "residual_arithmetic" in manifest["setup"]:
        raise Failure("legacy comparison rejects explicit residual arithmetic")
    if len(passes) != 2:
        raise Failure("reference must retain two executions")
    selected = manifest["positions"]
    expected_names = {f"{name}.bf16" for name in WIDTHS}
    for output in passes:
        if set(output) != expected_names or any(len(output[f"{name}.bf16"]) != len(selected) * width * 2
                                                for name, width in WIDTHS.items()):
            raise Failure("reference pass output geometry drifted")
    rows = []
    for index, position in enumerate(selected):
        stages = {}
        for name, width in WIDTHS.items():
            span = slice(index * width * 2, (index + 1) * width * 2)
            native = payloads[f"{name}.bf16"][span]
            reference = [output[f"{name}.bf16"][span] for output in passes]
            stages[name] = {"native_vs_pass1": row_metrics(native, reference[0], width),
                            "native_vs_pass2": row_metrics(native, reference[1], width),
                            "pass1_vs_pass2": row_metrics(reference[0], reference[1], width)}
        rows.append({"position": position, "input_token": manifest["input_tokens"][position],
                     "gpu_choice": manifest["gpu_choices"][position], "stages": stages})
    result = bound_record({"schema": f"{prefix}ReferenceComparisonV1", "authority": "none",
            "qualification": False, "benchmark_comparable": False, "numerical_pass_claimed": False,
            "tolerance_reviewed": False, "cause_established": False,
            "process_absence_independently_proven": False,
            "reference_execution": "two-independent-full-sequence-executions-use-cache-false",
            "reference_byte_identical": passes[0] == passes[1],
            "reference_model": {"repository": core.PINNED_REPOSITORY, "revision": core.PINNED_REVISION},
            "native_target": manifest["setup"]["target"], "reference_target": core.TARGET,
            "input_tokens": manifest["input_tokens"], "positions": selected,
            "rows": rows, "nonclaim": NONCLAIM}, expected_arithmetic)
    if expected_arithmetic is not None:
        result["reference_arithmetic"] = "unmodified-pinned-transformers-bf16-full-sequence"
    return result


def run(arguments: list[str], *, expected_arithmetic: str | None = None) -> None:
    capture_profile(expected_arithmetic)
    if len(arguments) != 6:
        raise Failure("usage: engineering_tp1_final_stage_reference.py CAPTURE WITNESS PINS PINS-SHA256 MODEL-SOURCE NEW-OUTPUT")
    capture_path, witness_path, pins_path, pins_sha, model_path, output_path = arguments
    core.require_isolated_python()
    core.require_virtual_environment()
    source_hashes = implementation(expected_arithmetic=expected_arithmetic)
    with ExitStack() as held:
        pins_parent, pins_name = core.open_parent(Path(pins_path), "native pins")
        held.enter_context(pins_parent)
        parent, name = core.open_parent(Path(output_path), "TP1 reference output")
        held.enter_context(parent)
        pins_file = held.enter_context(pins_parent.open_file(pins_name, "native pins"))
        forbidden = {pins_parent.identity}
        input_directories = {}
        for path in (Path(__file__).parent, Path(model_path), Path(model_path) / "target", Path(model_path) / "draft"):
            directory = held.enter_context(core.SecureDirectory.open(path, "input/source directory"))
            forbidden.add(directory.identity)
            input_directories[path] = directory
        pin_bytes = pins_file.read(maximum=65536)
        pins = validate_pins(pin_bytes, pins_sha, expected_arithmetic=expected_arithmetic)
        with held_directory(Path(capture_path), CAPTURE_FILES, "native capture", forbidden) as payloads, \
                held_directory(Path(witness_path), WITNESS_FILES, "native witnesses", forbidden) as witness:
            reject_output_alias(parent, forbidden)
            manifest = validate_capture(payloads, witness, pins, expected_arithmetic=expected_arithmetic)
            with core.authenticate_model_source(Path(model_path)) as source:
                if source.target.identity != input_directories[Path(model_path) / "target"].identity:
                    raise Failure("authenticated model target directory changed")
                dependencies = core.load_dependencies()
                model = core.load_model(dependencies, source)
                passes = []
                for _ in range(2):
                    passes.append(execute(model, dependencies.torch, manifest["input_tokens"], manifest["positions"]))
                    source.validate()
            result = compare(payloads, passes, manifest, expected_arithmetic=expected_arithmetic)
        pins_file.validate()
        with pins_parent.open_file(pins_name, "reopened native pins") as reopened:
            if reopened.identity != pins_file.identity:
                raise Failure("native pin file identity drifted")
        with core.SecureDirectory.open(Path(pins_path).parent, "reopened native pins parent") as reopened:
            if reopened.identity != pins_parent.identity:
                raise Failure("native pin parent identity drifted")
        equal(implementation(expected_arithmetic=expected_arithmetic), source_hashes, "reference implementation")
        outputs = {f"pass{index}-{name}": data for index, output in enumerate(passes, 1) for name, data in output.items()}
        result.update(implementation_sha256=source_hashes, native_pins_sha256=digest(pin_bytes),
                      native_files_sha256={name: digest(data) for name, data in payloads.items()},
                      native_witnesses_sha256={name: digest(data) for name, data in witness.items()},
                      reference_files_sha256={name: digest(data) for name, data in outputs.items()})
        with core.SecureDirectory.open(Path(output_path).parent, "output parent binding") as reopened:
            if reopened.identity != parent.identity:
                raise Failure("output parent changed during reference execution")
        reject_output_alias(parent, forbidden)
        os.mkdir(name, mode=0o700, dir_fd=parent.fd)
        with parent.child(name, "new TP1 reference output") as output:
            # Preserve both raw passes before reporting nonfinite or repeat disagreement.
            for filename, data in outputs.items():
                core.write_new(output.fd, filename, data, "raw reference pass")
            core.write_new(output.fd, "comparison.json", core.canonical_bytes(result), "TP1 comparison")
            os.fsync(output.fd)
        os.fsync(parent.fd)
    print(f"output={output_path} authority=none qualification=false reference_byte_identical={result['reference_byte_identical']}")


def reject_output_alias(parent, forbidden) -> None:
    if parent.identity in forbidden:
        raise Failure("output parent aliases an authenticated input/source directory")


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (Failure, OSError, ValueError, KeyError, TypeError, AttributeError, OverflowError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
