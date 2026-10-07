#!/usr/bin/env python3
"""CPU-only same-input layer-0 attention-output residual diagnostic."""

from __future__ import annotations

from contextlib import ExitStack
import importlib.util
import json
import math
import os
from pathlib import Path
import struct
import sys


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


reference = load_sibling(
    "ferric_tp1_layer0_residual_input", "engineering_tp1_final_stage_reference.py"
)
ablation = load_sibling(
    "ferric_tp1_layer0_residual_dependencies", "engineering_rmsnorm_ablation.py"
)
head = load_sibling(
    "ferric_tp1_layer0_residual_head_helpers",
    "engineering_tp1_lm_head_ablation.py",
)
core = reference.core
Failure = reference.Failure
digest = core.sha256_bytes

WIDTH = 4096
INDEX = "model.safetensors.index.json"
CONFIG = "config.json"
WEIGHT = "model.layers.0.self_attn.o_proj.weight"
WEIGHT_SHARD = "model-00001-of-00005.safetensors"
WEIGHT_BYTES = WIDTH * WIDTH * 2
WEIGHT_OFFSETS = [1_555_054_848, 1_588_609_280]
PROJECTION_SOURCE = "device/qwen3-tp-kernels-v1/src/projection.rs"
PROJECTION_SOURCE_SHA256 = "f900506b27296d248e0ccea0bbf96be2a14418134a40937e5b6c7e0830f46d1c"
PROJECTION_SYMBOL = "ferric_qwen3_tp_gemv_partial_bf16_f32_v1"
COLLECTIVE_SOURCE = "adapters/m1-engineering-execution-v1/src/tp_execution/collective.rs"
COLLECTIVE_SOURCE_SHA256 = "3650e86002f942bb524600ac4a970ae39d48e55a878020d823b9da7927d23e65"
COLLECTIVE_SYMBOL = "reduce_residual_bf16_v1"
CAPTURE_SCHEMA = "FerricTpResidualBoundaryCaptureV1"
INTENT_SCHEMA = "FerricTpResidualBoundaryIntentV1"
RECEIPT_SCHEMA = "FerricTpResidualBoundaryCaptureReceiptV1"
PAYLOADS = (
    ("projection_input", "projection-input.bf16", "bf16-little-endian", 2),
    ("residual_before", "residual-before.bf16", "bf16-little-endian", 2),
    ("projection_partial", "projection-partial.f32le", "f32-little-endian", 4),
    ("hidden_after_broadcast", "hidden-after-broadcast.bf16", "bf16-little-endian", 2),
)
CAPTURE_FILES = {"intent.json", "manifest.json"} | {item[1] for item in PAYLOADS}
MAX_CAPTURE_BYTES = 4 * 1024 * 1024
MAX_RAW_BYTES = 8 * 1024 * 1024
MAX_DOCUMENT_BYTES = 4 * 1024 * 1024
NONCLAIM = (
    "CPU same-input layer-0 attention-output residual localization diagnostic only. "
    "This does not establish device or compiler behavior, a numerical tolerance, an "
    "accepted result, a causal defect, qualification authority, protected proof, "
    "performance, full-model execution, or any M1 gate closure."
)


def implementation() -> dict[str, str]:
    root = Path(__file__).resolve().parents[3]
    names = (
        "benches/m1/reference/engineering_tp1_layer0_residual_ablation.py",
        "benches/m1/reference/engineering_tp1_lm_head_ablation.py",
        "benches/m1/reference/engineering_tp1_final_stage_reference.py",
        "benches/m1/reference/engineering_rmsnorm_ablation.py",
        "benches/m1/reference/run.py",
        "benches/m1/reference/pyproject.toml",
        "benches/m1/reference/uv.lock",
        PROJECTION_SOURCE,
        COLLECTIVE_SOURCE,
    )
    with core.SecureDirectory.open(root, "same-input LM-head diagnostic source") as source:
        hashes = {
            name: digest(
                source.read(name, "diagnostic implementation", maximum=2 * 1024 * 1024)
            )
            for name in names
        }
    if hashes[PROJECTION_SOURCE] != PROJECTION_SOURCE_SHA256:
        raise Failure("source-spelled output projection implementation drifted")
    if hashes[COLLECTIVE_SOURCE] != COLLECTIVE_SOURCE_SHA256:
        raise Failure("source-spelled host residual reducer implementation drifted")
    return hashes


def parse_weight_binding(index_bytes: bytes, config_bytes: bytes) -> None:
    index = json.loads(
        index_bytes,
        object_pairs_hook=core._unique_object,
        parse_constant=core._reject_constant,
    )
    config = json.loads(
        config_bytes,
        object_pairs_hook=core._unique_object,
        parse_constant=core._reject_constant,
    )
    if type(index) is not dict or type(index.get("weight_map")) is not dict:
        raise Failure("checkpoint weight map is malformed")
    if index["weight_map"].get(WEIGHT) != WEIGHT_SHARD:
        raise Failure("layer-0 o_proj weight-to-shard mapping drifted")
    if type(config) is not dict or config.get("hidden_size") != WIDTH:
        raise Failure("checkpoint hidden-size contract drifted")
    if type(config.get("num_hidden_layers")) is not int or config["num_hidden_layers"] < 1:
        raise Failure("checkpoint has no layer zero")


