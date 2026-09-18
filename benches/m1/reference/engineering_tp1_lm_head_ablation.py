#!/usr/bin/env python3
"""CPU-only same-input LM-head diagnostic for authenticated TP1 readbacks."""

from __future__ import annotations

from contextlib import ExitStack
import importlib.util
import json
import os
from pathlib import Path
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
    "ferric_tp1_lm_head_input", "engineering_tp1_final_stage_reference.py"
)
ablation = load_sibling(
    "ferric_tp1_lm_head_dependencies", "engineering_rmsnorm_ablation.py"
)
core = reference.core
Failure = reference.Failure
digest = core.sha256_bytes

WIDTH = 4096
VOCABULARY = core.VOCABULARY_SIZE
INDEX = "model.safetensors.index.json"
CONFIG = "config.json"
WEIGHT = "lm_head.weight"
WEIGHT_SHARD = "model-00005-of-00005.safetensors"
WEIGHT_BYTES = VOCABULARY * WIDTH * 2
WEIGHT_OFFSETS = [0, WEIGHT_BYTES]
GEMM_SOURCE = "device/qwen3-all-kernels-v1/src/gemm.rs"
GEMM_SOURCE_SHA256 = "f2af804475ab920b262f1a0c782cdc817f002f06e7dbdca1d78679698332ef07"
LM_HEAD_SYMBOL = "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1"
MAX_RAW_BYTES = 8 * 1024 * 1024
MAX_DOCUMENT_BYTES = 4 * 1024 * 1024
NONCLAIM = (
    "CPU same-input LM-head localization diagnostic only. This does not establish "
    "device or compiler behavior, a numerical tolerance, an accepted result, a causal "
    "defect, qualification authority, protected proof, performance, full-model "
    "execution, or any M1 gate closure."
)


def implementation() -> dict[str, str]:
    root = Path(__file__).resolve().parents[3]
    names = (
        "benches/m1/reference/engineering_tp1_lm_head_ablation.py",
        "benches/m1/reference/engineering_tp1_final_stage_reference.py",
        "benches/m1/reference/engineering_rmsnorm_ablation.py",
        "benches/m1/reference/run.py",
        "benches/m1/reference/pyproject.toml",
        "benches/m1/reference/uv.lock",
        GEMM_SOURCE,
    )
    with core.SecureDirectory.open(root, "same-input LM-head diagnostic source") as source:
        hashes = {
            name: digest(
                source.read(name, "diagnostic implementation", maximum=2 * 1024 * 1024)
            )
            for name in names
        }
    if hashes[GEMM_SOURCE] != GEMM_SOURCE_SHA256:
        raise Failure("source-spelled LM-head implementation drifted")
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
        raise Failure("LM-head weight-to-shard mapping drifted")
    if "lm_head.bias" in index["weight_map"]:
        raise Failure("checkpoint unexpectedly contains an LM-head bias")
    if type(config) is not dict or config.get("tie_word_embeddings") is not False:
        raise Failure("checkpoint LM-head tying contract drifted")


def tensor_bytes(tensor, torch) -> bytes:
    if tensor.device.type != "cpu" or tensor.dtype not in (
        torch.bfloat16,
        torch.float32,
    ):
        raise Failure("only CPU BF16/FP32 tensors can be recorded")
    return (
        tensor.detach()
        .reshape(-1)
        .contiguous()
        .view(torch.uint8)
        .numpy()
        .tobytes()
    )


def require_finite(tensor, torch, label: str) -> None:
    if not bool(torch.isfinite(tensor).all()):
        raise Failure(f"nonfinite {label} is outside the replay boundary")


