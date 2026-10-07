#!/usr/bin/env python3
"""CPU-only same-input final-RMS diagnostic for authenticated TP1 readbacks."""

from __future__ import annotations

from contextlib import ExitStack
import importlib.util
import json
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
    "ferric_tp1_final_stage_rms_input", "engineering_tp1_final_stage_reference.py"
)
ablation = load_sibling(
    "ferric_tp1_final_stage_rms_arithmetic", "engineering_rmsnorm_ablation.py"
)
core = reference.core
Failure = reference.Failure
digest = core.sha256_bytes

WIDTH = 4096
EPSILON = 1e-6
INDEX = "model.safetensors.index.json"
WEIGHT = "model.norm.weight"
WEIGHT_SHARD = "model-00004-of-00005.safetensors"
RMSNORM_SOURCE = "device/qwen3-all-kernels-v1/src/rmsnorm.rs"
RMSNORM_SOURCE_SHA256 = "905fa46c5cf6d0b9e605167a61774d9c3d93611fa551884b410c3998fd73908d"
MAX_RAW_BYTES = 4 * 1024 * 1024
MAX_DOCUMENT_BYTES = 16 * 1024 * 1024
SOURCE_CASE = "serial-sqrt-reciprocal"
SELECTED_ABLATION_PAYLOADS = {
    "hf.normalized.f32": "hf-normalized.f32",
    "hf.normalized.bf16": "hf-preweight.bf16",
    "hf.weighted.f32": "hf-weighted.f32",
    "hf.output.bf16": "hf-output.bf16",
    f"{SOURCE_CASE}.normalized.f32": "source-normalized.f32",
    f"{SOURCE_CASE}.normalized.bf16": "source-preweight.bf16",
    f"{SOURCE_CASE}.weighted.f32": "source-weighted.f32",
    f"{SOURCE_CASE}.output.bf16": "source-output.bf16",
}
NONCLAIM = (
    "CPU same-input localization diagnostic only. This does not establish device or "
    "compiler behavior, a numerical tolerance, an accepted result, a causal defect, "
    "qualification authority, protected proof, performance, or any M1 gate closure."
)


def implementation() -> dict[str, str]:
    root = Path(__file__).resolve().parents[3]
    names = (
        "benches/m1/reference/engineering_tp1_rmsnorm_ablation.py",
        "benches/m1/reference/engineering_tp1_final_stage_reference.py",
        "benches/m1/reference/engineering_rmsnorm_ablation.py",
        "benches/m1/reference/run.py",
        "benches/m1/reference/pyproject.toml",
        "benches/m1/reference/uv.lock",
        RMSNORM_SOURCE,
    )
    with core.SecureDirectory.open(root, "same-input RMS diagnostic source") as source:
        hashes = {
            name: digest(source.read(name, "diagnostic implementation", maximum=2 * 1024 * 1024))
            for name in names
        }
    if hashes[RMSNORM_SOURCE] != RMSNORM_SOURCE_SHA256:
        raise Failure("source-spelled RMSNorm implementation drifted")
    return hashes


def parse_weight_binding(index_bytes: bytes, config_bytes: bytes) -> None:
    index = json.loads(index_bytes, object_pairs_hook=core._unique_object,
                       parse_constant=core._reject_constant)
    config = json.loads(config_bytes, object_pairs_hook=core._unique_object,
                        parse_constant=core._reject_constant)
    if type(index) is not dict or type(index.get("weight_map")) is not dict:
        raise Failure("checkpoint weight map is malformed")
    if index["weight_map"].get(WEIGHT) != WEIGHT_SHARD:
        raise Failure("final RMS weight-to-shard mapping drifted")
    epsilon = config.get("rms_norm_eps") if type(config) is dict else None
    if type(epsilon) is not float or struct.pack("<f", epsilon) != struct.pack("<f", EPSILON):
        raise Failure("checkpoint final RMS epsilon drifted")


def load_final_weight(source, safetensors, torch):
    parse_weight_binding(
        source.files[INDEX].read(exact=core.MODEL_FILES[INDEX][0]),
        source.files["config.json"].read(exact=core.MODEL_FILES["config.json"][0]),
    )
    with safetensors.safe_open(
        f"/proc/self/fd/{source.files[WEIGHT_SHARD].fd}", framework="pt", device="cpu"
    ) as shard:
        if shard.get_slice(WEIGHT).get_shape() != [WIDTH]:
            raise Failure("final RMS weight geometry drifted")
        weight = shard.get_tensor(WEIGHT).clone()
    if tuple(weight.shape) != (WIDTH,) or weight.dtype != torch.bfloat16:
        raise Failure("final RMS weight shape or dtype drifted")
    raw = ablation.tensor_bytes(weight, torch)
    if len(raw) != WIDTH * 2:
        raise Failure("final RMS weight byte extent drifted")
    source.validate()
    return weight, raw