def tensor_bytes(tensor, torch) -> bytes:
    try:
        return head.tensor_bytes(tensor, torch)
    except head.Failure as error:
        raise Failure(str(error)) from error


def require_finite(tensor, torch, label: str) -> None:
    try:
        head.require_finite(tensor, torch, label)
    except head.Failure as error:
        raise Failure(str(error)) from error


def load_projection_weight(source, safetensors, torch):
    parse_weight_binding(
        source.files[INDEX].read(exact=core.MODEL_FILES[INDEX][0]),
        source.files[CONFIG].read(exact=core.MODEL_FILES[CONFIG][0]),
    )
    with safetensors.safe_open(
        f"/proc/self/fd/{source.files[WEIGHT_SHARD].fd}",
        framework="pt",
        device="cpu",
    ) as shard:
        if shard.get_slice(WEIGHT).get_shape() != [WIDTH, WIDTH]:
            raise Failure("layer-0 o_proj weight geometry drifted")
        weight = shard.get_tensor(WEIGHT).clone()
    if (
        tuple(weight.shape) != (WIDTH, WIDTH)
        or weight.dtype != torch.bfloat16
        or weight.device.type != "cpu"
        or not weight.is_contiguous()
        or tuple(weight.stride()) != (WIDTH, 1)
    ):
        raise Failure("layer-0 o_proj shape, dtype, device, or layout drifted")
    require_finite(weight, torch, "layer-0 o_proj weight")
    if weight.numel() * weight.element_size() != WEIGHT_BYTES:
        raise Failure("layer-0 o_proj byte extent drifted")
    source.validate()
    descriptor = {
        "tensor": WEIGHT,
        "shard": WEIGHT_SHARD,
        "shape": [WIDTH, WIDTH],
        "dtype": "bf16-little-endian",
        "data_offsets": WEIGHT_OFFSETS,
        "tensor_bytes": WEIGHT_BYTES,
        "shard_bytes": core.MODEL_FILES[WEIGHT_SHARD][0],
        "shard_sha256": core.MODEL_FILES[WEIGHT_SHARD][1],
        "emitted_tensor_sha256": digest(tensor_bytes(weight, torch)),
        "retained_weight_bytes": False,
    }
    return weight, descriptor


