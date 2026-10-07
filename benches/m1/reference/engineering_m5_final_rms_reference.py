#!/usr/bin/env python3
"""Independent final-RMS row capture for the opt-in engineering M=5 diagnostic."""

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
    "ferric_m5_final_rms_reference", Path(__file__).with_name("engineering_m5_reference.py")
)
if specification is None or specification.loader is None:
    raise ImportError("cannot load sibling M=5 reference")
reference = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = reference
specification.loader.exec_module(reference)
core = reference.core
Failure = core.ReferenceFailure
WIDTH = 4096
ROW_BYTES = WIDTH * 2
LOGITS_BYTES = reference.ROWS * reference.ROW_BYTES
READBACK_BYTES = LOGITS_BYTES + 2 * reference.ROWS * ROW_BYTES
CAPTURE_FILES = {
    "capture.json": 1024 * 1024,
    "target-logits.bf16": LOGITS_BYTES,
    "final-rms-capture.json": 64 * 1024,
    "target-final-rms-input.bf16": ROW_BYTES,
    "target-final-rms-output.bf16": ROW_BYTES,
}


def decode_row(data: bytes) -> list[float]:
    if len(data) != ROW_BYTES:
        raise Failure("final-RMS row must contain exactly 4096 BF16 values")
    values = [struct.unpack("<f", struct.pack("<I", bits << 16))[0]
              for (bits,) in struct.iter_unpack("<H", data)]
    if not all(math.isfinite(value) for value in values):
        raise Failure("final-RMS row contains a nonfinite value")
    return values


def require_equal(actual, expected, label: str) -> None:
    if core.canonical_bytes(actual) != core.canonical_bytes(expected):
        raise Failure(f"final-RMS {label} drifted")


def validate_capture(payloads: dict[str, bytes]) -> tuple[list[int], dict]:
    if set(payloads) != set(CAPTURE_FILES):
        raise Failure("final-RMS capture has an unexpected file roster")
    capture = json.loads(payloads["capture.json"], object_pairs_hook=core._unique_object)
    sequence = reference.validate_capture(capture, payloads["target-logits.bf16"])
    require_equal(capture["smoke"]["speculative_k4"]["dispatch_generation"],
                  capture["dispatch_generation"], "nested smoke dispatch generation")
    require_equal(capture["shape"], [1, 5, core.VOCABULARY_SIZE], "logits shape")
    require_equal(capture["positions"], [128, 129, 130, 131, 132], "logits positions")
    require_equal(capture["logits"], {"path": "target-logits.bf16", "bytes": LOGITS_BYTES,
                                    "sha256": reference.digest(payloads["target-logits.bf16"])},
                  "logits descriptor")
    base = capture["rows"][0]["offset_bytes"]
    for index, row in enumerate(capture["rows"]):
        data = payloads["target-logits.bf16"][index * reference.ROW_BYTES:(index + 1) * reference.ROW_BYTES]
        require_equal(row, {
            "active_index": index, "position": 128 + index, "bytes": reference.ROW_BYTES,
            "offset_bytes": base + index * reference.ROW_BYTES, "sha256": reference.digest(data),
            "argmax_token": capture["smoke"]["speculative_k4"]["target_choices"][index],
        }, f"logits row {index}")
    document = json.loads(payloads["final-rms-capture.json"], object_pairs_hook=core._unique_object)
    expected = {
        "format": "FERRIC-ENGINEERING-S1-K4-FINAL-RMS-V1", "authority": "none",
        "qualification": False, "benchmark_comparable": False,
        "numerical_comparison_performed": False,
        "retained_payload_scope": "all-target-logits-and-final-rms-row3-only",
        "target_segment": 4, "active_index": 3, "position": 131,
        "input_token_id": sequence[131], "dispatch_generation": capture["dispatch_generation"],
        "capture_sha256": reference.digest(payloads["capture.json"]),
        "epsilon_f32_bits": 0x358637BD, "weight_tensor": "model.norm.weight",
    }
    keys = set(expected) | {"completed_readback", "input", "output", "scope", "nonclaim"}
    if not isinstance(document, dict) or set(document) != keys:
        raise Failure("final-RMS transcript has an unexpected field roster")
    for key in ("scope", "nonclaim"):
        if not isinstance(document[key], str) or not 0 < len(document[key]) <= 4096:
            raise Failure(f"final-RMS explanatory field {key} is invalid")
    for key, value in expected.items():
        require_equal(document.get(key), value, key)
    completed = document["completed_readback"]
    if not isinstance(completed, dict) or set(completed) != {"offset_bytes", "bytes", "sha256"}:
        raise Failure("final-RMS completed readback descriptor is invalid")
    core.require_sha256(completed["sha256"], "unpublished completed readback digest")
    require_equal(completed, {"offset_bytes": base, "bytes": READBACK_BYTES,
                              "sha256": completed["sha256"]}, "readback geometry")
    for index, name in enumerate(("input", "output")):
        filename = f"target-final-rms-{name}.bf16"
        data = payloads[filename]
        decode_row(data)
        require_equal(document[name], {
            "path": filename, "shape": [WIDTH], "dtype": "bf16-little-endian",
            "bytes": ROW_BYTES, "sha256": reference.digest(data),
            "offset_bytes": base + LOGITS_BYTES + (index * reference.ROWS + 3) * ROW_BYTES,
        }, f"{name} descriptor")
    return sequence, document


