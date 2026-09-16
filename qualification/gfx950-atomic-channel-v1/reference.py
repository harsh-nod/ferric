#!/usr/bin/env python3
"""Independent exact-bit reference for indexed atomic storage, never a GPU runner."""

import argparse
import hashlib
import json
from pathlib import Path
import random
import struct

WORDS = 256
CASES = ("zero", "walking", "alternating", "random")
SEED = 20260916


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def decode_words(data):
    require(len(data) == WORDS * 4, "exactly 256 little-endian u32 words required")
    return list(struct.unpack("<256I", data))


def reference(values):
    """The contract preserves every bit, independently of lane or queue order."""
    require(len(values) == WORDS, "reference extent mismatch")
    require(all(type(value) is int and 0 <= value <= 0xFFFFFFFF for value in values),
            "reference domain must be exact u32 values")
    return list(values)


def expected_record(inputs, name):
    require(name in CASES, "unknown reference case")
    return {
        "schema": "ferric-atomic-channel-reference-v1",
        "case": name,
        "input_sha256": digest(inputs),
        "reference_sha256": digest(Path(__file__).read_bytes()),
        "expected_words": reference(decode_words(inputs)),
        "word_count": WORDS,
        "comparison": "exact u32 bit preservation in both channel and output",
    }


def generate(name, directory):
    require(name in CASES, "unknown reference case")
    if name == "zero":
        values = [0] * WORDS
    elif name == "walking":
        values = [1 << (index % 32) for index in range(WORDS)]
    elif name == "alternating":
        values = [0xFFFFFFFF, 0, 0xAAAAAAAA, 0x55555555] * (WORDS // 4)
    else:
        rng = random.Random(SEED)
        values = [rng.getrandbits(32) for _ in range(WORDS)]
    inputs = struct.pack("<256I", *values)
    directory.mkdir(mode=0o700, parents=False, exist_ok=False)
    (directory / "inputs.u32le").write_bytes(inputs)
    expected = expected_record(inputs, name)
    (directory / "expected.json").write_text(json.dumps(expected, indent=2) + "\n")
    return expected


def check(case_directory, run_directory, artifact_path, probe_path, worker_path):
    inputs = (case_directory / "inputs.u32le").read_bytes()
    expected = json.loads((case_directory / "expected.json").read_text())
    require(expected == expected_record(inputs, expected["case"]),
            "reference no longer binds exact input bytes and reference source")
    artifact_bytes = artifact_path.read_bytes()
    artifact = json.loads(artifact_bytes)
    require(artifact["schema"] == "ferric-atomic-channel-artifact-v1",
            "unexpected artifact schema")
    report_bytes = (run_directory / "report.json").read_bytes()
    report = json.loads(report_bytes)
    require(report["schema"] == "ferric-atomic-channel-probe-v1"
            and report["authority"] == "none", "unexpected report schema or authority")
    require(report["artifact"] == artifact, "inspected artifact was substituted")
    require(report["probe_sha256"] == digest(probe_path.read_bytes())
            and report["worker_sha256"] == digest(worker_path.read_bytes()),
            "probe or worker executable identity mismatch")
    require(report["input_sha256"] == digest(inputs), "dispatched input identity mismatch")
    require(report["completed_dispatches"] == 1
            and report["input_immutability_and_all_guards_passed"] is True
            and report["free_close_and_worker_exit_passed"] is True,
            "missing completion, immutability, guards or cleanup")
    for field, filename in (("channels_sha256", "channels.u32le"),
                            ("output_sha256", "output.u32le")):
        data = (run_directory / filename).read_bytes()
        require(digest(data) == report[field], "raw result does not match report hash")
        require(decode_words(data) == expected["expected_words"],
                "independent exact-bit result mismatch")
    evidence = {
        "schema": "ferric-atomic-channel-numerical-evidence-v1",
        "authority": "none",
        "status": "pass",
        "scope": "per-index atomic storage; not cross-workgroup publication or model inference",
        "case": expected["case"],
        "reference_sha256": expected["reference_sha256"],
        "artifact_record_sha256": digest(artifact_bytes),
        "report_sha256": digest(report_bytes),
        "input_sha256": digest(inputs),
        "channels_sha256": report["channels_sha256"],
        "output_sha256": report["output_sha256"],
        "object_sha256": artifact["object_sha256"],
        "source_sha256": artifact["source_sha256"],
        "probe_sha256": report["probe_sha256"],
        "worker_sha256": report["worker_sha256"],
        "completed_dispatches": 1,
        "channel_words_compared": WORDS,
        "output_words_compared": WORDS,
        "exact_word_comparisons": WORDS * 2,
        "timing_claim": "none; host intervals are not GPU event time",
    }
    with (run_directory / "numerical.json").open("x") as stream:
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