def validate_capture(payloads: dict[str, bytes], witness: dict[str, bytes], pins: dict) -> dict:
    if set(payloads) != CAPTURE_FILES or sum(map(len, payloads.values())) > MAX_CAPTURE_BYTES:
        raise Failure("boundary capture file roster or total extent drifted")
    if set(witness) != reference.WITNESS_FILES:
        raise Failure("native witness roster drifted")
    for name in ("process.exit", "wrapper.exit", "group-probe.exit"):
        if witness[name] != b"0\n":
            raise Failure(f"{name} must record successful native termination")
    if witness["group-after.txt"] != b"leader and group absent\n":
        raise Failure("native process leader/group absence not confirmed")
    stdout = witness["stdout.txt"]
    if not 0 < len(stdout) <= MAX_CAPTURE_BYTES or not stdout.endswith(b"\n"):
        raise Failure("native stdout extent or termination drifted")
    lines = stdout.splitlines()
    if len(lines) != 4 or not all(lines):
        raise Failure("native stdout must contain Setup, Measurement, Closed, Receipt")
    setup, measured, closed, receipt = map(reference.document, lines)
    manifest = reference.document(payloads["manifest.json"])
    intent = reference.document(payloads["intent.json"])
    expected = {
        "schema": CAPTURE_SCHEMA,
        "authority": "none",
        "complete": True,
        "worker_close_confirmed": True,
        "qualification": False,
        "benchmark_comparable": False,
        "numerical_pass_claimed": False,
        "maximum_total_bytes": MAX_CAPTURE_BYTES,
    }
    reference.object_keys(
        manifest,
        set(expected)
        | {"setup", "closed", "positions", "epoch", "input_tokens", "gpu_choices",
           "rows", "payloads", "nonclaim"},
        "boundary manifest",
    )
    for key, value in expected.items():
        reference.equal(manifest[key], value, f"boundary manifest {key}")
    if type(manifest["nonclaim"]) is not str or not 0 < len(manifest["nonclaim"]) <= 4096:
        raise Failure("boundary nonclaim missing")
    reference.equal(manifest["setup"], setup, "stdout/manifest setup")
    reference.equal(manifest["closed"], closed, "stdout/manifest close")
    reference.integer(manifest["epoch"], 0, 2**64 - 1, "capture epoch")
    for key in reference.PIN_FIELDS - {"positions"}:
        reference.equal(setup[key], pins[key], f"setup frozen {key}")
    captured_tensor_sha = setup.get("residual_boundary_capture", {}).get(
        "output_projection_weight_shard_sha256"
    )
    core.require_sha256(captured_tensor_sha, "captured output projection tensor SHA256")
    if captured_tensor_sha == "0" * 64:
        raise Failure("captured output projection tensor SHA256 is zero")
    selection = {
        "positions": pins["positions"],
        "layer": 0,
        "operation": "AttentionOutputSum",
        "tensor_parallel_rank": 0,
        "tensor_parallel_world": 1,
        "output_projection_weight_tensor": WEIGHT,
        "output_projection_weight_shape": [WIDTH, WIDTH],
        "projection_input_elements": WIDTH,
        "residual_elements": WIDTH,
        "projection_partial_elements": WIDTH,
        "hidden_after_elements": WIDTH,
        "residual_source": "host_staged_collective_input",
        "output_projection_weight_shard_sha256": captured_tensor_sha,
        "benchmark_comparable": False,
        "timing": "diagnostic readbacks invalidate all performance measurements",
    }
    setup_expected = {
        "schema": "FerricQwen3TpEngineeringSetupV1",
        "authority": "none",
        "model": core.PINNED_REPOSITORY,
        "dtype": "BF16",
        "target": "gfx950:xnack-",
        "tensor_parallel": 1,
        "running_worker_sha256": [pins["worker_sha256"]],
        "executable_identity": "live_proc_exe_sha256",
        "collective": "host_staged_fp32_rank_order_reduce_bf16_residual",
        "prefill": "token_at_a_time_m1",
        "decoding": "greedy_lowest_id_fixed_length",
        "numerical_status": "Contracted; compare emitted token IDs independently",
        "timing": (
            "monotonic controller clock; includes IPC, host collectives and per-token "
            "progress logging; excludes setup"
        ),
        "conservative_ring_packet_limit": 131072,
        "residual_boundary_capture": selection,
    }
    reference.object_keys(
        setup,
        (reference.PIN_FIELDS - {"positions"}) | set(setup_expected)
        | {"worker_pids", "rank_zero_dispatch_budget", "conservative_ring_packet_limit",
           "model_intake_seconds", "setup_seconds", "numerical_status", "timing"},
        "setup",
    )
    for key, value in setup_expected.items():
        reference.equal(setup[key], value, f"setup {key}")
    pids = setup["worker_pids"]
    if type(pids) is not list or len(pids) != 1:
        raise Failure("setup requires one worker PID")
    reference.integer(pids[0], 2, 2**32 - 1, "worker PID")
    for key in ("rank_zero_dispatch_budget", "conservative_ring_packet_limit"):
        reference.integer(setup[key], 1, 2**64 - 1, key)
    for key in ("model_intake_seconds", "setup_seconds"):
        reference.duration(setup[key], key)
    prompt = pins["prompt_tokens"]
    required = len(prompt) + pins["new_tokens"] - 1
    reference.equal(setup["rank_zero_dispatch_budget"], required * 544, "dispatch budget")
    selected = reference.positions(manifest["positions"], required)
    reference.equal(selected, pins["positions"], "selected positions")
    sequence = reference.tokens(manifest["input_tokens"], "consumed sequence")
    choices = reference.tokens(manifest["gpu_choices"], "actual GPU choices")
    reference.equal(len(choices), required, "choice count")
    reference.equal(sequence, prompt + choices[len(prompt) - 1 : -1], "consumed sequence")
    reference.equal(
        intent,
        {"schema": INTENT_SCHEMA, "authority": "none", "complete": False,
         "setup": setup, "positions": selected, "required_steps": required,
         "benchmark_comparable": False, "qualification": False},
        "boundary intent",
    )
    reference.equal(
        closed,
        {"schema": "FerricQwen3TpEngineeringClosedV1", "authority": "none",
         "worker_pids": pids, "all_workers_exited": True,
         "whole_seconds": closed["whole_seconds"]},
        "successful worker close",
    )
    reference.duration(closed["whole_seconds"], "whole seconds")
    reference.equal(
        receipt,
        {"schema": RECEIPT_SCHEMA, "authority": "none", "qualification": False,
         "benchmark_comparable": False,
         "manifest_sha256": digest(payloads["manifest.json"]),
         "manifest_bytes": len(payloads["manifest.json"])},
        "boundary manifest receipt",
    )
    measurement = {
        "schema": "FerricQwen3TpEngineeringMeasurementV1", "authority": "none",
        "run": 0, "warmup": False, "world_size": 1, "prompt_tokens": prompt,
        "generated_tokens": choices[len(prompt) - 1 :], "kv_tokens_processed": required,
        "rank_dispatch_counts": [required * 544], "benchmark_comparable": False,
    }
    reference.object_keys(
        measured,
        set(measurement) | {"generated_text", "generated_utf8_bytes", "ttft_seconds",
                            "tpot_seconds", "decode_intervals_seconds", "generation_seconds"},
        "measurement",
    )
    for key, value in measurement.items():
        reference.equal(measured[key], value, f"measurement {key}")
    for key in ("ttft_seconds", "tpot_seconds", "generation_seconds"):
        reference.duration(measured[key], key)
    intervals = measured["decode_intervals_seconds"]
    if type(intervals) is not list or len(intervals) != pins["new_tokens"] - 1:
        raise Failure("decode interval roster drifted")
    for interval in intervals:
        reference.duration(interval, "decode interval")
    encoded = measured["generated_utf8_bytes"]
    if type(encoded) is not list or len(encoded) > 65536:
        raise Failure("generated byte extent drifted")
    for value in encoded:
        reference.integer(value, 0, 255, "generated byte")
    try:
        generated_text = bytes(encoded).decode("utf-8")
    except UnicodeDecodeError:
        generated_text = None
    reference.equal(measured["generated_text"], generated_text, "generated text/bytes")

    expected_payloads = []
    for _, filename, dtype, element_bytes in PAYLOADS:
        raw = payloads[filename]
        extent = len(selected) * WIDTH * element_bytes
        if len(raw) != extent:
            raise Failure(f"boundary payload {filename} extent drifted")
        expected_payloads.append({
            "file": filename, "shape": [len(selected), WIDTH], "dtype": dtype,
            "element_bytes": element_bytes, "bytes": extent, "sha256": digest(raw),
        })
    reference.equal(manifest["payloads"], expected_payloads, "boundary payload descriptors")
    rows = manifest["rows"]
    if type(rows) is not list or len(rows) != len(selected):
        raise Failure("boundary row roster drifted")
    for ordinal, row in enumerate(rows):
        reference.object_keys(
            row,
            {"position", "input_token", "layer", "operation", "projection_input",
             "residual_before", "projection_partial", "hidden_after_broadcast",
             "gpu_hidden_matches_host_broadcast"},
            "boundary row",
        )
        reference.equal(row["position"], selected[ordinal], "boundary row position")
        reference.equal(row["input_token"], sequence[selected[ordinal]], "boundary row token")
        reference.equal(row["layer"], 0, "boundary row layer")
        reference.equal(row["operation"], "AttentionOutputSum", "boundary row operation")
        if type(row["gpu_hidden_matches_host_broadcast"]) is not bool:
            raise Failure("boundary producer comparison changed type")
        for key, filename, _, element_bytes in PAYLOADS:
            row_extent = WIDTH * element_bytes
            span = slice(ordinal * row_extent, (ordinal + 1) * row_extent)
            reference.equal(
                row[key],
                {"file": filename, "offset_bytes": ordinal * row_extent,
                 "bytes": row_extent, "sha256": digest(payloads[filename][span])},
                f"boundary row {key}",
            )
    return manifest