def native_rows(payloads: dict[str, bytes], manifest: dict) -> list[dict]:
    selected = manifest["positions"]
    residual = payloads["residual.bf16"]
    normalized = payloads["normalized.bf16"]
    expected = len(selected) * WIDTH * 2
    if len(residual) != expected or len(normalized) != expected:
        raise Failure("native final RMS payload extent drifted")
    rows = []
    for ordinal, position in enumerate(selected):
        span = slice(ordinal * WIDTH * 2, (ordinal + 1) * WIDTH * 2)
        rows.append({
            "ordinal": ordinal,
            "position": position,
            "input_token": manifest["input_tokens"][position],
            "gpu_choice": manifest["gpu_choices"][position],
            "residual": residual[span],
            "normalized": normalized[span],
        })
    return rows


def bf16_row(data: bytes, torch):
    if sys.byteorder != "little" or len(data) != WIDTH * 2:
        raise Failure("native residual is not one little-endian BF16 row")
    tensor = torch.frombuffer(bytearray(data), dtype=torch.bfloat16).reshape(1, WIDTH).clone()
    if ablation.tensor_bytes(tensor, torch) != data:
        raise Failure("native residual BF16 decoding changed its bytes")
    return tensor


def compare_normalized(left: bytes, right: bytes) -> dict:
    exact = ablation.compare_outputs(left, right)
    exact["metrics"] = reference.row_metrics(left, right, WIDTH)
    return exact


def record_row(row: dict, cases: dict, payloads: dict[str, bytes]) -> tuple[dict, dict[str, bytes]]:
    if set(cases) != {"hf", "serial-rsqrt", SOURCE_CASE}:
        raise Failure("RMS arithmetic case roster drifted")
    required = {"input.bf16", "weight.bf16"} | set(SELECTED_ABLATION_PAYLOADS)
    if not required <= set(payloads):
        raise Failure("RMS arithmetic payload roster is incomplete")
    if payloads["input.bf16"] != row["residual"]:
        raise Failure("RMS replay input differs from the held native residual")
    if len(payloads["weight.bf16"]) != WIDTH * 2:
        raise Failure("RMS replay weight extent drifted")
    for name in SELECTED_ABLATION_PAYLOADS:
        expected = WIDTH * (4 if name.endswith(".f32") else 2)
        if len(payloads[name]) != expected:
            raise Failure("RMS replay intermediate extent drifted")
    native = row["normalized"]
    hf = payloads["hf.output.bf16"]
    source = payloads[f"{SOURCE_CASE}.output.bf16"]
    if any(len(value) != WIDTH * 2 for value in (native, hf, source)):
        raise Failure("RMS replay output extent drifted")
    prefix = f"position-{row['position']:06d}"
    retained = {
        f"{prefix}.native-residual.bf16": row["residual"],
        f"{prefix}.native-normalized.bf16": native,
    }
    for source_name, output_name in SELECTED_ABLATION_PAYLOADS.items():
        retained[f"{prefix}.{output_name}"] = payloads[source_name]
    record = {
        "position": row["position"],
        "input_token": row["input_token"],
        "gpu_choice": row["gpu_choice"],
        "source_case": SOURCE_CASE,
        "hf_scalars": cases["hf"],
        "source_scalars": cases[SOURCE_CASE],
        "comparisons": {
            "native_normalized_vs_hf": compare_normalized(native, hf),
            "native_normalized_vs_source_serial_fp32": compare_normalized(native, source),
            "hf_vs_source_serial_fp32": compare_normalized(hf, source),
        },
        "payloads": {
            name: {"bytes": len(data), "sha256": digest(data)}
            for name, data in sorted(retained.items())
        },
    }
    return record, retained


def replay_rows(payloads: dict[str, bytes], manifest: dict, weight, weight_bytes: bytes,
                torch, qwen) -> tuple[list[dict], dict[str, bytes]]:
    records = []
    retained = {"final-rms-weight.bf16": weight_bytes}
    for row in native_rows(payloads, manifest):
        cases, _, arithmetic_payloads = ablation.ablate(
            bf16_row(row["residual"], torch), weight, torch, qwen
        )
        if arithmetic_payloads.get("weight.bf16") != weight_bytes:
            raise Failure("RMS replay weight differs from the authenticated final RMS weight")
        record, row_payloads = record_row(row, cases, arithmetic_payloads)
        if retained.keys() & row_payloads.keys():
            raise Failure("RMS replay output filename collision")
        retained.update(row_payloads)
        records.append(record)
    if not 1 <= len(records) <= 8:
        raise Failure("RMS replay row count drifted")
    return records, retained


