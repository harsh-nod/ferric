#!/usr/bin/env python3
"""CPU-only checkpoint-row RMSNorm ablation; no model forward or GPU execution."""

from __future__ import annotations

import importlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import sys


specification = importlib.util.spec_from_file_location(
    "ferric_rmsnorm_reference_core", Path(__file__).with_name("run.py")
)
if specification is None or specification.loader is None:
    raise ImportError("cannot load sibling reference core")
core = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = core
specification.loader.exec_module(core)
Failure = core.ReferenceFailure

WIDTH = 4096
TOKEN = 13
EPSILON = 1e-6
SHARD = "model-00001-of-00005.safetensors"
EMBEDDING = "model.embed_tokens.weight"
WEIGHT = "model.layers.0.input_layernorm.weight"
QWEN_SOURCE_SHA256 = "704c914530530a1acb0b443add1f520404e3ac2c28c0ab7e16f80f86cfe8ccb2"


def implementation() -> dict:
    root = Path(__file__).resolve().parents[3]
    names = (
        "benches/m1/reference/engineering_rmsnorm_ablation.py",
        "benches/m1/reference/run.py", "benches/m1/reference/pyproject.toml",
        "benches/m1/reference/uv.lock", "device/qwen3-all-kernels-v1/src/rmsnorm.rs",
    )
    with core.SecureDirectory.open(root, "ablation source") as source:
        return {name: core.sha256_bytes(source.read(name, "implementation")) for name in names}


def qwen_source_identity(qwen) -> dict:
    path = Path(qwen.__file__).resolve(strict=True)
    core.require_under_prefix(path, Path(sys.prefix), "pinned Qwen3 implementation")
    digest = core.sha256_bytes(path.read_bytes())
    if digest != QWEN_SOURCE_SHA256:
        raise Failure("pinned Qwen3 RMSNorm implementation drifted")
    return {"path": str(path), "sha256": digest}


def load_cpu_dependencies():
    # Reuse the reference's pins/provenance checks, not its GPU-admitting loader.
    core.require_isolated_python()
    core.require_virtual_environment()
    if sys.version_info[:2] != (3, 12) or sys.byteorder != "little":
        raise Failure("ablation requires pinned Python 3.12 on a little-endian CPU")
    for name in ("ROCR_VISIBLE_DEVICES", "HIP_VISIBLE_DEVICES", "CUDA_VISIBLE_DEVICES",
                 "HSA_VISIBLE_DEVICES"):
        os.environ[name] = "-1"
    for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE", "HF_DATASETS_OFFLINE"):
        os.environ[name] = "1"
    os.environ["OMP_NUM_THREADS"] = "1"
    os.environ["MKL_NUM_THREADS"] = "1"
    names = ("accelerate", "numpy", "psutil", "safetensors", "tokenizers", "torch",
             "transformers", "triton-rocm")
    modules = {name: importlib.import_module("triton" if name == "triton-rocm" else name)
               for name in names}
    from importlib.metadata import version

    versions = {name: (version(name) if name == "triton-rocm" else modules[name].__version__)
                for name in names}
    for name, actual in versions.items():
        if actual != core.DEPENDENCY_VERSIONS[name]:
            raise Failure(f"pinned reference dependency {name} drifted")
    core.validate_dependency_provenance(modules)
    torch = modules["torch"]
    if not isinstance(torch.version.hip, str) or not torch.version.hip.startswith("7.2"):
        raise Failure("pinned torch build does not report ROCm 7.2")
    torch.set_num_threads(1)
    torch.set_num_interop_threads(1)
    torch.set_default_device("cpu")
    qwen = importlib.import_module("transformers.models.qwen3.modeling_qwen3")
    identity = qwen_source_identity(qwen)
    return torch, modules["safetensors"], qwen, dict(python="3.12", **versions), identity


def tensor_bytes(tensor, torch) -> bytes:
    if tensor.device.type != "cpu" or tensor.dtype not in (torch.bfloat16, torch.float32):
        raise Failure("only CPU BF16/FP32 tensors can be recorded")
    if not bool(torch.isfinite(tensor).all()):
        raise Failure("nonfinite tensor in ablation")
    return tensor.detach().reshape(-1).contiguous().view(torch.uint8).numpy().tobytes()


def scalar_bits(value, torch) -> str:
    if value.dtype != torch.float32 or value.numel() != 1:
        raise Failure("intermediate scalar is not FP32")
    return f"0x{struct.unpack('<I', tensor_bytes(value, torch))[0]:08x}"