def native_rows(payloads: dict[str, bytes], manifest: dict) -> list[dict]:
    rows = []
    for ordinal, manifest_row in enumerate(manifest["rows"]):
        row = dict(manifest_row)
        for key, filename, _, element_bytes in PAYLOADS:
            extent = WIDTH * element_bytes
            row[key] = payloads[filename][ordinal * extent : (ordinal + 1) * extent]
        rows.append(row)
    return rows


def bf16_row(data: bytes, width: int, torch, label: str):
    if sys.byteorder != "little" or len(data) != width * 2:
        raise Failure(f"{label} is not one little-endian BF16 row")
    tensor = torch.frombuffer(bytearray(data), dtype=torch.bfloat16).reshape(1, width).clone()
    if tensor_bytes(tensor, torch) != data:
        raise Failure(f"{label} BF16 decoding changed its bytes")
    return tensor


def validate_linear_inputs(normalized, weight, torch) -> tuple[int, int]:
    if (
        normalized.ndim != 2
        or normalized.shape[0] != 1
        or weight.ndim != 2
        or weight.shape[1] != normalized.shape[1]
        or normalized.dtype != torch.bfloat16
        or weight.dtype != torch.bfloat16
        or normalized.device.type != "cpu"
        or weight.device.type != "cpu"
    ):
        raise Failure("LM-head inputs must be one BF16 CPU row and one BF16 CPU matrix")
    require_finite(normalized, torch, "normalized input")
    require_finite(weight, torch, "LM-head weight")
    return int(weight.shape[0]), int(weight.shape[1])


def torch_linear_once(normalized, weight, torch) -> bytes:
    validate_linear_inputs(normalized, weight, torch)
    try:
        return head.torch_linear_once(normalized, weight, torch)
    except head.Failure as error:
        raise Failure(str(error)) from error


def prepare_source_weights(weight, torch):
    if (
        tuple(weight.shape) != (WIDTH, WIDTH)
        or weight.dtype != torch.bfloat16
        or weight.device.type != "cpu"
    ):
        raise Failure("source-spelled projection weight geometry drifted")
    require_finite(weight, torch, "layer-0 o_proj weight")
    by_inner = torch.empty((WIDTH, WIDTH), dtype=torch.float32, device="cpu")
    by_inner.copy_(weight.transpose(0, 1))
    if not by_inner.is_contiguous() or tuple(by_inner.stride()) != (WIDTH, 1):
        raise Failure("source-spelled FP32 weight workspace layout drifted")
    require_finite(by_inner, torch, "FP32 layer-0 o_proj weight workspace")
    return by_inner


def ascending_fp32_once(normalized, weights_by_inner, torch) -> tuple[bytes, bytes]:
    try:
        return head.ascending_fp32_once(normalized, weights_by_inner, torch)
    except head.Failure as error:
        raise Failure(str(error)) from error