def load_head_weight(source, safetensors, torch):
    parse_weight_binding(
        source.files[INDEX].read(exact=core.MODEL_FILES[INDEX][0]),
        source.files[CONFIG].read(exact=core.MODEL_FILES[CONFIG][0]),
    )
    with safetensors.safe_open(
        f"/proc/self/fd/{source.files[WEIGHT_SHARD].fd}",
        framework="pt",
        device="cpu",
    ) as shard:
        if shard.get_slice(WEIGHT).get_shape() != [VOCABULARY, WIDTH]:
            raise Failure("LM-head weight geometry drifted")
        weight = shard.get_tensor(WEIGHT).clone()
    if (
        tuple(weight.shape) != (VOCABULARY, WIDTH)
        or weight.dtype != torch.bfloat16
        or weight.device.type != "cpu"
        or not weight.is_contiguous()
        or tuple(weight.stride()) != (WIDTH, 1)
    ):
        raise Failure("LM-head weight shape, dtype, device, or layout drifted")
    require_finite(weight, torch, "LM-head weight")
    if weight.numel() * weight.element_size() != WEIGHT_BYTES:
        raise Failure("LM-head weight byte extent drifted")
    source.validate()
    descriptor = {
        "tensor": WEIGHT,
        "shard": WEIGHT_SHARD,
        "shape": [VOCABULARY, WIDTH],
        "dtype": "bf16-little-endian",
        "data_offsets": WEIGHT_OFFSETS,
        "tensor_bytes": WEIGHT_BYTES,
        "shard_bytes": core.MODEL_FILES[WEIGHT_SHARD][0],
        "shard_sha256": core.MODEL_FILES[WEIGHT_SHARD][1],
    }
    return weight, descriptor


def native_rows(payloads: dict[str, bytes], manifest: dict) -> list[dict]:
    selected = manifest["positions"]
    normalized = payloads["normalized.bf16"]
    logits = payloads["logits.bf16"]
    if len(normalized) != len(selected) * WIDTH * 2:
        raise Failure("native normalized payload extent drifted")
    if len(logits) != len(selected) * VOCABULARY * 2:
        raise Failure("native logit payload extent drifted")
    rows = []
    for ordinal, position in enumerate(selected):
        norm_span = slice(ordinal * WIDTH * 2, (ordinal + 1) * WIDTH * 2)
        logit_span = slice(
            ordinal * VOCABULARY * 2, (ordinal + 1) * VOCABULARY * 2
        )
        rows.append(
            {
                "ordinal": ordinal,
                "position": position,
                "input_token": manifest["input_tokens"][position],
                "gpu_choice": manifest["gpu_choices"][position],
                "normalized": normalized[norm_span],
                "logits": logits[logit_span],
            }
        )
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
    vocabulary, _ = validate_linear_inputs(normalized, weight, torch)
    with torch.inference_mode():
        output = torch.nn.functional.linear(normalized, weight, None)
    if (
        tuple(output.shape) != (1, vocabulary)
        or output.dtype != torch.bfloat16
        or output.device.type != "cpu"
    ):
        raise Failure("pinned Torch BF16 linear output geometry drifted")
    return tensor_bytes(output, torch)


def prepare_source_weights(weight, torch):
    if (
        tuple(weight.shape) != (VOCABULARY, WIDTH)
        or weight.dtype != torch.bfloat16
        or weight.device.type != "cpu"
    ):
        raise Failure("source-spelled replay weight geometry drifted")
    require_finite(weight, torch, "LM-head weight")
    by_inner = torch.empty((WIDTH, VOCABULARY), dtype=torch.float32, device="cpu")
    by_inner.copy_(weight.transpose(0, 1))
    if not by_inner.is_contiguous() or tuple(by_inner.stride()) != (VOCABULARY, 1):
        raise Failure("source-spelled FP32 weight workspace layout drifted")
    require_finite(by_inner, torch, "FP32 LM-head weight workspace")
    return by_inner


def ascending_fp32_once(normalized, weights_by_inner, torch) -> tuple[bytes, bytes]:
    if (
        normalized.ndim != 2
        or normalized.shape[0] != 1
        or weights_by_inner.ndim != 2
        or weights_by_inner.shape[0] != normalized.shape[1]
        or normalized.dtype != torch.bfloat16
        or weights_by_inner.dtype != torch.float32
        or normalized.device.type != "cpu"
        or weights_by_inner.device.type != "cpu"
        or not weights_by_inner.is_contiguous()
    ):
        raise Failure("source-spelled LM-head inputs drifted")
    require_finite(normalized, torch, "normalized input")
    require_finite(weights_by_inner, torch, "FP32 LM-head weight workspace")
    inner = int(normalized.shape[1])
    vocabulary = int(weights_by_inner.shape[1])
    activation = normalized.reshape(inner).to(torch.float32)
    accumulator = torch.zeros(vocabulary, dtype=torch.float32, device="cpu")
    product = torch.empty(vocabulary, dtype=torch.float32, device="cpu")
    with torch.inference_mode():
        for column in range(inner):
            torch.mul(activation[column], weights_by_inner[column], out=product)
            torch.add(accumulator, product, out=accumulator)
        rounded = accumulator.to(torch.bfloat16)
    if tuple(rounded.shape) != (vocabulary,) or rounded.dtype != torch.bfloat16:
        raise Failure("source-spelled LM-head output geometry drifted")
    # Outputs are retained even when arithmetic overflow creates nonfinite values.
    return tensor_bytes(accumulator, torch), tensor_bytes(rounded, torch)