def result_document(manifest: dict, records: list[dict], retained: dict[str, bytes],
                    source_hashes: dict[str, str], pin_bytes: bytes,
                    native_payloads: dict[str, bytes], witness: dict[str, bytes],
                    versions: dict, qwen_identity: dict) -> dict:
    return {
        "schema": "FerricTp1SameInputFinalRmsDiagnosticV1",
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
        "weight": {
            "tensor": WEIGHT,
            "shard": WEIGHT_SHARD,
            "shape": [WIDTH],
            "dtype": "bf16-little-endian",
            "bytes": len(retained["final-rms-weight.bf16"]),
            "sha256": digest(retained["final-rms-weight.bf16"]),
        },
        "epsilon_f32_bits": f"0x{struct.unpack('<I', struct.pack('<f', EPSILON))[0]:08x}",
        "arithmetic": {
            "hf": "actual pinned Qwen3RMSNorm CPU implementation and its explicit formula",
            "source_serial_fp32": "ascending FP32 square sum, divide by width, add epsilon, "
                                  "sqrt then reciprocal, BF16 pre-weight narrowing, FP32 "
                                  "BF16-weight "
                                  "multiply, BF16 output",
        },
        "finite_replay_boundary": {
            "residual_weight_and_cpu_intermediates_required_finite": True,
            "native_normalized_observation_may_be_nonfinite": True,
            "native_capture_is_separate_evidence": True,
        },
        "input": {
            "origin": "authenticated native TP1 final-stage residual readback",
            "positions": manifest["positions"],
            "input_tokens": manifest["input_tokens"],
        },
        "rows": records,
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
            "usage: engineering_tp1_rmsnorm_ablation.py "
            "CAPTURE WITNESS PINS PINS-SHA256 MODEL-SOURCE NEW-OUTPUT"
        )
    capture_path, witness_path, pins_path, pins_sha, model_path, output_path = arguments
    core.require_isolated_python()
    core.require_virtual_environment()
    source_hashes = implementation()
    with ExitStack() as held:
        pins_parent, pins_name = core.open_parent(Path(pins_path), "native pins")
        held.enter_context(pins_parent)
        output_parent, output_name = core.open_parent(Path(output_path), "RMS diagnostic output")
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
            Path(capture_path), reference.CAPTURE_FILES, "native capture", forbidden
        ) as native_payloads, reference.held_directory(
            Path(witness_path), reference.WITNESS_FILES, "native witnesses", forbidden
        ) as witness:
            reference.reject_output_alias(output_parent, forbidden)
            manifest = reference.validate_capture(native_payloads, witness, pins)
            with core.authenticate_model_source(Path(model_path)) as model_source:
                expected_target = input_directories[Path(model_path) / "target"].identity
                if model_source.target.identity != expected_target:
                    raise Failure("authenticated model target directory changed")
                torch, safetensors, qwen, versions, qwen_identity = ablation.load_cpu_dependencies()
                weight, weight_bytes = load_final_weight(model_source, safetensors, torch)
                records, retained = replay_rows(
                    native_payloads, manifest, weight, weight_bytes, torch, qwen
                )
                model_source.validate()
            result = result_document(
                manifest, records, retained, source_hashes, pin_bytes,
                native_payloads, witness, versions, qwen_identity,
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
        reference.equal(implementation(), source_hashes, "RMS diagnostic implementation")
        if ablation.qwen_source_identity(qwen) != qwen_identity:
            raise Failure("pinned Qwen3 RMSNorm implementation changed during use")
        if torch.get_num_threads() != 1 or torch.get_num_interop_threads() != 1:
            raise Failure("CPU thread bounds changed")
        with core.SecureDirectory.open(
            Path(output_path).parent, "output parent binding"
        ) as reopened:
            if reopened.identity != output_parent.identity:
                raise Failure("output parent changed during RMS replay")
        reference.reject_output_alias(output_parent, forbidden)
        total = sum(map(len, retained.values()))
        if total > MAX_RAW_BYTES:
            raise Failure("RMS diagnostic raw payload bound exceeded")
        result_bytes = core.canonical_bytes(result)
        if not 0 < len(result_bytes) <= MAX_DOCUMENT_BYTES:
            raise Failure("RMS diagnostic record bound exceeded")
        os.mkdir(output_name, mode=0o700, dir_fd=output_parent.fd)
        with output_parent.child(output_name, "new same-input RMS diagnostic output") as output:
            for filename, data in sorted(retained.items()):
                core.write_new(output.fd, filename, data, "same-input RMS diagnostic payload")
            core.write_new(
                output.fd, "diagnostic.json", result_bytes,
                "same-input RMS diagnostic record",
            )
            os.fsync(output.fd)
        os.fsync(output_parent.fd)
    print(
        f"output={output_path} authority=none qualification=false "
        f"cause_established=false rows={len(records)}"
    )


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (
        Failure, ablation.Failure, OSError, ValueError, KeyError, TypeError,
        AttributeError, OverflowError, ImportError, RuntimeError,
    ) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