def host_residual_once(partial: bytes, residual: bytes, torch) -> tuple[bytes, bytes]:
    if sys.byteorder != "little" or len(partial) != WIDTH * 4 or len(residual) != WIDTH * 2:
        raise Failure("host residual operands have invalid geometry")
    partial_tensor = torch.frombuffer(bytearray(partial), dtype=torch.float32).clone()
    residual_tensor = bf16_row(residual, WIDTH, torch, "host residual").reshape(WIDTH)
    with torch.inference_mode():
        summed = torch.zeros(WIDTH, dtype=torch.float32, device="cpu")
        torch.add(summed, partial_tensor, out=summed)
        torch.add(summed, residual_tensor.to(torch.float32), out=summed)
    summed_raw = tensor_bytes(summed, torch)
    rounded = bytearray()
    for (bits,) in struct.iter_unpack("<I", summed_raw):
        value = ((bits + 0x7FFF + ((bits >> 16) & 1)) & 0xFFFFFFFF) >> 16
        rounded.extend(struct.pack("<H", value))
    return summed_raw, bytes(rounded)


def projection_rounded_residual_once(partial: bytes, residual: bytes, torch) -> dict[str, bytes]:
    if sys.byteorder != "little" or len(partial) != WIDTH * 4 or len(residual) != WIDTH * 2:
        raise Failure("projection-first residual operands have invalid geometry")
    partial_tensor = torch.frombuffer(bytearray(partial), dtype=torch.float32).clone()
    # Use the pinned conversion for nonfinite inputs too; retain the original partial separately.
    with torch.inference_mode():
        projection = partial_tensor.to(torch.bfloat16)
        widened = projection.to(torch.float32)
    projection_raw = tensor_bytes(projection, torch)
    widened_raw = tensor_bytes(widened, torch)
    summed, hidden = host_residual_once(widened_raw, residual, torch)
    return {"projection_bf16": projection_raw, "partial_f32": widened_raw,
            "sum_f32": summed, "hidden_bf16": hidden}


def _ordered_f32(bits: int) -> int:
    return (
        0x80000000 - (bits & 0x7FFFFFFF)
        if bits & 0x80000000
        else 0x80000000 + bits
    )


def f32_metrics(actual: bytes, expected: bytes) -> dict:
    if len(actual) != WIDTH * 4 or len(expected) != WIDTH * 4:
        raise Failure("FP32 comparison row extent drifted")
    actual_bits = [item[0] for item in struct.iter_unpack("<I", actual)]
    expected_bits = [item[0] for item in struct.iter_unpack("<I", expected)]
    actual_values = [item[0] for item in struct.iter_unpack("<f", actual)]
    expected_values = [item[0] for item in struct.iter_unpack("<f", expected)]
    nonfinite_actual = sum(not math.isfinite(value) for value in actual_values)
    nonfinite_expected = sum(not math.isfinite(value) for value in expected_values)
    result = {
        "bit_mismatches": sum(left != right for left, right in zip(actual_bits, expected_bits)),
        "actual_nonfinite_count": nonfinite_actual,
        "expected_nonfinite_count": nonfinite_expected,
        "max_fp32_ulp": None,
        "max_abs": None,
        "rmse": None,
    }
    if nonfinite_actual == 0 and nonfinite_expected == 0:
        differences = [abs(left - right) for left, right in zip(actual_values, expected_values)]
        result["max_fp32_ulp"] = max(
            abs(_ordered_f32(left) - _ordered_f32(right))
            for left, right in zip(actual_bits, expected_bits)
        )
        result["max_abs"] = max(differences)
        result["rmse"] = math.sqrt(math.fsum(value * value for value in differences) / WIDTH)
    return result


