#!/usr/bin/env python3
"""Independent finite protocol checker; never runs or schedules GPU work."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

PATTERNS = ("zero", "request", "ready", "max", "mixed")
REPETITIONS = 8
SHAPES = ("payload_short", "flags_short", "payload_empty", "flags_empty")
FILES = ("payload.f32le", "flags.u32le", "input.f32le", "statuses.u32le", "values.f32le")
SIZES = (512, 512, 512, 1024, 1024)
POLICY = "40 valid fresh launches; >=1 Ready in each consumer wave per initial pattern; >=1 NotReady in each consumer wave overall; 4 invalid-shape controls"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode()


def write_new(path, data):
    with path.open("xb") as stream:
        stream.write(data)


def words(data, count):
    require(len(data) == count * 4, "raw word extent mismatch")
    return list(struct.unpack(f"<{count}I", data))


def cases():
    result = []
    for pattern in PATTERNS:
        for repetition in range(REPETITIONS):
            result.append((f"{pattern}-{repetition}", {
                "schema": "ferric-static-publication-case-v1", "pattern": pattern,
                "repetition": repetition, "shape": "valid"}))
    for shape in SHAPES:
        result.append((shape, {"schema": "ferric-static-publication-case-v1",
                              "pattern": "ready", "repetition": 0, "shape": shape}))
    return result


def case_bytes(case):
    require(type(case) is dict and type(case.get("repetition")) is int, "case repetition must be an integer, not bool")
    require(case in [item[1] for item in cases()], "case outside frozen matrix")
    pattern = case["pattern"]
    repetition = case["repetition"]
    tag = PATTERNS.index(pattern) * REPETITIONS + repetition + 1
    values = [(-1 if cell % 2 else 1) * (tag * 256 + cell + 1) for cell in range(128)]
    inputs = struct.pack("<128f", *values)
    constants = {"zero": 0, "request": 1, "ready": 2, "max": 0xFFFFFFFF}
    mixed = (0, 1, 2, 0xFFFFFFFF, 0xA5A55A5A, 3)
    flags = [mixed[(cell + repetition) % len(mixed)] if pattern == "mixed"
             else constants[pattern] for cell in range(128)]
    return inputs, struct.pack("<128I", *flags)


def expected(case):
    inputs, flags = case_bytes(case)
    return {"schema": "ferric-static-publication-reference-v1", "case": case,
            "policy": POLICY, "reference_sha256": digest(Path(__file__).read_bytes()),
            "case_sha256": digest(encoded(case)), "input_sha256": digest(inputs),
            "initial_flags_sha256": digest(flags), "expected_payload_bits": words(inputs, 128)}


def prepare(directory):
    directory.mkdir(mode=0o700)
    for name, case in cases():
        target = directory / name / "case"
        target.mkdir(parents=True, mode=0o700)
        inputs, flags = case_bytes(case)
        for filename, data in (("case.json", encoded(case)), ("inputs.f32le", inputs),
                               ("flags.u32le", flags), ("expected.json", encoded(expected(case)))):
            write_new(target / filename, data)
    matrix = {"schema": "ferric-static-publication-matrix-v1", "policy": POLICY,
              "reference_sha256": digest(Path(__file__).read_bytes()),
              "cases": [name for name, _ in cases()], "dispatches": 44}
    write_new(directory / "matrix.json", encoded(matrix))
    return matrix


def checked_artifact(artifact_path, object_path, source_path):
    data = artifact_path.read_bytes()
    artifact = json.loads(data)
    require(artifact["schema"] == "ferric-static-publication-artifact-v1", "artifact schema")
    require(artifact["source_sha256"] == digest(source_path.read_bytes())
            and artifact["object_sha256"] == digest(object_path.read_bytes()), "source/object substitution")
    require(artifact["workgroup"] == [128, 1, 1] and artifact["grid"] == [256, 1, 1]
            and artifact["allocation_bytes"] == list(SIZES), "artifact fixed geometry/storage")
    require(artifact["metadata"]["symbol"] == "ferric_gfx950_static_publication_v1"
            and artifact["metadata"]["wavefront_size"] == 64, "artifact symbol/wavefront")
    return artifact, digest(data)


def evaluate(case_directory, run_directory, artifact_path, probe_path, worker_path, object_path, source_path):
    case_raw = (case_directory / "case.json").read_bytes()
    case = json.loads(case_raw)
    wanted = expected(case)
    require(case_raw == encoded(case), "noncanonical case bytes")
    require(json.loads((case_directory / "expected.json").read_bytes()) == wanted, "reference drift")
    inputs, initial_flags = case_bytes(case)
    require((case_directory / "inputs.f32le").read_bytes() == inputs
            and (case_directory / "flags.u32le").read_bytes() == initial_flags, "case data substitution")
    artifact, artifact_hash = checked_artifact(artifact_path, object_path, source_path)
    report_raw = (run_directory / "report.json").read_bytes()
    report = json.loads(report_raw)
    require(report["schema"] == "ferric-static-publication-probe-v1" and report["authority"] == "none",
            "report schema/authority")
    require(report["artifact"] == artifact and report["case"] == case, "report artifact/case substitution")
    require(report["case_sha256"] == digest(case_raw)
            and report["probe_sha256"] == digest(probe_path.read_bytes())
            and report["worker_sha256"] == digest(worker_path.read_bytes())
            and report["input_sha256"] == digest(inputs)
            and report["initial_flags_sha256"] == digest(initial_flags), "execution input identity")
    require(type(report["completed_dispatches"]) is int and report["completed_dispatches"] == 1,
            "exactly one completed dispatch required")
    for key in ("fresh_worker_and_allocations", "input_immutability_and_all_guards_passed",
                "free_close_and_worker_exit_passed"):
        require(report[key] is True, "missing lifecycle/immutability/guard check")
    require(len(report["buffer_sha256"]) == 5, "raw result hash roster")
    raw = [(run_directory / filename).read_bytes() for filename in FILES]
    for index, data in enumerate(raw):
        require(len(data) == SIZES[index] and digest(data) == report["buffer_sha256"][index], "raw hash/extent")
    require(raw[2] == inputs, "immutable input mismatch")
    payload, flags, _, statuses, values = [words(data, size // 4) for data, size in zip(raw, SIZES)]
    ready_cells, not_ready_cells = [], []
    if case["shape"] != "valid":
        require(raw[0] == bytes([0xA7]) * 512 and raw[1] == initial_flags, "invalid shape changed protocol buffers")
        require(statuses == [3] * 256 and values == [0] * 256, "invalid shape result")
    else:
        require(payload == wanted["expected_payload_bits"], "producer payload mismatch")
        require(statuses[:128] == [1] * 128 and values[:128] == [0] * 128, "producer status/value")
        require(all(flag in (1, 2) for flag in flags), "impossible final flag")
        for cell in range(128):
            status, value = statuses[128 + cell], values[128 + cell]
            require(status in (0, 2), "consumer status")
            if status == 2:
                require(value == payload[cell] and flags[cell] == 2, "Ready value/cell/final flag mismatch")
                ready_cells.append(cell)
            else:
                require(value == 0, "NotReady must return positive zero")
                not_ready_cells.append(cell)
    return {"schema": "ferric-static-publication-numerical-v1", "authority": "none",
            "status": "protocol_pass" if ready_cells or case["shape"] != "valid" else "no_ready_observed",
            "case": case, "ready_cells": ready_cells, "not_ready_cells": not_ready_cells,
            "ready_count": len(ready_cells), "not_ready_count": len(not_ready_cells),
            "report_sha256": digest(report_raw), "artifact_record_sha256": artifact_hash,
            "reference_sha256": wanted["reference_sha256"], "buffer_sha256": report["buffer_sha256"],
            "scope": "observed exact bits; no-read is a compiler/structural property, not inferred from zero output"}


def summarize(directory, paths):
    matrix = json.loads((directory / "matrix.json").read_bytes())
    require(matrix == {"schema": "ferric-static-publication-matrix-v1", "policy": POLICY,
                       "reference_sha256": digest(Path(__file__).read_bytes()),
                       "cases": [name for name, _ in cases()], "dispatches": 44}, "matrix drift")
    results = [evaluate(directory / name / "case", directory / name / "run", *paths) for name, _ in cases()]
    coverage = {pattern: [set(), set()] for pattern in PATTERNS}
    not_ready_coverage = [set(), set()]
    for result in results:
        if result["case"]["shape"] == "valid":
            for cell in result["ready_cells"]:
                coverage[result["case"]["pattern"]][cell // 64].add(cell)
            for cell in result["not_ready_cells"]:
                not_ready_coverage[cell // 64].add(cell)
    passed = all(all(wave for wave in waves) for waves in coverage.values()) and all(not_ready_coverage)
    return {"schema": "ferric-static-publication-suite-v1", "authority": "none",
            "status": "pass" if passed else "inconclusive", "policy": POLICY,
            "completed_dispatches": len(results), "valid_dispatches": 40, "invalid_dispatches": 4,
            "ready_count": sum(item["ready_count"] for item in results),
            "not_ready_count": sum(item["not_ready_count"] for item in results),
            "ready_coverage": {key: [sorted(wave) for wave in waves] for key, waves in coverage.items()},
            "not_ready_coverage": [sorted(wave) for wave in not_ready_coverage],
            "results": results, "timing_claim": "none", "protected_runtime_authority": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("prepare").add_argument("--directory", type=Path, required=True)
    for command in ("check", "summarize"):
        child = sub.add_parser(command)
        for name in ("artifact", "probe", "worker", "object", "source"):
            child.add_argument("--" + name, type=Path, required=True)
        for name in (("case-dir", "run-dir") if command == "check" else ("directory",)):
            child.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    if args.command == "prepare":
        result = prepare(args.directory)
    else:
        paths = (args.artifact, args.probe, args.worker, args.object, args.source)
        if args.command == "check":
            result = evaluate(args.case_dir, args.run_dir, *paths)
            write_new(args.run_dir / "numerical.json", encoded(result))
        else:
            result = summarize(args.directory, paths)
            write_new(args.directory / "summary.json", encoded(result))
    print(json.dumps(result, sort_keys=True))
    if result.get("status") == "inconclusive":
        raise SystemExit(3)


if __name__ == "__main__":
    main()
