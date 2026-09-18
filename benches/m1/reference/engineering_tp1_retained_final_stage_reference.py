#!/usr/bin/env python3
"""CPU-only comparison with the authenticated, retained gfx942 reference passes."""

from contextlib import ExitStack
import importlib.util
import os
from pathlib import Path
import sys


specification = importlib.util.spec_from_file_location(
    "ferric_tp1_retained_final_stage_core",
    Path(__file__).with_name("engineering_tp1_final_stage_reference.py"),
)
if specification is None or specification.loader is None:
    raise ImportError("cannot load sibling final-stage reference")
reference = importlib.util.module_from_spec(specification)
sys.modules[specification.name] = reference
specification.loader.exec_module(reference)
core = reference.core
Failure = reference.Failure
digest = reference.digest

# These records identify the already retained run, not a new GPU execution.
HISTORICAL_COMPARISON_SHA256 = "556872313ab799547778ebc842095709d3e388b97930edd54c7324b6334a6c88"
HISTORICAL_RECORD_HASHES = {
    "argv.txt": "ebf1094c262129f6eb2a245716a65b2affc81d223f4afc70b4e18765b8a17630",
    "started.txt": "befef478253f7921328a796e05d9b3b1d03cc92b94b6b418faa08bc3897e7020",
    "finished.txt": "920b1e87428c8e7a7f554f372e73af5ca31030a570d6b7324f2555f7f3beacd9",
}
PASS_FILES = {f"pass{ordinal}-{name}.bf16" for ordinal in (1, 2) for name in reference.WIDTHS}
HISTORY_FILES = PASS_FILES | {"comparison.json", "native-pins.json"} | set(HISTORICAL_RECORD_HASHES)
IMPLEMENTATION_FILES = {"engineering_tp1_final_stage_reference.py", "run.py", "pyproject.toml", "uv.lock"}
COMPARISON_FIELDS = {
    "schema", "authority", "qualification", "benchmark_comparable", "numerical_pass_claimed",
    "tolerance_reviewed", "cause_established", "process_absence_independently_proven",
    "reference_execution", "reference_byte_identical", "reference_model", "native_target",
    "reference_target", "input_tokens", "positions", "rows", "nonclaim", "implementation_sha256",
    "native_pins_sha256", "native_files_sha256", "native_witnesses_sha256", "reference_files_sha256",
}
NONCLAIM = (
    "CPU comparison of new gfx950 native captures with retained gfx942 reference bytes. "
    "No fresh reference execution, tolerance acceptance, numerical pass, causal attribution, "
    "protected proof, performance claim, or M1 gate closure."
)


def validate_history(payloads: dict[str, bytes], expected_sha256: str, native: dict):
    core.require_sha256(expected_sha256, "retained comparison SHA256")
    reference.equal(expected_sha256, HISTORICAL_COMPARISON_SHA256, "admitted historical comparison")
    if set(payloads) != HISTORY_FILES or sum(map(len, payloads.values())) > reference.MAX_BYTES:
        raise Failure("retained reference roster or total extent drifted")
    reference.equal(digest(payloads["comparison.json"]), expected_sha256, "retained comparison bytes")
    historical = reference.document(payloads["comparison.json"])
    reference.object_keys(historical, COMPARISON_FIELDS, "retained comparison")
    expected = {
        "schema": "FerricTpFinalStageReferenceComparisonV1", "authority": "none",
        "qualification": False, "benchmark_comparable": False, "numerical_pass_claimed": False,
        "tolerance_reviewed": False, "cause_established": False,
        "process_absence_independently_proven": False,
        "reference_execution": "two-independent-full-sequence-executions-use-cache-false",
        "reference_model": {"repository": core.PINNED_REPOSITORY, "revision": core.PINNED_REVISION},
        "native_target": "gfx950:xnack-", "reference_target": core.TARGET,
    }
    for key, value in expected.items():
        reference.equal(historical[key], value, f"retained {key}")
    old_pins = reference.validate_pins(payloads["native-pins.json"], historical["native_pins_sha256"])
    reference.equal(old_pins["model_bundle_id"], native["setup"]["model_bundle_id"], "model content identity")
    for key in ("prompt", "prompt_tokens", "new_tokens"):
        reference.equal(old_pins[key], native["setup"][key], f"retained workload {key}")
    sequence = reference.tokens(historical["input_tokens"], "retained consumed sequence")
    selected = reference.positions(historical["positions"], len(sequence))
    reference.equal(sequence, native["input_tokens"], "retained consumed sequence")
    reference.equal(selected, native["positions"], "retained positions")
    reference.equal(selected, old_pins["positions"], "retained pin positions")
    reference.equal(sequence[:len(old_pins["prompt_tokens"])], old_pins["prompt_tokens"], "retained prompt tokens")
    reference.equal(len(sequence), len(old_pins["prompt_tokens"]) + old_pins["new_tokens"] - 1,
                    "retained sequence length")
    reference.object_keys(historical["implementation_sha256"], IMPLEMENTATION_FILES, "original implementation")
    for name, value in historical["implementation_sha256"].items():
        core.require_sha256(value, f"original implementation {name}")
    for name, expected_hash in HISTORICAL_RECORD_HASHES.items():
        reference.equal(digest(payloads[name]), expected_hash, f"original producer record {name}")
        payloads[name].decode("utf-8")
    reference.object_keys(historical["reference_files_sha256"], PASS_FILES, "retained tensor hashes")
    passes = []
    for ordinal in (1, 2):
        output = {}
        for name, width in reference.WIDTHS.items():
            filename = f"pass{ordinal}-{name}.bf16"
            expected_hash = historical["reference_files_sha256"][filename]
            core.require_sha256(expected_hash, filename)
            raw = payloads[filename]
            reference.equal(digest(raw), expected_hash, f"retained tensor {filename}")
            reference.equal(len(raw), len(selected) * width * 2, f"retained tensor extent {filename}")
            output[f"{name}.bf16"] = raw
        passes.append(output)
    reference.equal(historical["reference_byte_identical"], passes[0] == passes[1], "original repeat observation")
    return historical, old_pins, passes