def record_row(row: dict, native: dict[str, bytes], passes: dict[str, list], retained: dict[str, bytes]) -> dict:
    if set(passes) != {
        "torch_bf16_linear", "source_ascending_fp32", "native_partial_host",
        "native_partial_bf16_first",
    }:
        raise Failure("residual replay case roster drifted")
    if any(len(values) != 2 for values in passes.values()):
        raise Failure("each residual replay case must retain exactly two passes")
    prefix = f"position-{row['position']:06d}"
    row_payloads = {
        f"{prefix}.native-projection-input.bf16": native["projection_input"],
        f"{prefix}.native-residual-before.bf16": native["residual_before"],
        f"{prefix}.native-projection-partial.f32": native["projection_partial"],
        f"{prefix}.native-hidden-after-broadcast.bf16": native["hidden_after_broadcast"],
    }
    for case, values in passes.items():
        for number, output in enumerate(values, 1):
            for field, raw in output.items():
                suffix = "bf16" if field.endswith("bf16") else "f32"
                row_payloads[f"{prefix}.{case}-pass{number}-{field}.{suffix}"] = raw
    for name, raw in row_payloads.items():
        expected = WIDTH * (2 if name.endswith(".bf16") else 4)
        if len(raw) != expected:
            raise Failure(f"residual retained payload {name} extent drifted")
    if retained.keys() & row_payloads.keys():
        raise Failure("residual replay output filename collision")
    retained.update(row_payloads)
    torch_passes = passes["torch_bf16_linear"]
    source_passes = passes["source_ascending_fp32"]
    native_host = passes["native_partial_host"]
    native_bf16_first = passes["native_partial_bf16_first"]
    comparisons = {
        "native_partial_vs_torch_widened": [
            f32_metrics(native["projection_partial"], value["partial_f32"])
            for value in torch_passes
        ],
        "native_partial_vs_source_ordered": [
            f32_metrics(native["projection_partial"], value["partial_f32"])
            for value in source_passes
        ],
        "native_hidden_vs_torch": [
            reference.row_metrics(native["hidden_after_broadcast"], value["hidden_bf16"], WIDTH)
            for value in torch_passes
        ],
        "native_hidden_vs_source_ordered": [
            reference.row_metrics(native["hidden_after_broadcast"], value["hidden_bf16"], WIDTH)
            for value in source_passes
        ],
        "native_hidden_vs_native_partial_host": [
            reference.row_metrics(native["hidden_after_broadcast"], value["hidden_bf16"], WIDTH)
            for value in native_host
        ],
        "native_hidden_vs_native_partial_bf16_first": [
            reference.row_metrics(native["hidden_after_broadcast"], value["hidden_bf16"], WIDTH)
            for value in native_bf16_first
        ],
        "native_partial_host_vs_bf16_first_hidden": [
            reference.row_metrics(
                native_host[index]["hidden_bf16"], native_bf16_first[index]["hidden_bf16"], WIDTH
            )
            for index in range(2)
        ],
        "native_partial_bf16_first_vs_torch_hidden": [
            reference.row_metrics(
                native_bf16_first[index]["hidden_bf16"], torch_passes[index]["hidden_bf16"], WIDTH
            )
            for index in range(2)
        ],
        "native_projection_bf16_first_vs_torch": [
            reference.row_metrics(
                native_bf16_first[index]["projection_bf16"], torch_passes[index]["projection_bf16"], WIDTH
            )
            for index in range(2)
        ],
        "torch_vs_source_hidden": [
            reference.row_metrics(
                torch_passes[index]["hidden_bf16"],
                source_passes[index]["hidden_bf16"],
                WIDTH,
            )
            for index in range(2)
        ],
        "torch_vs_source_partial": [
            f32_metrics(
                torch_passes[index]["partial_f32"],
                source_passes[index]["partial_f32"],
            )
            for index in range(2)
        ],
        "torch_repeat_partial": f32_metrics(
            torch_passes[0]["partial_f32"], torch_passes[1]["partial_f32"]
        ),
        "source_repeat_partial": f32_metrics(
            source_passes[0]["partial_f32"], source_passes[1]["partial_f32"]
        ),
        "native_partial_host_repeat_sum": f32_metrics(
            native_host[0]["sum_f32"], native_host[1]["sum_f32"]
        ),
        "torch_repeat_hidden": reference.row_metrics(
            torch_passes[0]["hidden_bf16"], torch_passes[1]["hidden_bf16"], WIDTH
        ),
        "source_repeat_hidden": reference.row_metrics(
            source_passes[0]["hidden_bf16"], source_passes[1]["hidden_bf16"], WIDTH
        ),
        "native_partial_host_repeat_hidden": reference.row_metrics(
            native_host[0]["hidden_bf16"], native_host[1]["hidden_bf16"], WIDTH
        ),
        "native_partial_bf16_first_repeat_partial": f32_metrics(
            native_bf16_first[0]["partial_f32"], native_bf16_first[1]["partial_f32"]
        ),
        "native_partial_bf16_first_repeat_hidden": reference.row_metrics(
            native_bf16_first[0]["hidden_bf16"], native_bf16_first[1]["hidden_bf16"], WIDTH
        ),
    }
    repeat = {
        case: {field: values[0][field] == values[1][field] for field in values[0]}
        for case, values in passes.items()
    }
    return {
        "position": row["position"],
        "input_token": row["input_token"],
        "capture_gpu_hidden_matches_host_broadcast": row["gpu_hidden_matches_host_broadcast"],
        "repeat_agreement": repeat,
        "comparisons": comparisons,
        "payloads": {
            name: {"bytes": len(data), "sha256": digest(data)}
            for name, data in sorted(row_payloads.items())
        },
    }


def replay_rows(payloads: dict[str, bytes], manifest: dict, weight, torch):
    rows = native_rows(payloads, manifest)
    if not 1 <= len(rows) <= 8:
        raise Failure("residual replay permits one to eight selected rows")
    weights_by_inner = prepare_source_weights(weight, torch)
    records = []
    retained: dict[str, bytes] = {}
    for row in rows:
        projection_input = bf16_row(row["projection_input"], WIDTH, torch, "projection input")
        require_finite(projection_input, torch, "projection input")
        require_finite(
            bf16_row(row["residual_before"], WIDTH, torch, "residual before"),
            torch,
            "residual before",
        )
        torch_passes = []
        source_passes = []
        native_passes = []
        native_bf16_first_passes = []
        for _ in range(2):
            torch_bf16 = torch_linear_once(projection_input, weight, torch)
            torch_partial = tensor_bytes(
                bf16_row(torch_bf16, WIDTH, torch, "Torch projection").to(torch.float32),
                torch,
            )
            torch_sum, torch_hidden = host_residual_once(torch_partial, row["residual_before"], torch)
            torch_passes.append({"projection_bf16": torch_bf16, "partial_f32": torch_partial,
                                 "sum_f32": torch_sum, "hidden_bf16": torch_hidden})
            source_partial, source_bf16 = ascending_fp32_once(
                projection_input, weights_by_inner, torch
            )
            source_sum, source_hidden = host_residual_once(
                source_partial, row["residual_before"], torch
            )
            source_passes.append({"projection_bf16": source_bf16, "partial_f32": source_partial,
                                  "sum_f32": source_sum, "hidden_bf16": source_hidden})
            native_sum, native_hidden = host_residual_once(
                row["projection_partial"], row["residual_before"], torch
            )
            native_passes.append({"sum_f32": native_sum, "hidden_bf16": native_hidden})
            native_bf16_first_passes.append(
                projection_rounded_residual_once(
                    row["projection_partial"], row["residual_before"], torch
                )
            )
        records.append(
            record_row(
                row,
                {key: row[key] for key, _, _, _ in PAYLOADS},
                {"torch_bf16_linear": torch_passes,
                 "source_ascending_fp32": source_passes,
                 "native_partial_host": native_passes,
                 "native_partial_bf16_first": native_bf16_first_passes},
                retained,
            )
        )
    return records, retained


