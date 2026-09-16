#!/usr/bin/env python3
"""Independent full-sequence reference for the separate engineering S1/K4 capture."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import struct
import sys


def load_core():
    specification = importlib.util.spec_from_file_location(
        "ferric_m1_reference_core", Path(__file__).with_name("run.py")
    )
    if specification is None or specification.loader is None:
        raise ImportError("cannot load pinned sibling reference core")
    module = importlib.util.module_from_spec(specification)
    sys.modules[specification.name] = module
    specification.loader.exec_module(module)
    return module


core = load_core()
Failure = core.ReferenceFailure
ROWS = 5
ROW_BYTES = core.VOCABULARY_SIZE * 2
TARGET_PREPACKED = "d6d8b55c73595645dedeade005d4e853098cf3a22faf1c045922d00367ecff7e"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def tokens(value, count: int, label: str) -> list[int]:
    if not isinstance(value, list) or len(value) != count:
        raise Failure(f"{label} must contain exactly {count} tokens")
    if any(type(token) is not int or not 0 <= token < core.VOCABULARY_SIZE for token in value):
        raise Failure(f"{label} contains an invalid token")
    return value


def validate_capture(document: dict, logits: bytes) -> list[int]:
    required = {
        "format": "FERRIC-ENGINEERING-S1-K4-ALL-LOGITS-V1",
        "authority": "none", "qualification": False,
        "benchmark_comparable": False, "numerical_comparison_performed": False,
        "scope": "single-sequence-k4-five-target-positions-with-real-paired-prefill",
        "reference_sequence_semantics": "full-128-token-prefix-plus-anchor-plus-four-actual-draft-proposals",
        "positions": [128, 129, 130, 131, 132],
        "shape": [1, 5, core.VOCABULARY_SIZE], "dtype": "bf16-little-endian",
    }
    for key, expected in required.items():
        if document.get(key) != expected or type(document.get(key)) is not type(expected):
            raise Failure(f"capture field {key} drifted")
    prefix = tokens(document["prefix_token_ids"], 128, "prefix")
    target = tokens(document["target_token_ids"], 5, "target positions")
    smoke = document["smoke"]
    for key, expected in {
        "authority": "none", "artifact_authority": "none", "benchmark_comparable": False,
        "authenticated_AB_exercised": False, "compiler_origin_authenticated": False,
        "current_publication_selected": False, "worker_v3_authenticated": False,
        "hardware_completion_observed": True, "target": "gfx942:xnack-",
        "program_strategy": "AttributedMfma13", "program_count": 13,
    }.items():
        if smoke.get(key) != expected or type(smoke.get(key)) is not type(expected):
            raise Failure(f"smoke field {key} drifted")
    if (smoke["identities"]["model_bundle_sha256"] != core.PINNED_MODEL_IDENTITY
            or smoke["identities"]["target_prepacked_sha256"] != TARGET_PREPACKED):
        raise Failure("capture model differs from independently authenticated target checkpoint")
    for value in smoke["identities"].values():
        core.require_sha256(value, "capture identity")
    speculative = smoke["speculative_k4"]
    if prefix != tokens(smoke["prompt"]["physical_token_ids"], 128, "physical prefix"):
        raise Failure("reference prefix differs from the actual native prefix")
    anchor = tokens([smoke["paired_prefill"]["first_token_id"]], 1, "anchor")
    if target != anchor + tokens(speculative["draft_choices"], 4, "actual proposals"):
        raise Failure("reference target sequence differs from the actual anchor/proposals")
    generation = document["dispatch_generation"]
    if type(generation) is not int or generation <= 0 or generation != speculative["dispatch_generation"]:
        raise Failure("capture generation drifted")
    if document["logits"] != {
        "path": "target-logits.bf16", "bytes": ROWS * ROW_BYTES, "sha256": digest(logits)
    } or len(logits) != ROWS * ROW_BYTES:
        raise Failure("capture all-row bytes differ from transcript")
    expected_choices = tokens(speculative["target_choices"], 5, "actual target choices")
    rows = core.exact_array(document["rows"], ROWS, "capture rows")
    base = rows[0]["offset_bytes"]
    if type(base) is not int or base < 0 or base % 2:
        raise Failure("capture allocation offset is invalid")
    for index, row in enumerate(rows):
        data = logits[index * ROW_BYTES:(index + 1) * ROW_BYTES]
        if row != {
            "active_index": index, "position": 128 + index, "bytes": ROW_BYTES,
            "offset_bytes": base + index * ROW_BYTES, "sha256": digest(data),
            "argmax_token": expected_choices[index],
        } or core.bf16_argmax(data) != expected_choices[index]:
            raise Failure(f"capture row {index} geometry, values, or target choice drifted")
    return prefix + target


def execute(model, torch, sequence: list[int]) -> bytes:
    if len(sequence) != 133:
        raise Failure("reference requires the exact 128+5 sequence")
    parameter = next(model.parameters(), None)
    if parameter is None or parameter.device.type != "cuda" or parameter.dtype != torch.bfloat16:
        raise Failure("reference model must be BF16 on the guarded visible GPU")
    with torch.inference_mode():
        ids = torch.tensor([sequence], dtype=torch.long, device=parameter.device)
        mask = torch.ones_like(ids)
        result = model.model(input_ids=ids, attention_mask=mask, return_dict=True, use_cache=False)
        hidden = result.last_hidden_state
        if hidden.dtype != torch.bfloat16 or tuple(hidden.shape) != (1, 133, 4096):
            raise Failure("reference full-sequence hidden-state shape drifted")
        projected = model.lm_head(hidden[:, 128:133, :]).to(dtype=torch.bfloat16)
        if tuple(projected.shape) != (1, 5, core.VOCABULARY_SIZE):
            raise Failure("reference five-position logits shape drifted")
        output = b"".join(core.serialize_bf16_tensor(projected[0, index, :], torch) for index in range(5))
    torch.cuda.synchronize(parameter.device)
    return output


def row_metrics(actual: bytes, reference: bytes) -> dict:
    actual_token = core.bf16_argmax(actual)
    reference_token = core.bf16_argmax(reference)
    maximum_ulp = 0
    maximum_absolute = 0.0
    squared = 0.0
    for (left,), (right,) in zip(struct.iter_unpack("<H", actual), struct.iter_unpack("<H", reference), strict=True):
        def ordered(bits):
            return 0x8000 - (bits & 0x7FFF) if bits & 0x8000 else 0x8000 + bits
        maximum_ulp = max(maximum_ulp, abs(ordered(left) - ordered(right)))
        difference = abs(struct.unpack("<f", struct.pack("<I", left << 16))[0]
                         - struct.unpack("<f", struct.pack("<I", right << 16))[0])
        maximum_absolute = max(maximum_absolute, difference)
        squared += difference * difference
    return {
        "ferric_argmax_token": actual_token, "reference_argmax_token": reference_token,
        "token_mismatch": actual_token != reference_token, "max_bf16_ulp": maximum_ulp,
        "max_absolute_error": maximum_absolute, "rmse": math.sqrt(squared / core.VOCABULARY_SIZE),
        "finite_logits": True,
    }


def implementation() -> dict:
    root = Path(__file__).parent
    names = (Path(__file__).name, "run.py", "pyproject.toml", "uv.lock")
    with core.SecureDirectory.open(root, "reference source") as directory:
        return {name: digest(directory.read(name, "reference implementation", maximum=2 * 1024 * 1024)) for name in names}


def run(arguments: list[str]) -> None:
    core.require_isolated_python()
    core.require_virtual_environment()
    if len(arguments) != 3:
        raise Failure("usage: engineering_m5_reference.py CAPTURE-DIRECTORY MODEL-SOURCE NEW-OUTPUT-DIRECTORY")
    capture_path, model_path, output_path = map(Path, arguments)
    source_hashes = implementation()
    with core.SecureDirectory.open(capture_path, "M=5 capture") as captures:
        if captures.entries() != {"capture.json", "target-logits.bf16"}:
            raise Failure("M=5 capture has an unexpected file roster")
        with captures.open_file("capture.json", "M=5 transcript") as transcript:
            data = transcript.read(maximum=1024 * 1024)
            document = json.loads(data, object_pairs_hook=core._unique_object)
            with captures.open_file("target-logits.bf16", "M=5 captured logits") as captured:
                actual = captured.read(exact=ROWS * ROW_BYTES)
                sequence = validate_capture(document, actual)
                with core.authenticate_model_source(model_path) as model_source:
                    dependencies = core.load_dependencies()
                    model = core.load_model(dependencies, model_source)
                    reference = execute(model, dependencies.torch, sequence)
                    if execute(model, dependencies.torch, sequence) != reference:
                        raise Failure("independent reference executions were not byte-identical")
                    model_source.validate()
                transcript.validate()
                captured.validate()
        if captures.read("capture.json", "reopened transcript") != data:
            raise Failure("capture transcript changed during reference execution")
        if digest(captures.read("target-logits.bf16", "reopened logits", maximum=ROWS * ROW_BYTES)) != digest(actual):
            raise Failure("capture logits changed during reference execution")
    if implementation() != source_hashes:
        raise Failure("reference implementation changed during execution")
    rows = [dict(position=128 + index, **row_metrics(
        actual[index * ROW_BYTES:(index + 1) * ROW_BYTES],
        reference[index * ROW_BYTES:(index + 1) * ROW_BYTES],
    )) for index in range(ROWS)]
    result = {
        "format": "FERRIC-ENGINEERING-S1-K4-DIFFERENTIAL-V1",
        "authority": "none", "qualification": False, "benchmark_comparable": False,
        "tolerance_reviewed": False, "numerical_pass_claimed": False,
        "reference_execution": "independent-full-133-token-sequence-without-ferric-kv-two-byte-identical-runs",
        "capture_sha256": digest(data), "ferric_logits_sha256": digest(actual),
        "reference_logits_sha256": digest(reference), "implementation_sha256": source_hashes,
        "reference_model": {"repository": core.PINNED_REPOSITORY, "revision": core.PINNED_REVISION},
        "input_token_ids": sequence, "rows": rows,
        "token_mismatch_count": sum(row["token_mismatch"] for row in rows),
        "max_bf16_ulp": max(row["max_bf16_ulp"] for row in rows),
        "nonclaim": "Engineering numerical metrics only. No protected R29 gate closure, tolerance acceptance, general K8/K16 coverage, or performance claim.",
    }
    parent, name = core.open_parent(output_path, "M=5 reference output")
    with parent:
        os.mkdir(name, mode=0o700, dir_fd=parent.fd)
        with parent.child(name, "M=5 new output") as output:
            core.write_new(output.fd, "reference-logits.bf16", reference, "reference logits")
            core.write_new(output.fd, "comparison.json", core.canonical_bytes(result), "M=5 metrics")
    print(f"output={output_path} authority=none qualification=false token_mismatches={result['token_mismatch_count']} max_bf16_ulp={result['max_bf16_ulp']}")


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (Failure, OSError, ValueError, KeyError, TypeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
