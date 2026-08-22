#!/usr/bin/env python3
"""Exercise deterministic and hostile Ferric M1 benchmark-suite inputs."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, NoReturn


SUITES = (
    "adversarial",
    "d10",
    "differential",
    "serving",
    "speculation",
)
INPUT_FORMAT = "FERRIC-M1-BENCHMARK-INPUT-V1"
RECORDS_FORMAT = "FERRIC-M1-BENCHMARK-RECORDS-V1"
DIFFERENTIAL_PAIRS_FORMAT = "FERRIC-M1-DIFFERENTIAL-PAIRS-V1"
DIFFERENTIAL_OUTPUT_FORMAT = "FERRIC-M1-DIFFERENTIAL-OUTPUT-V1"
TARGET = "gfx942:xnack-"
VOCABULARY_SIZE = 151_936


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def digest(label: str) -> str:
    return hashlib.sha256(label.encode("ascii")).hexdigest()


def invoke(
    repo: Path, suite: str, arguments: list[str], *, expected_status: int = 0
) -> subprocess.CompletedProcess[bytes]:
    command = [
        "cargo",
        "run",
        "--quiet",
        "--locked",
        "-p",
        "ferric-m1-benchmarks",
        "--bin",
        f"ferric-m1-{suite}",
        "--",
        *arguments,
    ]
    result = subprocess.run(
        command,
        cwd=repo,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != expected_status:
        fail(
            f"{suite} command returned {result.returncode}, expected "
            f"{expected_status}: {' '.join(arguments)}\n"
            f"{result.stdout.decode(errors='replace')}"
            f"{result.stderr.decode(errors='replace')}"
        )
    return result


def load_canonical(raw: bytes, description: str) -> dict[str, Any]:
    try:
        value = json.loads(raw)
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse {description}: {error}")
    if not isinstance(value, dict) or canonical_bytes(value) != raw:
        fail(f"{description} is not canonical JSON")
    return value


def write(path: Path, value: Any) -> None:
    path.write_bytes(canonical_bytes(value))


def plan_input(descriptor: dict[str, Any]) -> dict[str, Any]:
    identities = {
        identifier: digest(f"{descriptor['suite']}:identity:{identifier}")
        for identifier in descriptor["required_identities"]
    }
    cases = [
        {
            "id": f"{kind}.001",
            "input_sha256": digest(f"{descriptor['suite']}:input:{kind}"),
            "kind": kind,
            "workload_sha256": digest(f"{descriptor['suite']}:workload:{kind}"),
        }
        for kind in descriptor["case_kinds"]
    ]
    return {
        "cases": cases,
        "format": INPUT_FORMAT,
        "identities": identities,
        "suite": descriptor["suite"],
        "target": TARGET,
    }


def record_input(
    descriptor: dict[str, Any], plan: dict[str, Any], plan_raw: bytes
) -> dict[str, Any]:
    recorded = descriptor["minimum_recorded_samples"]
    attributes = {
        identifier: digest(f"{descriptor['suite']}:record:{identifier}")
        for identifier in descriptor["required_record_attributes"]
    }
    observations = []
    for case in plan["cases"]:
        measurements = {
            metric["id"]: [1] * recorded for metric in descriptor["raw_metrics"]
        }
        observations.append(
            {
                "attributes": attributes,
                "case_id": case["id"],
                "kind": case["kind"],
                "measurements": measurements,
                "recorded_samples": recorded,
                "status": "completed",
                "warmups": descriptor["minimum_warmups"],
            }
        )
    return {
        "format": RECORDS_FORMAT,
        "observations": observations,
        "plan_sha256": hashlib.sha256(plan_raw).hexdigest(),
        "suite": descriptor["suite"],
    }


def validate_descriptor(
    descriptor: dict[str, Any], requirements: dict[str, Any], suite: str
) -> None:
    if descriptor.get("suite") != suite or descriptor.get("target") != TARGET:
        fail(f"{suite} descriptor identity drifted")
    if descriptor.get("authority") != "benchmark-run-plan-only":
        fail(f"{suite} descriptor promotes its authority")
    nonclaim = descriptor.get("nonclaim")
    if not isinstance(nonclaim, str) or "does not" not in nonclaim or "close" not in nonclaim:
        fail(f"{suite} descriptor lacks an explicit nonclaim")
    obligation = next(
        (
            item
            for item in requirements["roadmap_requirements"]
            if item["id"] == descriptor.get("obligation_id")
        ),
        None,
    )
    path = next(
        (
            item
            for item in requirements["path_obligations"]
            if item["id"] == descriptor.get("path_id")
        ),
        None,
    )
    if obligation is None or obligation["obligation_state"] != "Open":
        fail(f"{suite} descriptor does not bind one open roadmap obligation")
    if path is None or path["obligation_state"] != "Open":
        fail(f"{suite} descriptor does not bind one open path obligation")
    if path["repository"] != "ferric" or path["path"] != descriptor.get("source_path"):
        fail(f"{suite} descriptor source binding drifted")
    for roster in (
        descriptor.get("case_kinds"),
        descriptor.get("required_identities"),
        descriptor.get("required_record_attributes"),
    ):
        if not isinstance(roster, list) or not roster or roster != sorted(set(roster)):
            fail(f"{suite} descriptor contains a weak or nondeterministic roster")


def differential_rows(kind: str) -> int:
    if kind in {
        "decode-s1-c8192",
        "prefill-s1-t128",
        "prefill-s1-t2048",
        "prefill-s1-t512",
    }:
        return 1
    if kind in {"decode-s8-c8192", "prefill-s8-t128"}:
        return 8
    if kind == "decode-s32-c8192":
        return 32
    fail(f"unknown differential case kind: {kind}")


def output_manifest(
    plan: dict[str, Any],
    case: dict[str, Any],
    producer: str,
    logits_path: Path,
    logits: bytes,
    tokens_path: Path,
    tokens: bytes,
    runner_transcript_sha256: str,
) -> dict[str, Any]:
    if producer == "ferric":
        producer_identity = "benchmark-executable"
        protocol_identity = "benchmark-protocol"
    else:
        producer_identity = "reference-implementation"
        protocol_identity = "reference-protocol"
    return {
        "authority": "externally-collected-model-output-only",
        "case_id": case["id"],
        "environment_sha256": plan["identities"]["environment"],
        "format": DIFFERENTIAL_OUTPUT_FORMAT,
        "input_sha256": case["input_sha256"],
        "kind": case["kind"],
        "logits": {
            "bytes": len(logits),
            "encoding": "bf16-le",
            "path": logits_path.name,
            "sha256": hashlib.sha256(logits).hexdigest(),
        },
        "plan_sha256": hashlib.sha256(canonical_bytes(plan)).hexdigest(),
        "producer": producer,
        "producer_sha256": plan["identities"][producer_identity],
        "protocol_sha256": plan["identities"][protocol_identity],
        "runner_transcript_sha256": runner_transcript_sha256,
        "shape": {
            "rows": differential_rows(case["kind"]),
            "vocabulary_size": VOCABULARY_SIZE,
        },
        "tokens": {
            "bytes": len(tokens),
            "encoding": "u32-le",
            "path": tokens_path.name,
            "sha256": hashlib.sha256(tokens).hexdigest(),
        },
        "workload_sha256": case["workload_sha256"],
    }


def exercise_differential_producer(
    repo: Path,
    scratch: Path,
    plan_path: Path,
    plan: dict[str, Any],
    plan_raw: bytes,
) -> None:
    pairs = []
    output_manifests: dict[str, Path] = {}
    runner_transcripts: dict[str, tuple[Path, bytes]] = {}
    for case in plan["cases"]:
        rows = differential_rows(case["kind"])
        logits = (0x3F80).to_bytes(2, "little") * (rows * VOCABULARY_SIZE)
        tokens = (0).to_bytes(4, "little") * rows
        runner_path = scratch / f"{case['id']}.runner.json"
        runner_bytes = canonical_bytes(
            {
                "case_id": case["id"],
                "status": "synthetic-policy-fixture-only",
            }
        )
        runner_path.write_bytes(runner_bytes)
        runner_sha256 = hashlib.sha256(runner_bytes).hexdigest()
        runner_transcripts[case["id"]] = (runner_path, runner_bytes)
        manifests = {}
        for producer in ("ferric", "reference"):
            prefix = f"{case['id']}.{producer}"
            logits_path = scratch / f"{prefix}.logits.bf16le"
            tokens_path = scratch / f"{prefix}.tokens.u32le"
            logits_path.write_bytes(logits)
            tokens_path.write_bytes(tokens)
            manifest_path = scratch / f"{prefix}.output.json"
            write(
                manifest_path,
                output_manifest(
                    plan,
                    case,
                    producer,
                    logits_path,
                    logits,
                    tokens_path,
                    tokens,
                    runner_sha256,
                ),
            )
            manifests[producer] = manifest_path
            output_manifests[f"{case['id']}:{producer}"] = manifest_path
        pairs.append(
            {
                "case_id": case["id"],
                "ferric_output_manifest": manifests["ferric"].name,
                "kind": case["kind"],
                "reference_output_manifest": manifests["reference"].name,
                "runner_transcript": {
                    "bytes": len(runner_bytes),
                    "path": runner_path.name,
                    "sha256": runner_sha256,
                },
            }
        )
    pairs_value = {
        "authority": "externally-collected-differential-pairs-only",
        "format": DIFFERENTIAL_PAIRS_FORMAT,
        "pairs": pairs,
        "plan_sha256": hashlib.sha256(plan_raw).hexdigest(),
        "suite": "differential",
    }
    pairs_path = scratch / "differential.pairs.json"
    write(pairs_path, pairs_value)

    raw_directory = scratch / "differential.raw"
    records_path = scratch / "differential.produced-records.json"
    invoke(
        repo,
        "differential",
        ["produce", str(plan_path), str(pairs_path), str(raw_directory), str(records_path)],
    )
    if len(list(raw_directory.iterdir())) != 7:
        fail("differential producer did not emit the exact raw-record roster")
    records = load_canonical(records_path.read_bytes(), "produced differential records")
    if len(records.get("observations", [])) != 7:
        fail("differential producer record roster drifted")
    for observation in records["observations"]:
        metrics = observation["measurements"]
        if (
            metrics["maximum-logit-ulp-error"] != [0]
            or metrics["token-mismatches"] != [0]
        ):
            fail("equal synthetic differential outputs did not compare exactly")
    transcript_path = scratch / "differential.produced-transcript.json"
    invoke(
        repo,
        "differential",
        ["validate", str(plan_path), str(records_path), str(transcript_path)],
    )

    first = plan["cases"][0]
    runner_path, runner_bytes = runner_transcripts[first["id"]]
    runner_path.write_bytes(runner_bytes + b"\n")
    invoke(
        repo,
        "differential",
        [
            "produce",
            str(plan_path),
            str(pairs_path),
            str(scratch / "substituted-runner.raw"),
            str(scratch / "substituted-runner.records.json"),
        ],
        expected_status=1,
    )
    runner_path.write_bytes(runner_bytes)

    ferric_manifest_path = output_manifests[f"{first['id']}:ferric"]
    ferric_manifest = load_canonical(
        ferric_manifest_path.read_bytes(), "Ferric output manifest"
    )
    ferric_manifest["producer_sha256"] = digest("substituted producer")
    write(ferric_manifest_path, ferric_manifest)
    invoke(
        repo,
        "differential",
        [
            "produce",
            str(plan_path),
            str(pairs_path),
            str(scratch / "substituted.raw"),
            str(scratch / "substituted.records.json"),
        ],
        expected_status=1,
    )

    ferric_manifest["producer_sha256"] = plan["identities"]["benchmark-executable"]
    wrong_tokens = (1).to_bytes(4, "little")
    wrong_tokens_path = scratch / ferric_manifest["tokens"]["path"]
    wrong_tokens_path.write_bytes(wrong_tokens)
    ferric_manifest["tokens"]["sha256"] = hashlib.sha256(wrong_tokens).hexdigest()
    write(ferric_manifest_path, ferric_manifest)
    invoke(
        repo,
        "differential",
        [
            "produce",
            str(plan_path),
            str(pairs_path),
            str(scratch / "wrong-argmax.raw"),
            str(scratch / "wrong-argmax.records.json"),
        ],
        expected_status=1,
    )

    wrong_tokens_path.write_bytes((0).to_bytes(4, "little"))
    ferric_manifest["tokens"]["sha256"] = hashlib.sha256(
        (0).to_bytes(4, "little")
    ).hexdigest()
    logits_path = scratch / ferric_manifest["logits"]["path"]
    logits = logits_path.read_bytes()
    nonfinite_logits = (0x7F80).to_bytes(2, "little") + logits[2:]
    logits_path.write_bytes(nonfinite_logits)
    ferric_manifest["logits"]["sha256"] = hashlib.sha256(nonfinite_logits).hexdigest()
    write(ferric_manifest_path, ferric_manifest)
    invoke(
        repo,
        "differential",
        [
            "produce",
            str(plan_path),
            str(pairs_path),
            str(scratch / "nonfinite.raw"),
            str(scratch / "nonfinite.records.json"),
        ],
        expected_status=1,
    )


def exercise_suite(
    repo: Path, scratch: Path, requirements: dict[str, Any], suite: str
) -> None:
    first = invoke(repo, suite, ["describe"]).stdout
    second = invoke(repo, suite, ["describe"]).stdout
    if first != second:
        fail(f"{suite} descriptor is nondeterministic")
    descriptor = load_canonical(first, f"{suite} descriptor")
    validate_descriptor(descriptor, requirements, suite)

    input_path = scratch / f"{suite}.input.json"
    plan_a = scratch / f"{suite}.plan-a.json"
    plan_b = scratch / f"{suite}.plan-b.json"
    write(input_path, plan_input(descriptor))
    invoke(repo, suite, ["plan", str(input_path), str(plan_a)])
    invoke(repo, suite, ["plan", str(input_path), str(plan_b)])
    if plan_a.read_bytes() != plan_b.read_bytes():
        fail(f"{suite} plans are nondeterministic")
    plan_raw = plan_a.read_bytes()
    plan = load_canonical(plan_raw, f"{suite} plan")
    if suite == "differential":
        exercise_differential_producer(repo, scratch, plan_a, plan, plan_raw)

    records_path = scratch / f"{suite}.records.json"
    transcript_path = scratch / f"{suite}.transcript.json"
    records = record_input(descriptor, plan, plan_raw)
    write(records_path, records)
    invoke(
        repo,
        suite,
        ["validate", str(plan_a), str(records_path), str(transcript_path)],
    )
    transcript = load_canonical(
        transcript_path.read_bytes(), f"{suite} ingestion transcript"
    )
    if (
        transcript.get("status") != "RECORDS_ACCEPTED"
        or transcript.get("authority")
        != "checked-benchmark-record-structure-only"
        or transcript.get("nonclaim") != descriptor["nonclaim"]
    ):
        fail(f"{suite} ingestion transcript promoted its authority")

    invoke(
        repo,
        suite,
        ["plan", str(input_path), str(plan_a)],
        expected_status=1,
    )

    records["plan_sha256"] = digest(f"{suite}:wrong-plan")
    wrong_identity = scratch / f"{suite}.wrong-identity.json"
    wrong_output = scratch / f"{suite}.wrong-identity.transcript.json"
    write(wrong_identity, records)
    invoke(
        repo,
        suite,
        ["validate", str(plan_a), str(wrong_identity), str(wrong_output)],
        expected_status=1,
    )

    records = record_input(descriptor, plan, plan_raw)
    records["observations"][0]["status"] = "failed"
    failed_record = scratch / f"{suite}.failed-record.json"
    failed_output = scratch / f"{suite}.failed-record.transcript.json"
    write(failed_record, records)
    invoke(
        repo,
        suite,
        ["validate", str(plan_a), str(failed_record), str(failed_output)],
        expected_status=1,
    )

    records = record_input(descriptor, plan, plan_raw)
    del records["observations"][0]["measurements"][descriptor["raw_metrics"][0]["id"]]
    missing_metric = scratch / f"{suite}.missing-metric.json"
    missing_output = scratch / f"{suite}.missing-metric.transcript.json"
    write(missing_metric, records)
    invoke(
        repo,
        suite,
        ["validate", str(plan_a), str(missing_metric), str(missing_output)],
        expected_status=1,
    )


def main() -> None:
    repo = Path(sys.argv[1] if len(sys.argv) == 2 else ".").resolve()
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = load_canonical(
        requirements_path.read_bytes(), "M1 requirements manifest"
    )
    with tempfile.TemporaryDirectory(prefix="ferric-m1-benchmark-policy.") as temporary:
        scratch = Path(temporary)
        for suite in SUITES:
            exercise_suite(repo, scratch, requirements, suite)
    print("PASS: Ferric M1 benchmark suites are deterministic and fail closed")


if __name__ == "__main__":
    main()