def result_document(
    manifest: dict,
    records: list[dict],
    retained: dict[str, bytes],
    weight_descriptor: dict,
    source_hashes: dict[str, str],
    pin_bytes: bytes,
    native_payloads: dict[str, bytes],
    witness: dict[str, bytes],
    versions: dict,
    qwen_identity: dict,
) -> dict:
    repeat = [
        agreement
        for record in records
        for case in record["repeat_agreement"].values()
        for agreement in case.values()
    ]
    return {
        "schema": "FerricTp1SameInputLayer0ResidualDiagnosticV1",
        "authority": "none",
        "qualification": False,
        "benchmark_comparable": False,
        "numerical_pass_claimed": False,
        "tolerance_reviewed": False,
        "cause_established": False,
        "device_behavior_established": False,
        "compiler_behavior_established": False,
        "full_model_execution": False,
        "gpu_execution": False,
        "native_target": manifest["setup"]["target"],
        "model": {
            "repository": core.PINNED_REPOSITORY,
            "revision": core.PINNED_REVISION,
            "identity": core.PINNED_MODEL_IDENTITY,
        },
        "weight": weight_descriptor,
        "projection_kernel": {
            "source": PROJECTION_SOURCE,
            "source_sha256": PROJECTION_SOURCE_SHA256,
            "symbol": PROJECTION_SYMBOL,
            "m": 1,
            "n": WIDTH,
            "k": WIDTH,
            "arithmetic": (
                "ascending inner index; separate BF16-to-FP32 multiply and FP32 add; "
                "native output remains FP32"
            ),
            "artifact_hsaco_id": manifest["setup"]["artifact_hsaco_id"],
            "artifact_manifest_id": manifest["setup"]["artifact_manifest_id"],
            "artifact_handoff_id": manifest["setup"]["artifact_handoff_id"],
        },
        "host_reducer": {
            "source": COLLECTIVE_SOURCE,
            "source_sha256": COLLECTIVE_SOURCE_SHA256,
            "symbol": COLLECTIVE_SYMBOL,
            "world_size": 1,
            "arithmetic": (
                "one FP32 partial plus BF16 residual widened to FP32; manual FP32-bit "
                "round-to-nearest-even narrowing to BF16"
            ),
        },
        "cpu_cases": {
            "pinned_torch_bf16_linear": (
                "torch.nn.functional.linear on held BF16 input and weight; BF16 result "
                "widens to FP32 before the source-spelled host residual reducer"
            ),
            "source_ascending_fp32": (
                "ascending inner dimension; explicit separate FP32 multiply and add; "
                "FP32 partial enters the source-spelled host residual reducer"
            ),
            "captured_native_partial_host": (
                "held native FP32 partial enters the same source-spelled host residual reducer"
            ),
            "captured_native_partial_bf16_first": (
                "the identical held native FP32 partial narrows through pinned Torch to BF16, "
                "then widens to FP32 before the same host residual reducer; only projection "
                "rounding placement varies from captured_native_partial_host, not accumulation"
            ),
            "passes_per_case": 2,
            "actual_runtime_unmeasured": True,
        },
        "finite_replay_boundary": {
            "projection_input_residual_and_weight_required_finite": True,
            "cpu_and_native_outputs_may_be_nonfinite_and_are_retained": True,
            "finite_error_metrics_are_null_when_either_compared_row_is_nonfinite": True,
            "native_capture_is_separate_evidence": True,
        },
        "input": {
            "origin": "authenticated native TP1 layer-0 residual-boundary readback",
            "positions": manifest["positions"],
            "input_tokens": manifest["input_tokens"],
        },
        "rows": records,
        "repeat_agreement_all": all(repeat),
        "payloads": {
            name: {"bytes": len(data), "sha256": digest(data)}
            for name, data in sorted(retained.items())
        },
        "implementation_sha256": source_hashes,
        "qwen_implementation": qwen_identity,
        "dependencies": versions,
        "native_pins_sha256": digest(pin_bytes),
        "native_files_sha256": {
            name: digest(data) for name, data in sorted(native_payloads.items())
        },
        "native_witnesses_sha256": {
            name: digest(data) for name, data in sorted(witness.items())
        },
        "nonclaim": NONCLAIM,
    }