def row_metrics(actual: bytes, expected: bytes) -> dict:
    left, right = decode_row(actual), decode_row(expected)
    differences = [a - b for a, b in zip(left, right, strict=True)]
    def ordered(bits):
        return 0x8000 - (bits & 0x7FFF) if bits & 0x8000 else 0x8000 + bits
    pairs = list(zip(struct.iter_unpack("<H", actual), struct.iter_unpack("<H", expected), strict=True))
    return {
        "elements": WIDTH, "bit_mismatches": sum(a != b for a, b in pairs),
        "max_bf16_ulp": max(abs(ordered(a[0]) - ordered(b[0])) for a, b in pairs),
        "max_absolute_error": max(abs(value) for value in differences),
        "rmse": math.sqrt(sum(value * value for value in differences) / WIDTH),
        "finite_values": True,
    }


def tensor_row(tensor, torch, device) -> bytes:
    if (tensor.dtype != torch.bfloat16 or tuple(tensor.shape) != (1, 133, WIDTH)
            or tensor.device != device or sys.byteorder != "little"):
        raise Failure("full-sequence final-RMS tensor geometry, dtype, or device drifted")
    row = tensor[0, 131, :].detach().contiguous().cpu()
    data = bytes(row.view(torch.uint8).tolist())
    decode_row(data)
    return data


@contextmanager
def capture_hooks(norm, torch, device):
    captured = {}
    def before(module, arguments):
        if module is not norm or len(arguments) != 1 or "input" in captured:
            raise Failure("final-RMS input hook invocation drifted")
        captured["input"] = tensor_row(arguments[0], torch, device)

    def after(module, arguments, output):
        if module is not norm or "input" not in captured or "output" in captured:
            raise Failure("final-RMS output hook invocation drifted")
        captured["output"] = tensor_row(output, torch, device)

    with ExitStack() as hooks:
        hooks.callback(norm.register_forward_pre_hook(before).remove)
        hooks.callback(norm.register_forward_hook(after).remove)
        yield captured
        if set(captured) != {"input", "output"}:
            raise Failure("full-sequence reference did not capture final RMS exactly once")


def execute(model, torch, sequence: list[int]) -> dict[str, bytes]:
    parameter = next(model.parameters(), None)
    if parameter is None or parameter.device.type != "cuda" or parameter.dtype != torch.bfloat16:
        raise Failure("reference model must be BF16 on the guarded visible GPU")
    with capture_hooks(model.model.norm, torch, parameter.device) as captured:
        logits = reference.execute(model, torch, sequence)
    return {"reference-logits.bf16": logits,
            "reference-final-rms-input.bf16": captured["input"],
            "reference-final-rms-output.bf16": captured["output"]}


def implementation() -> dict:
    names = (Path(__file__).name, "engineering_m5_reference.py", "run.py", "pyproject.toml", "uv.lock")
    with core.SecureDirectory.open(Path(__file__).parent, "final-RMS reference source") as directory:
        return {name: reference.digest(directory.read(name, "reference implementation", maximum=2 * 1024 * 1024))
                for name in names}