def validate_inputs(x, weight, torch) -> None:
    if (x.ndim != 2 or x.shape[0] != 1 or x.shape[1] not in (128, 1024, WIDTH)
            or tuple(weight.shape) != (x.shape[1],)
            or x.dtype != torch.bfloat16 or weight.dtype != torch.bfloat16):
        raise Failure("RMS inputs must be one admitted BF16 row and matching BF16 weight")
    tensor_bytes(x, torch)
    tensor_bytes(weight, torch)


def load_inputs(source, safetensors, torch):
    index = json.loads(source.files["model.safetensors.index.json"].read(),
                       object_pairs_hook=core._unique_object)
    if any(index["weight_map"].get(name) != SHARD for name in (EMBEDDING, WEIGHT)):
        raise Failure("checkpoint tensor-to-shard mapping drifted")
    config = json.loads(source.files["config.json"].read(), object_pairs_hook=core._unique_object)
    if config.get("rms_norm_eps") != EPSILON:
        raise Failure("checkpoint RMSNorm epsilon drifted")
    # The unchanged full-model authenticator has already hashed this held inode.
    with safetensors.safe_open(f"/proc/self/fd/{source.files[SHARD].fd}",
                               framework="pt", device="cpu") as shard:
        embedding = shard.get_slice(EMBEDDING)
        if embedding.get_shape() != [core.VOCABULARY_SIZE, WIDTH]:
            raise Failure("checkpoint embedding shape drifted")
        x = embedding[TOKEN:TOKEN + 1, :].clone()
        weight = shard.get_tensor(WEIGHT).clone()
    validate_inputs(x, weight, torch)
    if tuple(x.shape) != (1, WIDTH):
        raise Failure("checkpoint row width drifted")
    source.validate()
    return x, weight


def serial_square_sum(x, torch):
    values = x.to(torch.float32).reshape(-1)
    total = torch.zeros((), dtype=torch.float32, device="cpu")
    for column in range(values.numel()):
        square = values[column] * values[column]
        total = total + square
    scalar_bits(total, torch)
    return total


def rounded_weighting(normalized, weight, torch):
    rounded = normalized.to(torch.bfloat16)
    weighted = rounded.to(torch.float32) * weight.to(torch.float32)
    output = weighted.to(torch.bfloat16)
    for tensor in (normalized, rounded, weighted, output):
        tensor_bytes(tensor, torch)
    return rounded, weighted, output


def compare_outputs(left: bytes, right: bytes) -> dict:
    if len(left) != len(right) or len(left) % 2:
        raise Failure("BF16 comparison extent drifted")
    differences = [dict(index=index, left_bits=f"0x{a:04x}", right_bits=f"0x{b:04x}")
                   for index, ((a,), (b,)) in enumerate(zip(
                       struct.iter_unpack("<H", left), struct.iter_unpack("<H", right))) if a != b]
    return {"elements": len(left) // 2, "bit_mismatches": len(differences),
            "differences": differences}


def ablate(x, weight, torch, qwen):
    validate_inputs(x, weight, torch)
    payloads = {"input.bf16": tensor_bytes(x, torch), "weight.bf16": tensor_bytes(weight, torch)}
    cases = {}
    with torch.inference_mode():
        norm = qwen.Qwen3RMSNorm(x.shape[1], eps=EPSILON).to(device="cpu", dtype=torch.bfloat16)
        norm.weight.copy_(weight)
        actual_hf = norm(x)
        hf_bytes = tensor_bytes(actual_hf, torch)
        payloads["hf.output.bf16"] = hf_bytes
        serial_sum = serial_square_sum(x, torch)
        hf_mean = x.to(torch.float32).pow(2).mean(-1, keepdim=True)
        serial_mean = serial_sum / x.shape[1]
        for name, mean in (("hf", hf_mean), ("serial-rsqrt", serial_mean),
                           ("serial-sqrt-reciprocal", serial_mean)):
            stabilized = mean + EPSILON
            if not bool((stabilized > 0).all()):
                raise Failure("RMS denominator is not positive")
            scalars = {"mean_square_bits": scalar_bits(mean, torch),
                       "stabilized_bits": scalar_bits(stabilized, torch)}
            if name == "serial-sqrt-reciprocal":
                denominator = torch.sqrt(stabilized)
                inverse = 1.0 / denominator
                scalars["denominator_bits"] = scalar_bits(denominator, torch)
            else:
                inverse = torch.rsqrt(stabilized)
            scalars["inverse_rms_bits"] = scalar_bits(inverse, torch)
            if name != "hf":
                scalars["serial_square_sum_bits"] = scalar_bits(serial_sum, torch)
            normalized = x.to(torch.float32) * inverse
            rounded, weighted, output = rounded_weighting(normalized, weight, torch)
            raw = tensor_bytes(output, torch)
            if name == "hf" and raw != hf_bytes:
                raise Failure("explicit pinned HF formula differs from actual HF RMSNorm")
            for suffix, tensor in (("normalized.f32", normalized), ("normalized.bf16", rounded),
                                   ("weighted.f32", weighted), ("output.bf16", output)):
                payloads[f"{name}.{suffix}"] = tensor_bytes(tensor, torch)
            cases[name] = scalars
    comparisons = {}
    for left, right in (("hf", "serial-rsqrt"), ("serial-rsqrt", "serial-sqrt-reciprocal"),
                        ("hf", "serial-sqrt-reciprocal")):
        comparisons[f"{left}_vs_{right}"] = compare_outputs(
            payloads[f"{left}.output.bf16"], payloads[f"{right}.output.bf16"])
    return cases, comparisons, payloads