def record_row(row: dict, passes: dict[str, list], retained: dict[str, bytes]) -> dict:
    if set(passes) != {"torch_bf16_linear", "source_ascending_fp32"}:
        raise Failure("LM-head replay case roster drifted")
    torch_passes = passes["torch_bf16_linear"]
    source_passes = passes["source_ascending_fp32"]
    if len(torch_passes) != 2 or len(source_passes) != 2:
        raise Failure("each LM-head replay case must retain exactly two passes")
    prefix = f"position-{row['position']:06d}"
    row_payloads = {
        f"{prefix}.native-normalized.bf16": row["normalized"],
        f"{prefix}.native-logits.bf16": row["logits"],
    }
    for number, raw in enumerate(torch_passes, 1):
        row_payloads[f"{prefix}.torch-pass{number}-logits.bf16"] = raw
    for number, (fp32, bf16) in enumerate(source_passes, 1):
        row_payloads[f"{prefix}.source-pass{number}-logits.f32"] = fp32
        row_payloads[f"{prefix}.source-pass{number}-logits.bf16"] = bf16
    expected_extents = {
        f"{prefix}.native-normalized.bf16": WIDTH * 2,
        f"{prefix}.native-logits.bf16": VOCABULARY * 2,
        **{
            f"{prefix}.torch-pass{number}-logits.bf16": VOCABULARY * 2
            for number in (1, 2)
        },
        **{
            f"{prefix}.source-pass{number}-logits.bf16": VOCABULARY * 2
            for number in (1, 2)
        },
        **{
            f"{prefix}.source-pass{number}-logits.f32": VOCABULARY * 4
            for number in (1, 2)
        },
    }
    if set(row_payloads) != set(expected_extents):
        raise Failure("LM-head retained row roster drifted")
    for name, expected in expected_extents.items():
        if len(row_payloads[name]) != expected:
            raise Failure(f"LM-head retained payload {name} extent drifted")
    if retained.keys() & row_payloads.keys():
        raise Failure("LM-head replay output filename collision")
    retained.update(row_payloads)
    torch_bf16 = torch_passes
    source_bf16 = [value[1] for value in source_passes]
    source_fp32 = [value[0] for value in source_passes]
    comparisons = {
        "native_vs_torch_pass1": reference.row_metrics(
            row["logits"], torch_bf16[0], VOCABULARY
        ),
        "native_vs_torch_pass2": reference.row_metrics(
            row["logits"], torch_bf16[1], VOCABULARY
        ),
        "native_vs_source_pass1": reference.row_metrics(
            row["logits"], source_bf16[0], VOCABULARY
        ),
        "native_vs_source_pass2": reference.row_metrics(
            row["logits"], source_bf16[1], VOCABULARY
        ),
        "torch_pass1_vs_source_pass1": reference.row_metrics(
            torch_bf16[0], source_bf16[0], VOCABULARY
        ),
        "torch_pass1_vs_pass2": reference.row_metrics(
            torch_bf16[0], torch_bf16[1], VOCABULARY
        ),
        "source_pass1_vs_pass2": reference.row_metrics(
            source_bf16[0], source_bf16[1], VOCABULARY
        ),
    }
    repeat = {
        "torch_bf16_logits_byte_identical": torch_bf16[0] == torch_bf16[1],
        "source_bf16_logits_byte_identical": source_bf16[0] == source_bf16[1],
        "source_fp32_accumulators_byte_identical": source_fp32[0] == source_fp32[1],
    }
    return {
        "position": row["position"],
        "input_token": row["input_token"],
        "gpu_choice": row["gpu_choice"],
        "repeat_agreement": repeat,
        "comparisons": comparisons,
        "payloads": {
            name: {"bytes": len(data), "sha256": digest(data)}
            for name, data in sorted(row_payloads.items())
        },
    }