def implementation(expected_arithmetic: str) -> dict[str, str]:
    result = reference.implementation(expected_arithmetic=expected_arithmetic)
    with core.SecureDirectory.open(Path(__file__).parent, "retained comparator source") as directory:
        name = Path(__file__).name
        result[name] = digest(directory.read(name, "retained comparator", maximum=2 * 1024 * 1024))
    return result


def run(arguments: list[str]) -> None:
    if len(arguments) != 8:
        raise Failure("usage: engineering_tp1_retained_final_stage_reference.py EXPECTED-ARITHMETIC "
                      "CAPTURE WITNESS PINS PINS-SHA256 RETAINED-REFERENCE COMPARISON-SHA256 NEW-OUTPUT")
    mode, capture_path, witness_path, pins_path, pins_sha, history_path, history_sha, output_path = arguments
    if type(mode) is not str or mode not in reference.ARITHMETIC_MODES:
        raise Failure("retained comparison requires an explicit arithmetic mode")
    reference.capture_profile(mode)
    core.require_isolated_python()
    source_hashes = implementation(mode)
    with ExitStack() as held:
        source = held.enter_context(core.SecureDirectory.open(Path(__file__).parent, "comparator source"))
        pins_parent, pins_name = core.open_parent(Path(pins_path), "new native pins")
        held.enter_context(pins_parent)
        pins_file = held.enter_context(pins_parent.open_file(pins_name, "new native pins"))
        parent, name = core.open_parent(Path(output_path), "retained comparison output")
        held.enter_context(parent)
        if name in parent.entries():
            raise Failure("comparison output already exists")
        forbidden = {source.identity, pins_parent.identity}
        pin_bytes = pins_file.read(maximum=65536)
        pins = reference.validate_pins(pin_bytes, pins_sha, expected_arithmetic=mode)
        with reference.held_directory(Path(capture_path), reference.CAPTURE_FILES, "new native capture", forbidden) as payloads, \
                reference.held_directory(Path(witness_path), reference.WITNESS_FILES, "new native witnesses", forbidden) as witnesses, \
                reference.held_directory(Path(history_path), HISTORY_FILES, "retained reference", forbidden) as history:
            reference.reject_output_alias(parent, forbidden)
            native = reference.validate_capture(payloads, witnesses, pins, expected_arithmetic=mode)
            historical, old_pins, passes = validate_history(history, history_sha, native)
            result = reference.compare(payloads, passes, native, expected_arithmetic=mode)
            result.update(
                schema="FerricTpRetainedArithmeticFinalStageReferenceComparisonV1",
                reference_execution="retained-two-full-sequence-passes-no-new-reference-execution",
                fresh_reference_execution=False, nonclaim=NONCLAIM,
                comparator_implementation_sha256=source_hashes,
                native_pins_sha256=digest(pin_bytes),
                native_files_sha256={key: digest(value) for key, value in payloads.items()},
                native_witnesses_sha256={key: digest(value) for key, value in witnesses.items()},
                reference_files_sha256=historical["reference_files_sha256"],
                retained_reference={
                    "comparison_sha256": history_sha,
                    "native_pins_sha256": historical["native_pins_sha256"],
                    "model_bundle_id": old_pins["model_bundle_id"],
                    "implementation_sha256": historical["implementation_sha256"],
                    "reference_execution": historical["reference_execution"],
                    "producer_records": {key: history[key].decode("utf-8") for key in HISTORICAL_RECORD_HASHES},
                    "files_sha256": {key: digest(value) for key, value in history.items()},
                },
            )
        pins_file.validate()
        with pins_parent.open_file(pins_name, "reopened native pins") as reopened:
            if reopened.identity != pins_file.identity:
                raise Failure("native pin file identity drifted")
        for path, directory in ((Path(pins_path).parent, pins_parent), (Path(__file__).parent, source),
                                (Path(output_path).parent, parent)):
            with core.SecureDirectory.open(path, "reopened comparison binding") as reopened:
                if reopened.identity != directory.identity:
                    raise Failure("comparison directory identity drifted")
        reference.equal(implementation(mode), source_hashes, "comparator implementation")
        reference.reject_output_alias(parent, forbidden)
        outputs = {key if key in PASS_FILES else f"retained-{key}": value for key, value in history.items()}
        outputs["comparison.json"] = core.canonical_bytes(result)
        if sum(map(len, outputs.values())) > reference.MAX_BYTES:
            raise Failure("retained comparison output exceeds bound")
        os.mkdir(name, mode=0o700, dir_fd=parent.fd)
        with parent.child(name, "new retained comparison output") as output:
            for filename, data in outputs.items():
                core.write_new(output.fd, filename, data, "retained comparison evidence")
            os.fsync(output.fd)
        os.fsync(parent.fd)
    print(f"output={output_path} authority=none qualification=false fresh_reference_execution=false")


if __name__ == "__main__":
    try:
        run(sys.argv[1:])
    except (Failure, OSError, ValueError, KeyError, TypeError, AttributeError, OverflowError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