def run(arguments: list[str]) -> None:
    if len(arguments) != 6:
        raise Failure(
            "usage: engineering_tp1_layer0_residual_ablation.py "
            "CAPTURE WITNESS PINS PINS-SHA256 MODEL-SOURCE NEW-OUTPUT"
        )
    capture_path, witness_path, pins_path, pins_sha, model_path, output_path = arguments
    core.require_isolated_python()
    core.require_virtual_environment()
    source_hashes = implementation()
    with ExitStack() as held:
        pins_parent, pins_name = core.open_parent(Path(pins_path), "native pins")
        held.enter_context(pins_parent)
        output_parent, output_name = core.open_parent(
            Path(output_path), "layer-0 residual diagnostic output"
        )
        held.enter_context(output_parent)
        pins_file = held.enter_context(pins_parent.open_file(pins_name, "native pins"))
        forbidden = {pins_parent.identity}
        input_directories = {}
        for path in (
            Path(__file__).parent,
            Path(model_path),
            Path(model_path) / "target",
            Path(model_path) / "draft",
        ):
            directory = held.enter_context(
                core.SecureDirectory.open(path, "input/source directory")
            )
            forbidden.add(directory.identity)
            input_directories[path] = directory
        pin_bytes = pins_file.read(maximum=65536)
        pins = reference.validate_pins(pin_bytes, pins_sha)
        with reference.held_directory(
            Path(capture_path), CAPTURE_FILES, "native boundary capture", forbidden
        ) as native_payloads, reference.held_directory(
            Path(witness_path), reference.WITNESS_FILES, "native witnesses", forbidden
        ) as witness:
            reference.reject_output_alias(output_parent, forbidden)
            manifest = validate_capture(native_payloads, witness, pins)
            with core.authenticate_model_source(Path(model_path)) as model_source:
                expected_target = input_directories[Path(model_path) / "target"].identity
                if model_source.target.identity != expected_target:
                    raise Failure("authenticated model target directory changed")
                torch, safetensors, qwen, versions, qwen_identity = (
                    ablation.load_cpu_dependencies()
                )
                weight, weight_descriptor = load_projection_weight(
                    model_source, safetensors, torch
                )
                captured_tensor_sha = manifest["setup"]["residual_boundary_capture"][
                    "output_projection_weight_shard_sha256"
                ]
                if weight_descriptor["emitted_tensor_sha256"] != captured_tensor_sha:
                    raise Failure(
                        "authenticated o_proj tensor does not match captured tensor identity"
                    )
                records, retained = replay_rows(
                    native_payloads, manifest, weight, torch
                )
                model_source.validate()
            result = result_document(
                manifest,
                records,
                retained,
                weight_descriptor,
                source_hashes,
                pin_bytes,
                native_payloads,
                witness,
                versions,
                qwen_identity,
            )
        pins_file.validate()
        with pins_parent.open_file(pins_name, "reopened native pins") as reopened:
            if reopened.identity != pins_file.identity:
                raise Failure("native pin file identity drifted")
        with core.SecureDirectory.open(
            Path(pins_path).parent, "reopened native pins parent"
        ) as reopened:
            if reopened.identity != pins_parent.identity:
                raise Failure("native pin parent identity drifted")
        reference.equal(
            implementation(), source_hashes, "layer-0 residual diagnostic implementation"
        )
        if ablation.qwen_source_identity(qwen) != qwen_identity:
            raise Failure("pinned Qwen3 implementation changed during use")
        if torch.get_num_threads() != 1 or torch.get_num_interop_threads() != 1:
            raise Failure("CPU thread bounds changed")
        with core.SecureDirectory.open(
            Path(output_path).parent, "output parent binding"
        ) as reopened:
            if reopened.identity != output_parent.identity:
                raise Failure("output parent changed during layer-0 residual replay")
        reference.reject_output_alias(output_parent, forbidden)
        total = sum(map(len, retained.values()))
        if total > MAX_RAW_BYTES:
            raise Failure("layer-0 residual diagnostic raw payload bound exceeded")
        result_bytes = core.canonical_bytes(result)
        if not 0 < len(result_bytes) <= MAX_DOCUMENT_BYTES:
            raise Failure("layer-0 residual diagnostic record bound exceeded")
        os.mkdir(output_name, mode=0o700, dir_fd=output_parent.fd)
        with output_parent.child(
            output_name, "new same-input layer-0 residual diagnostic output"
        ) as output:
            for filename, data in sorted(retained.items()):
                core.write_new(
                    output.fd, filename, data, "same-input layer-0 residual diagnostic payload"
                )
            core.write_new(
                output.fd,
                "diagnostic.json",
                result_bytes,
                "same-input layer-0 residual diagnostic record",
            )
            os.fsync(output.fd)
        os.fsync(output_parent.fd)
    print(
        f"output={output_path} authority=none qualification=false "
        f"cause_established=false rows={len(records)} "
        f"repeat_agreement_all={str(result['repeat_agreement_all']).lower()}"
    )


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (
        Failure,
        ablation.Failure,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        AttributeError,
        OverflowError,
        ImportError,
        RuntimeError,
    ) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