def run(arguments: list[str]) -> None:
    core.require_isolated_python()
    core.require_virtual_environment()
    if len(arguments) != 3:
        raise Failure("usage: engineering_m5_final_rms_reference.py CAPTURE-DIRECTORY MODEL-SOURCE NEW-OUTPUT-DIRECTORY")
    capture_path, model_path, output_path = map(Path, arguments)
    source_hashes = implementation()
    with core.SecureDirectory.open(capture_path, "final-RMS native capture") as directory, ExitStack() as held:
        if directory.entries() != set(CAPTURE_FILES):
            raise Failure("final-RMS capture has an unexpected file roster")
        files = {name: held.enter_context(directory.open_file(name, "native capture")) for name in CAPTURE_FILES}
        payloads = {name: file.read(maximum=CAPTURE_FILES[name]) for name, file in files.items()}
        sequence, document = validate_capture(payloads)
        with core.authenticate_model_source(model_path) as model_source:
            dependencies = core.load_dependencies()
            model = core.load_model(dependencies, model_source)
            outputs = execute(model, dependencies.torch, sequence)
            if execute(model, dependencies.torch, sequence) != outputs:
                raise Failure("two independent reference logits/final-RMS captures were not byte-identical")
            model_source.validate()
        for name, file in files.items():
            file.validate()
            if directory.read(name, "reopened native capture", maximum=CAPTURE_FILES[name]) != payloads[name]:
                raise Failure("native capture changed during reference execution")
        if directory.entries() != set(CAPTURE_FILES):
            raise Failure("native capture roster changed during reference execution")
    if implementation() != source_hashes:
        raise Failure("reference implementation changed during execution")
    rows = [dict(position=128 + index, **reference.row_metrics(
        payloads["target-logits.bf16"][index * reference.ROW_BYTES:(index + 1) * reference.ROW_BYTES],
        outputs["reference-logits.bf16"][index * reference.ROW_BYTES:(index + 1) * reference.ROW_BYTES],
    )) for index in range(reference.ROWS)]
    result = {
        "format": "FERRIC-ENGINEERING-S1-K4-FINAL-RMS-DIFFERENTIAL-V1",
        "authority": "none", "qualification": False, "benchmark_comparable": False,
        "tolerance_reviewed": False, "numerical_pass_claimed": False, "cause_established": False,
        "completed_readback_digest_independently_verified": False,
        "reference_execution": "independent-full-133-token-sequence-without-ferric-kv-two-byte-identical-runs",
        "reference_model": {"repository": core.PINNED_REPOSITORY, "revision": core.PINNED_REVISION},
        "position": 131, "active_index": 3, "input_token_ids": sequence,
        "implementation_sha256": source_hashes,
        "native_files_sha256": {name: reference.digest(data) for name, data in payloads.items()},
        "reference_files_sha256": {name: reference.digest(data) for name, data in outputs.items()},
        "native_completed_readback_descriptor": document["completed_readback"],
        "final_rms": {name: row_metrics(payloads[f"target-final-rms-{name}.bf16"],
                                        outputs[f"reference-final-rms-{name}.bf16"])
                      for name in ("input", "output")},
        "rows": rows, "token_mismatch_count": sum(row["token_mismatch"] for row in rows),
        "nonclaim": "Engineering intermediate-value comparison only. Native storage is opt-in host-visible readback storage. The full completed copy is not exported, so its digest is recorded, not independently reproduced. No causal attribution, tolerance acceptance, protected proof, or performance claim.",
    }
    parent, name = core.open_parent(output_path, "final-RMS reference output")
    with parent:
        os.mkdir(name, mode=0o700, dir_fd=parent.fd)
        with parent.child(name, "new final-RMS output") as output:
            for filename, data in outputs.items():
                core.write_new(output.fd, filename, data, "reference capture")
            core.write_new(output.fd, "comparison.json", core.canonical_bytes(result), "final-RMS comparison")
    print(f"output={output_path} authority=none qualification=false token_mismatches={result['token_mismatch_count']}")


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (Failure, OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
