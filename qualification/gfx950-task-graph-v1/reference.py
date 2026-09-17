#!/usr/bin/env python3
"""Independent topological reference for the bounded task graph, never a GPU runner."""

import argparse
import hashlib
import json
from pathlib import Path
import random
import struct

TASKS = 7
TILE = 128
DEPENDENCIES = ((), (0,), (0,), (1, 2), (3,), (3,), (4, 5))
CASES = ("zero", "ramp", "maximum", "random")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def decode_inputs(data):
    require(len(data) == TASKS * TILE * 4, "input extent must be896 u32 values")
    values = struct.unpack("<896I", data)
    require(all(value <= 1024 for value in values), "input value exceeds1024")
    return values


def reference(values):
    """Evaluate the DAG in topological order, independent of GPU queue/claim order."""
    require(len(values) == TASKS * TILE, "reference input extent mismatch")
    require(all(type(value) is int and 0 <= value <= 1024 for value in values),
            "reference input domain mismatch")
    results = []
    for task, dependencies in enumerate(DEPENDENCIES):
        tile_sum = sum(values[task * TILE:(task + 1) * TILE])
        results.append(tile_sum + sum(results[parent] for parent in dependencies))
    require(max(results) <= 0xFFFFFFFF, "reference does not fit u32")
    return results


def expected_record(inputs, name):
    return {
        "schema": "ferric-task-graph-reference-v1",
        "case": name,
        "input_sha256": digest(inputs),
        "reference_sha256": digest(Path(__file__).read_bytes()),
        "task_count": TASKS,
        "tile_words": TILE,
        "dependencies": [list(dependencies) for dependencies in DEPENDENCIES],
        "expected_payloads": reference(decode_inputs(inputs)),
        "positive_epochs": [1, 2, 3, 4],
        "stale_expected_epoch": 5,
        "stale_initialized_epoch": 4,
    }


def generate(name, directory):
    require(name in CASES, "unknown reference case")
    if name == "zero":
        values = [0] * (TASKS * TILE)
    elif name == "ramp":
        values = [(task * 113 + lane * 7) % 1025
                  for task in range(TASKS) for lane in range(TILE)]
    elif name == "maximum":
        values = [1024] * (TASKS * TILE)
    else:
        rng = random.Random(20260915)
        values = [rng.randrange(1025) for _ in range(TASKS * TILE)]
    inputs = struct.pack("<896I", *values)
    directory.mkdir(mode=0o700, parents=False, exist_ok=False)
    (directory / "inputs.u32le").write_bytes(inputs)
    record = expected_record(inputs, name)
    (directory / "expected.json").write_text(json.dumps(record, indent=2) + "\n")
    return record


def validate_epochs(epochs, payloads):
    require(type(epochs) is list and len(epochs) == 5, "exactly five epochs required")
    for index, epoch in enumerate(epochs):
        expected_epoch = index + 1
        actual_epoch = min(expected_epoch, 4)
        stale = index == 4
        require(epoch["expected_epoch"] == expected_epoch
                and epoch["initialized_epoch"] == actual_epoch
                and epoch["stale_epoch_negative"] is stale, "epoch sequence mismatch")
        state = epoch["state"]
        require(type(state) is list and len(state) == 13
                and all(type(word) is int and 0 <= word <= 0xFFFFFFFF for word in state),
                "state must contain13 exact u32 words")
        if stale:
            require(state == [4, 1, 0, 0, 0, 1] + [0] * 7,
                    "stale epoch changed graph state or failed exact error reporting")
        else:
            require(state[:4] == [expected_epoch, 0, 127, 127] and state[5] == 0,
                    "completion state mismatch")
            require(state[4] >> 14 == 0
                    and all((state[4] >> (2 * task)) & 3 in (1, 2) for task in range(TASKS)),
                    "task owner must be exactly one of two workgroups")
            require(state[6:] == payloads, "independent task payload mismatch")
        owners = [(state[4] >> (2 * task)) & 3 for task in range(TASKS)]
        cross_edges = [[parent, task] for task, parents in enumerate(DEPENDENCIES)
                       for parent in parents if owners[parent] and owners[task]
                       and owners[parent] != owners[task]]
        require(epoch["task_owners"] == owners
                and epoch["distinct_observed_workgroups"] == len(set(owners) - {0})
                and epoch["cross_workgroup_dependency_edges"] == cross_edges,
                "owner evidence does not match actual dependency endpoints")
        names = ("worker_dispatch_interval_ns", "host_dispatch_roundtrip_ns",
                 "host_epoch_reset_dispatch_readback_ns")
        times = [epoch[name] for name in names]
        require(all(type(value) is int and value >= 0 for value in times),
                "timings must be nonnegative host intervals")
        require(times[0] <= times[1] <= times[2], "nested host timing intervals inconsistent")