def replay_rows(payloads: dict[str, bytes], manifest: dict, weight, torch):
    rows = native_rows(payloads, manifest)
    if not 1 <= len(rows) <= 2:
        raise Failure("LM-head replay permits at most two selected rows")
    weights_by_inner = prepare_source_weights(weight, torch)
    records = []
    retained: dict[str, bytes] = {}
    for row in rows:
        normalized = bf16_row(row["normalized"], WIDTH, torch, "native normalized row")
        require_finite(normalized, torch, "native normalized row")
        torch_passes = [torch_linear_once(normalized, weight, torch) for _ in range(2)]
        source_passes = [
            ascending_fp32_once(normalized, weights_by_inner, torch) for _ in range(2)
        ]
        records.append(
            record_row(
                row,
                {
                    "torch_bf16_linear": torch_passes,
                    "source_ascending_fp32": source_passes,
                },
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
        for agreement in record["repeat_agreement"].values()
    ]
    return {
        "schema": "FerricTp1SameInputLmHeadDiagnosticV1",
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
        "kernel": {
            "source": GEMM_SOURCE,
            "source_sha256": GEMM_SOURCE_SHA256,
            "symbol": LM_HEAD_SYMBOL,
            "m": 1,
            "n": VOCABULARY,
            "k": WIDTH,
            "beta_f32": 0.0,
            "arithmetic": (
                "ascending BF16-to-FP32 multiply then FP32 add; "
                "one final BF16 narrowing"
            ),
            "artifact_hsaco_id": manifest["setup"]["artifact_hsaco_id"],
            "artifact_manifest_id": manifest["setup"]["artifact_manifest_id"],
            "artifact_handoff_id": manifest["setup"]["artifact_handoff_id"],
        },
        "cpu_cases": {
            "pinned_torch_bf16_linear": "torch.nn.functional.linear on held BF16 input and weight",
            "source_ascending_fp32": (
                "ascending inner dimension; explicit separate FP32 multiply and add; "
                "final BF16 narrowing"
            ),
            "passes_per_case": 2,
            "actual_runtime_unmeasured": True,
        },
        "finite_replay_boundary": {
            "normalized_input_and_weight_required_finite": True,
            "cpu_outputs_and_native_logits_may_be_nonfinite_and_are_retained": True,
            "finite_error_metrics_are_null_when_either_BF16_row_is_nonfinite": True,
            "native_capture_is_separate_evidence": True,
        },
        "input": {
            "origin": "authenticated native TP1 final-stage normalized readback",
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
            "usage: engineering_tp1_lm_head_ablation.py "
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
            Path(output_path), "LM-head diagnostic output"
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
                torch, safetensors, qwen, versions, qwen_identity = (
                    ablation.load_cpu_dependencies()
                )
                weight, weight_descriptor = load_head_weight(
                    model_source, safetensors, torch
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
            implementation(), source_hashes, "LM-head diagnostic implementation"
        )
        if ablation.qwen_source_identity(qwen) != qwen_identity:
            raise Failure("pinned Qwen3 implementation changed during use")
        if torch.get_num_threads() != 1 or torch.get_num_interop_threads() != 1:
            raise Failure("CPU thread bounds changed")
        with core.SecureDirectory.open(
            Path(output_path).parent, "output parent binding"
        ) as reopened:
            if reopened.identity != output_parent.identity:
                raise Failure("output parent changed during LM-head replay")
        reference.reject_output_alias(output_parent, forbidden)
        total = sum(map(len, retained.values()))
        if total > MAX_RAW_BYTES:
            raise Failure("LM-head diagnostic raw payload bound exceeded")
        result_bytes = core.canonical_bytes(result)
        if not 0 < len(result_bytes) <= MAX_DOCUMENT_BYTES:
            raise Failure("LM-head diagnostic record bound exceeded")
        os.mkdir(output_name, mode=0o700, dir_fd=output_parent.fd)
        with output_parent.child(
            output_name, "new same-input LM-head diagnostic output"
        ) as output:
            for filename, data in sorted(retained.items()):
                core.write_new(
                    output.fd, filename, data, "same-input LM-head diagnostic payload"
                )
            core.write_new(
                output.fd,
                "diagnostic.json",
                result_bytes,
                "same-input LM-head diagnostic record",
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