def run(arguments: list[str]) -> None:
    core.require_isolated_python()
    if len(arguments) != 2:
        raise Failure("usage: engineering_rmsnorm_ablation.py MODEL-SOURCE NEW-OUTPUT-DIRECTORY")
    sources = implementation()
    torch, safetensors, qwen, versions, qwen_identity = load_cpu_dependencies()
    # Authentication deliberately hashes all canonical target files, not only shard 1.
    with core.authenticate_model_source(Path(arguments[0])) as source:
        x, weight = load_inputs(source, safetensors, torch)
        cases, comparisons, payloads = ablate(x, weight, torch, qwen)
        source.validate()
    if implementation() != sources or qwen_source_identity(qwen) != qwen_identity:
        raise Failure("ablation implementation changed during use")
    if torch.get_num_threads() != 1 or torch.get_num_interop_threads() != 1:
        raise Failure("CPU thread bounds changed")
    result = {
        "format": "FERRIC-ENGINEERING-RMSNORM-ABLATION-V1", "authority": "none",
        "qualification": False, "benchmark_comparable": False, "numerical_pass_claimed": False,
        "tolerance_reviewed": False, "device_causality_established": False,
        "full_model_execution": False, "gpu_execution": False, "cpu_threads": 1,
        "model": {"repository": core.PINNED_REPOSITORY, "revision": core.PINNED_REVISION,
                  "identity": core.PINNED_MODEL_IDENTITY},
        "authentication": {"scope": "unchanged full canonical target-file SHA256 authentication",
                           "bytes_hashed": sum(size for size, _ in core.MODEL_FILES.values()),
                           "files": {name: {"bytes": size, "sha256": digest}
                                     for name, (size, digest) in core.MODEL_FILES.items()}},
        "input": {"tensor": EMBEDDING, "token": TOKEN, "shape": [1, WIDTH], "dtype": "bf16-le",
                  "shard": SHARD, "origin": "checkpoint embedding slice, not a captured native activation"},
        "weight": {"tensor": WEIGHT, "shape": [WIDTH], "dtype": "bf16-le", "shard": SHARD},
        "epsilon_f32_bits": f"0x{struct.unpack('<I', struct.pack('<f', EPSILON))[0]:08x}",
        "implementation_sha256": sources, "qwen_implementation": qwen_identity,
        "dependencies": versions, "cases": cases, "comparisons": comparisons,
        "payloads": {name: {"bytes": len(data), "sha256": core.sha256_bytes(data)}
                     for name, data in payloads.items()},
        "scope": "Actual pinned HF RMSNorm versus sequential FP32 sum with rsqrt, then only "
                 "sqrt plus reciprocal instead of rsqrt. All cases retain both BF16 boundaries. "
                 "HF intermediates are from its explicit formula, checked against actual HF output.",
        "nonclaim": "CPU reduction and transcendental behavior do not establish device behavior. "
                    "This first-layer checkpoint row does not localize the M5 position131 mismatch "
                    "or reproduce its final hidden state. No kernel change or tolerance is justified here.",
    }
    parent, name = core.open_parent(Path(arguments[1]), "ablation output")
    with parent:
        os.mkdir(name, mode=0o700, dir_fd=parent.fd)
        with parent.child(name, "new ablation output") as output:
            for filename, data in payloads.items():
                core.write_new(output.fd, filename, data, "ablation tensor")
            core.write_new(output.fd, "ablation.json", core.canonical_bytes(result), "ablation record")
    print(f"output={arguments[1]} gpu_execution=false device_causality_established=false")


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (Failure, OSError, ValueError, KeyError, TypeError, ImportError, RuntimeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