def check(case_directory, run_directory, artifact_path, probe_path, worker_path):
    inputs = (case_directory / "inputs.u32le").read_bytes()
    expected = json.loads((case_directory / "expected.json").read_text())
    require(expected == expected_record(inputs, expected["case"]),
            "reference artifact no longer matches independently recomputed inputs/source")
    artifact_bytes = artifact_path.read_bytes()
    artifact = json.loads(artifact_bytes)
    report_bytes = (run_directory / "report.json").read_bytes()
    report = json.loads(report_bytes)
    require(report["schema"] == "ferric-task-graph-probe-v1"
            and report["authority"] == "none", "unexpected report schema or authority")
    require(report["artifact"] == artifact, "inspected artifact was substituted")
    require(report["probe_sha256"] == digest(probe_path.read_bytes())
            and report["worker_sha256"] == digest(worker_path.read_bytes()),
            "probe or worker executable identity mismatch")
    require(report["input_sha256"] == digest(inputs), "dispatched input identity mismatch")
    require(report["completed_dispatches"] == 5
            and report["reused_worker_queue_and_allocations"] is True
            and report["input_immutability_and_all_guards_passed"] is True
            and report["free_close_and_worker_exit_passed"] is True,
            "missing exact completion, immutability, guards, or cleanup")
    validate_epochs(report["epochs"], expected["expected_payloads"])
    states = (run_directory / "states.u32le").read_bytes()
    packed = b"".join(struct.pack("<13I", *epoch["state"]) for epoch in report["epochs"])
    require(states == packed and report["states_sha256"] == digest(states),
            "raw atomic state does not match report/hash")
    lifecycle = report["host_lifecycle_ns"]
    require(type(lifecycle) is int and lifecycle >= sum(
        epoch["host_epoch_reset_dispatch_readback_ns"] for epoch in report["epochs"]),
        "host lifecycle does not cover all epochs")
    evidence = {
        "schema": "ferric-task-graph-numerical-evidence-v1", "status": "pass",
        "scope": "bounded engineering scheduler micrograph, not model inference or production authority",
        "case": expected["case"], "reference_sha256": expected["reference_sha256"],
        "artifact_record_sha256": digest(artifact_bytes), "report_sha256": digest(report_bytes),
        "input_sha256": digest(inputs), "states_sha256": digest(states),
        "object_sha256": artifact["object_sha256"], "source_sha256": artifact["source_sha256"],
        "probe_sha256": report["probe_sha256"], "worker_sha256": report["worker_sha256"],
        "positive_epochs": 4, "stale_epoch_rejections": 1,
        "independent_payload_values_checked": 28, "state_words_validated": 65,
        "deterministic_state_words_compared": 61,
        "cross_workgroup_dependency_observed": any(
            epoch["cross_workgroup_dependency_edges"] for epoch in report["epochs"]),
        "timing_claim": "host intervals only; no GPU-only timing or performance comparison",
    }
    destination = run_directory / "numerical.json"
    with destination.open("x") as stream:
        json.dump(evidence, stream, indent=2)
        stream.write("\n")
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    generate_parser = commands.add_parser("generate")
    generate_parser.add_argument("--case", choices=CASES, required=True)
    generate_parser.add_argument("--case-dir", type=Path, required=True)
    check_parser = commands.add_parser("check")
    for argument in ("case-dir", "run-dir", "artifact", "probe", "worker"):
        check_parser.add_argument("--" + argument, type=Path, required=True)
    args = parser.parse_args()
    if args.command == "generate":
        result = generate(args.case, args.case_dir)
    else:
        result = check(args.case_dir, args.run_dir, args.artifact, args.probe, args.worker)
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
