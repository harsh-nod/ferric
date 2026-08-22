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
DIFFERENTIAL_ACCEPTANCE_POLICY_FORMAT = (
    "FERRIC-M1-DIFFERENTIAL-ACCEPTANCE-POLICY-V1"
)
ADVERSARIAL_EXECUTION_FORMAT = "FERRIC-M1-ADVERSARIAL-EXECUTION-V1"
ADVERSARIAL_OBSERVATION_FORMAT = "FERRIC-M1-ADVERSARIAL-OBSERVATION-V1"
ADVERSARIAL_RUNNER_TRANSCRIPT_FORMAT = (
    "FERRIC-M1-ADVERSARIAL-RUNNER-TRANSCRIPT-V1"
)
ADVERSARIAL_CANARY_LAYOUT_FORMAT = "FERRIC-M1-ADVERSARIAL-CANARY-LAYOUT-V1"
ADVERSARIAL_FAULT_PLAN_FORMAT = "FERRIC-M1-ADVERSARIAL-FAULT-PLAN-V1"
ADVERSARIAL_EXHAUSTION_FORMAT = "FERRIC-M1-ADVERSARIAL-EXHAUSTION-V1"
TARGET = "gfx942:xnack-"
VOCABULARY_SIZE = 151_936
DIFFERENTIAL_ACCEPTANCE_POLICY_NONCLAIM = (
    "This artifact supplies plan-admitted differential thresholds only. It does not "
    "establish independent review, numerical correctness, hardware correctness, "
    "qualification authority, or close m1.r29."
)
DIFFERENTIAL_ACCEPTANCE_RESULT_NONCLAIM = (
    "This result authenticates exact target-only differential comparisons against "
    "one plan-admitted threshold policy only. It does not establish an independently "
    "reviewed threshold, prove operator or graph refinement, establish hardware "
    "correctness, grant qualification authority, or close m1.r29."
)
ADVERSARIAL_POLICY_NONCLAIM = (
    "Synthetic policy fixture parser exercise only; this document is not a "
    "benchmark record or evidence and does not establish device execution, exact "
    "completion, fault injection, safety, hardware correctness, or close m1.r30."
)
ADVERSARIAL_EXTERNAL_NONCLAIM = (
    "Authenticated external report bytes and structural joins only; this transcript "
    "does not establish device execution, exact completion, fault injection, safety, "
    "hardware correctness, or close m1.r30."
)


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


def require_no_staging(parent: Path, output: Path) -> None:
    prefix = f".{output.name}.staging."
    if any(entry.name.startswith(prefix) for entry in parent.iterdir()):
        fail(f"failed producer left an owned staging bundle for {output.name}")


def companion(path: Path, contents: bytes) -> dict[str, Any]:
    return {
        "bytes": len(contents),
        "path": path.name,
        "sha256": hashlib.sha256(contents).hexdigest(),
    }


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


def differential_acceptance_policy(descriptor: dict[str, Any]) -> dict[str, Any]:
    return {
        "authority": "externally-admitted-differential-threshold-policy-only",
        "cases": [
            {
                "kind": kind,
                "maximum_logit_ulp_error": (
                    128 if kind == "decode-s32-c8192" else 0
                ),
                "maximum_token_mismatches": 0,
            }
            for kind in descriptor["case_kinds"]
        ],
        "finite_logits_required": True,
        "format": DIFFERENTIAL_ACCEPTANCE_POLICY_FORMAT,
        "logit_metric": (
            "maximum-monotonic-bf16-ulp-distance-signed-zero-equal"
        ),
        "nonclaim": DIFFERENTIAL_ACCEPTANCE_POLICY_NONCLAIM,
        "obligation_id": "m1.r29",
        "path_id": "differential-bench",
        "suite": "differential",
        "target": TARGET,
        "token_metric": "ferric-reference-greedy-token-mismatch-count",
        "token_selection": "lowest-token-id-bf16-argmax",
    }


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


def exercise_adversarial_producer(
    repo: Path, scratch: Path, descriptor: dict[str, Any]
) -> None:
    layout = {
        "format": ADVERSARIAL_CANARY_LAYOUT_FORMAT,
        "regions": [
            {"expected_byte": 165, "length": 2, "name": "prefix", "offset": 0},
            {"expected_byte": 90, "length": 2, "name": "suffix", "offset": 4},
        ],
    }
    fault_points = {
        "canary": ("guard-bytes", "canary-intact"),
        "cancellation": (
            "in-flight-retirement",
            "cancelled-after-exact-completion",
        ),
        "exhaustion": ("kv-page-allocation", "out-of-pages-transactional"),
        "fault-injection": ("queue-transition", "terminal-fault-quarantined"),
        "rollback": ("accepted-prefix", "strict-prefix-rollback-refined"),
    }
    fault_plan = {
        "faults": [
            {
                "case_kind": kind,
                "expected_outcome": fault_points[kind][1],
                "id": f"{kind}.policy-fixture",
                "injection_point": fault_points[kind][0],
            }
            for kind in descriptor["case_kinds"]
        ],
        "format": ADVERSARIAL_FAULT_PLAN_FORMAT,
    }
    layout_path = scratch / "adversarial.canary-layout.json"
    fault_plan_path = scratch / "adversarial.fault-plan.json"
    write(layout_path, layout)
    write(fault_plan_path, fault_plan)

    case_files: dict[str, tuple[Path, Path]] = {}
    cases = []
    for kind in descriptor["case_kinds"]:
        case_id = f"{kind}.producer"
        input_path = scratch / f"{case_id}.input.json"
        workload_path = scratch / f"{case_id}.workload.json"
        write(
            input_path,
            {
                "case_id": case_id,
                "kind": kind,
                "status": "synthetic-policy-fixture-only",
            },
        )
        if kind == "exhaustion":
            workload = {
                "format": ADVERSARIAL_EXHAUSTION_FORMAT,
                "kind": kind,
                "parameters": {
                    "first_append_tokens": 8,
                    "max_context_tokens": 12,
                    "page_count": 2,
                    "page_tokens": 4,
                    "rejected_append_tokens": 1,
                },
            }
        else:
            workload = {
                "case_id": case_id,
                "kind": kind,
                "status": "synthetic-policy-fixture-only",
            }
        write(workload_path, workload)
        case_files[kind] = (input_path, workload_path)
        cases.append(
            {
                "id": case_id,
                "input_sha256": hashlib.sha256(input_path.read_bytes()).hexdigest(),
                "kind": kind,
                "workload_sha256": hashlib.sha256(
                    workload_path.read_bytes()
                ).hexdigest(),
            }
        )

    identities = {
        identifier: digest(f"adversarial:producer:{identifier}")
        for identifier in descriptor["required_identities"]
    }
    identities["canary-layout"] = hashlib.sha256(layout_path.read_bytes()).hexdigest()
    identities["fault-plan"] = hashlib.sha256(fault_plan_path.read_bytes()).hexdigest()
    plan_input_path = scratch / "adversarial.producer.input.json"
    plan_path = scratch / "adversarial.producer.plan.json"
    write(
        plan_input_path,
        {
            "cases": cases,
            "format": INPUT_FORMAT,
            "identities": identities,
            "suite": "adversarial",
            "target": TARGET,
        },
    )
    invoke(
        repo,
        "adversarial",
        ["plan", str(plan_input_path), str(plan_path)],
    )
    plan_raw = plan_path.read_bytes()
    plan = load_canonical(plan_raw, "adversarial producer plan")
    plan_sha256 = hashlib.sha256(plan_raw).hexdigest()

    canary_before = bytes([0xA5, 0xA5, 1, 2, 0x5A, 0x5A])
    canary_after = canary_before
    canary_before_path = scratch / "canary.before.bin"
    canary_after_path = scratch / "canary.after.bin"
    canary_before_path.write_bytes(canary_before)
    canary_after_path.write_bytes(canary_after)

    observation_paths: dict[str, Path] = {}
    transcript_paths: dict[str, Path] = {}
    execution_cases = []
    for case in plan["cases"]:
        kind = case["kind"]
        input_path, workload_path = case_files[kind]
        observation_path = (
            None if kind == "exhaustion" else scratch / f"{case['id']}.observation.json"
        )
        if observation_path is not None:
            observation_paths[kind] = observation_path
            transcript_paths[kind] = scratch / f"{case['id']}.runner.json"
        execution_cases.append(
            {
                "case_id": case["id"],
                "input": input_path.name,
                "kind": kind,
                "observation": (
                    observation_path.name if observation_path is not None else None
                ),
                "workload": workload_path.name,
            }
        )

    execution_path = scratch / "adversarial.execution.json"
    write(
        execution_path,
        {
            "authority": "externally-supplied-adversarial-execution-only",
            "canary_layout": layout_path.name,
            "cases": execution_cases,
            "fault_plan": fault_plan_path.name,
            "format": ADVERSARIAL_EXECUTION_FORMAT,
            "plan_sha256": plan_sha256,
            "suite": "adversarial",
        },
    )
    execution_sha256 = hashlib.sha256(execution_path.read_bytes()).hexdigest()
    fault_plan_sha256 = hashlib.sha256(fault_plan_path.read_bytes()).hexdigest()
    result_semantics = {
        "canary": "guard-byte-snapshot-comparison",
        "cancellation": "completion-before-reclamation",
        "fault-injection": "terminal-queue-fault-quarantine",
        "rollback": "accepted-prefix-kv-rollback",
    }
    faults = {fault["case_kind"]: fault for fault in fault_plan["faults"]}

    for case in plan["cases"]:
        kind = case["kind"]
        if kind == "exhaustion":
            continue
        if kind == "canary":
            result = {
                "after": companion(canary_after_path, canary_after),
                "before": companion(canary_before_path, canary_before),
            }
        elif kind == "cancellation":
            result = {
                "completion_observed": True,
                "free_pages_after": 8,
                "free_pages_before": 8,
                "live_requests_after": 0,
                "reclaimed_after_completion": True,
                "reclaimed_before_completion": False,
            }
        elif kind == "fault-injection":
            result = {
                "failures_observed": 1,
                "faults_injected": 1,
                "live_resources_after": 0,
                "queue_quarantined": True,
                "retry_denied": True,
            }
        elif kind == "rollback":
            result = {
                "accepted_tokens": 2,
                "committed_tokens_after": 4,
                "committed_tokens_before": 2,
                "free_pages_after_cleanup": 8,
                "free_pages_before": 8,
                "live_requests_after_cleanup": 0,
                "resident_tokens_after": 4,
                "resident_tokens_before": 6,
            }
        else:
            fail(f"unknown adversarial producer kind: {kind}")
        fault = {
            "expected_outcome": faults[kind]["expected_outcome"],
            "id": faults[kind]["id"],
            "injection_point": faults[kind]["injection_point"],
            "occurrences": 1,
        }
        hardware_evidence = None
        if kind == "canary":
            hardware_evidence = {
                "device_identity_sha256": digest("synthetic device identity"),
                "environment_identity_sha256": plan["identities"]["environment"],
                "hardware_report_sha256": digest("synthetic hardware report"),
                "hardware_transcript_sha256": digest(
                    "synthetic hardware transcript"
                ),
                "harness_binary_sha256": plan["identities"][
                    "benchmark-executable"
                ],
                "harness_protocol_sha256": plan["identities"][
                    "benchmark-protocol"
                ],
            }
        transcript_path = transcript_paths[kind]
        transcript = {
            "authority": "synthetic-policy-fixture-only",
            "bindings": {
                "execution_sha256": execution_sha256,
                "fault_plan_sha256": fault_plan_sha256,
                "input_sha256": case["input_sha256"],
                "plan_sha256": plan_sha256,
                "workload_sha256": case["workload_sha256"],
            },
            "case_id": case["id"],
            "fault": fault,
            "format": ADVERSARIAL_RUNNER_TRANSCRIPT_FORMAT,
            "hardware_claim": "none",
            "hardware_evidence": hardware_evidence,
            "kind": kind,
            "nonclaim": ADVERSARIAL_POLICY_NONCLAIM,
            "planned_runner": {
                "environment_sha256": plan["identities"]["environment"],
                "executable_sha256": plan["identities"]["benchmark-executable"],
                "protocol_sha256": plan["identities"]["benchmark-protocol"],
            },
            "provenance": "synthetic-policy-fixture-only",
            "reported_outcome": faults[kind]["expected_outcome"],
            "result": result,
            "result_semantics": result_semantics[kind],
            "status": "synthetic-policy-fixture-only",
            "suite": "adversarial",
            "target": TARGET,
        }
        write(transcript_path, transcript)
        transcript_raw = transcript_path.read_bytes()
        write(
            observation_paths[kind],
            {
                "authority": "synthetic-policy-fixture-only",
                "case_id": case["id"],
                "execution_sha256": execution_sha256,
                "fault": fault,
                "fault_plan_sha256": fault_plan_sha256,
                "format": ADVERSARIAL_OBSERVATION_FORMAT,
                "input_sha256": case["input_sha256"],
                "kind": kind,
                "plan_sha256": plan_sha256,
                "provenance": "synthetic-policy-fixture-only",
                "result": result,
                "runner_transcript": companion(transcript_path, transcript_raw),
                "status": "synthetic-policy-fixture-only",
                "workload_sha256": case["workload_sha256"],
            },
        )

    invoke(
        repo,
        "adversarial",
        ["check-policy-fixture", str(plan_path), str(execution_path)],
    )
    output_bundle = scratch / "adversarial.fixture.bundle"
    invoke(
        repo,
        "adversarial",
        ["produce", str(plan_path), str(execution_path), str(output_bundle)],
        expected_status=1,
    )
    if output_bundle.exists():
        fail("production adversarial command published a synthetic fixture")
    require_no_staging(scratch, output_bundle)

    external_inputs = {
        path: path.read_bytes()
        for path in [*observation_paths.values(), *transcript_paths.values()]
    }
    for kind, transcript_path in transcript_paths.items():
        transcript = load_canonical(
            transcript_path.read_bytes(), f"{kind} external-intake transcript"
        )
        transcript["authority"] = (
            "externally-reported-adversarial-runner-transcript-only"
        )
        transcript["nonclaim"] = ADVERSARIAL_EXTERNAL_NONCLAIM
        transcript["provenance"] = "external-report"
        transcript["status"] = "reported-unvalidated"
        write(transcript_path, transcript)
        observation_path = observation_paths[kind]
        observation = load_canonical(
            observation_path.read_bytes(), f"{kind} external-intake observation"
        )
        observation["authority"] = (
            "externally-collected-adversarial-observation-only"
        )
        observation["provenance"] = "external-report"
        observation["runner_transcript"] = companion(
            transcript_path, transcript_path.read_bytes()
        )
        observation["status"] = "reported-unvalidated"
        write(observation_path, observation)

    external_bundle = scratch / "adversarial.reported-unvalidated.bundle"
    invoke(
        repo,
        "adversarial",
        ["produce", str(plan_path), str(execution_path), str(external_bundle)],
    )
    if (
        len(list((external_bundle / "raw").iterdir())) != 5
        or len(list((external_bundle / "transcripts").iterdir())) != 5
    ):
        fail("adversarial external-intake output roster drifted")
    records_path = external_bundle / "records.json"
    records = load_canonical(records_path.read_bytes(), "adversarial external intake")
    if len(records.get("observations", [])) != 5:
        fail("adversarial external-intake records omitted a case")
    for observation in records["observations"]:
        metrics = observation["measurements"]
        if observation.get("status") != "completed":
            fail("adversarial collection status drifted")
        if observation["kind"] == "exhaustion":
            if metrics["faults-observed"] != [1] or metrics["unexpected-errors"] != [0]:
                fail("logical exhaustion observation did not pass exactly")
        elif (
            metrics["faults-observed"] != [0]
            or metrics["unexpected-errors"] != [1]
        ):
            fail("external adversarial intake was promoted to a passing observation")
    for raw_path in (external_bundle / "raw").iterdir():
        raw = load_canonical(raw_path.read_bytes(), "adversarial raw intake record")
        expected_status = (
            "observed" if raw["kind"] == "exhaustion" else "reported-unvalidated"
        )
        if raw.get("status") != expected_status:
            fail("adversarial raw intake authority drifted")
    validated_path = scratch / "adversarial.external-intake.validation.json"
    invoke(
        repo,
        "adversarial",
        ["validate", str(plan_path), str(records_path), str(validated_path)],
    )
    for path, contents in external_inputs.items():
        path.write_bytes(contents)

    def reject_mutation(path: Path, description: str, mutate: Any) -> None:
        original = path.read_bytes()
        value = load_canonical(original, description)
        mutate(value)
        write(path, value)
        invoke(
            repo,
            "adversarial",
            ["check-policy-fixture", str(plan_path), str(execution_path)],
            expected_status=1,
        )
        path.write_bytes(original)

    def reject_transcript_mutation(kind: str, description: str, mutate: Any) -> None:
        transcript_path = transcript_paths[kind]
        observation_path = observation_paths[kind]
        original_transcript = transcript_path.read_bytes()
        original_observation = observation_path.read_bytes()
        value = load_canonical(original_transcript, description)
        mutate(value)
        write(transcript_path, value)
        observation = load_canonical(original_observation, f"{kind} fixture observation")
        replacement = transcript_path.read_bytes()
        observation["runner_transcript"] = companion(transcript_path, replacement)
        write(observation_path, observation)
        invoke(
            repo,
            "adversarial",
            ["check-policy-fixture", str(plan_path), str(execution_path)],
            expected_status=1,
        )
        transcript_path.write_bytes(original_transcript)
        observation_path.write_bytes(original_observation)

    cancellation_observation = observation_paths["cancellation"]
    reject_mutation(
        cancellation_observation,
        "cancellation fixture observation",
        lambda value: value["fault"].__setitem__("id", "wrong.policy-fixture"),
    )
    reject_mutation(
        cancellation_observation,
        "cancellation fixture observation",
        lambda value: value["fault"].__setitem__("injection_point", "guard-bytes"),
    )
    reject_mutation(
        cancellation_observation,
        "cancellation fixture observation",
        lambda value: value["fault"].__setitem__("occurrences", 2),
    )

    cancellation_transcript = transcript_paths["cancellation"]
    reject_transcript_mutation(
        "cancellation",
        "cancellation fixture transcript",
        lambda value: value["bindings"].__setitem__(
            "execution_sha256", digest("wrong execution")
        ),
    )
    reject_transcript_mutation(
        "cancellation",
        "cancellation fixture transcript",
        lambda value: value["planned_runner"].__setitem__(
            "environment_sha256", digest("wrong environment")
        ),
    )
    reject_transcript_mutation(
        "cancellation",
        "cancellation fixture transcript",
        lambda value: value.__setitem__(
            "hardware_evidence", {"device_identity_sha256": digest("device")}
        ),
    )
    reject_transcript_mutation(
        "cancellation",
        "cancellation fixture transcript",
        lambda value: value["result"].__setitem__("completion_observed", False),
    )
    reject_transcript_mutation(
        "cancellation",
        "cancellation fixture transcript",
        lambda value: value.__setitem__(
            "reported_outcome", "cancellation-boundary-violation"
        ),
    )
    transcript_raw = cancellation_transcript.read_bytes()
    observation_raw = cancellation_observation.read_bytes()
    cancellation_transcript.write_bytes(transcript_raw + b"\n")
    observation = load_canonical(observation_raw, "cancellation fixture observation")
    observation["runner_transcript"] = companion(
        cancellation_transcript, cancellation_transcript.read_bytes()
    )
    write(cancellation_observation, observation)
    invoke(
        repo,
        "adversarial",
        ["check-policy-fixture", str(plan_path), str(execution_path)],
        expected_status=1,
    )
    cancellation_transcript.write_bytes(transcript_raw)
    cancellation_observation.write_bytes(observation_raw)

    insecure_parent = scratch / "adversarial.insecure-output-parent"
    insecure_parent.mkdir(mode=0o700)
    insecure_parent.chmod(0o777)
    insecure_output = insecure_parent / "bundle"
    invoke(
        repo,
        "adversarial",
        ["produce", str(plan_path), str(execution_path), str(insecure_output)],
        expected_status=1,
    )
    if insecure_output.exists():
        fail("adversarial producer published beneath an untrusted output parent")
    require_no_staging(insecure_parent, insecure_output)
    insecure_parent.chmod(0o700)


def exercise_differential_producer(
    repo: Path,
    scratch: Path,
    plan_path: Path,
    plan: dict[str, Any],
    plan_raw: bytes,
    policy_path: Path,
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

    output_bundle = scratch / "differential.bundle"
    invoke(
        repo,
        "differential",
        ["produce", str(plan_path), str(pairs_path), str(output_bundle)],
    )
    raw_directory = output_bundle / "raw"
    records_path = output_bundle / "records.json"
    if len(list(raw_directory.iterdir())) != 7:
        fail("differential producer did not emit the exact raw-record roster")

    bare_output = scratch / "differential.bare.bundle"
    bare_result = subprocess.run(
        [
            str(repo / "target/debug/ferric-m1-differential"),
            "produce",
            plan_path.name,
            pairs_path.name,
            bare_output.name,
        ],
        cwd=scratch,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if bare_result.returncode != 0 or not (bare_output / "records.json").is_file():
        fail(
            "differential producer rejected bare current-directory paths:\n"
            + bare_result.stderr.decode(errors="replace")
        )
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

    acceptance_result = invoke(
        repo,
        "differential",
        [
            "check-acceptance",
            str(plan_path),
            str(pairs_path),
            str(policy_path),
        ],
    )
    acceptance = load_canonical(
        acceptance_result.stdout, "differential acceptance result"
    )
    if (
        acceptance.get("authority")
        != "checked-differential-policy-conformance-only"
        or acceptance.get("status") != "POLICY_CONFORMING"
        or acceptance.get("nonclaim") != DIFFERENTIAL_ACCEPTANCE_RESULT_NONCLAIM
        or len(acceptance.get("cases", [])) != 7
    ):
        fail("differential acceptance result promoted its authority")
    for case in acceptance["cases"]:
        if set(case) != {
            "case_id",
            "comparison",
            "ferric_output",
            "kind",
            "reference_output",
            "runner_transcript_sha256",
            "status",
            "threshold",
        }:
            fail("differential acceptance case omitted an identity binding")
        for producer in ("ferric_output", "reference_output"):
            if set(case[producer]) != {
                "logits_bytes",
                "logits_sha256",
                "manifest_sha256",
                "tokens_bytes",
                "tokens_sha256",
            }:
                fail("differential acceptance output identity schema drifted")

    content_case = plan["cases"][0]
    original_contents: dict[Path, bytes] = {}
    for producer in ("ferric", "reference"):
        manifest_path = output_manifests[f"{content_case['id']}:{producer}"]
        manifest_bytes = manifest_path.read_bytes()
        manifest = load_canonical(
            manifest_bytes, f"{producer} content-substitution output manifest"
        )
        logits_path = scratch / manifest["logits"]["path"]
        logits = logits_path.read_bytes()
        changed_logits = (0x3F81).to_bytes(2, "little") + logits[2:]
        original_contents[manifest_path] = manifest_bytes
        original_contents[logits_path] = logits
        logits_path.write_bytes(changed_logits)
        manifest["logits"]["sha256"] = hashlib.sha256(changed_logits).hexdigest()
        write(manifest_path, manifest)
    substituted_result = invoke(
        repo,
        "differential",
        [
            "check-acceptance",
            str(plan_path),
            str(pairs_path),
            str(policy_path),
        ],
    )
    substituted = load_canonical(
        substituted_result.stdout, "content-substituted differential acceptance result"
    )
    if substituted_result.stdout == acceptance_result.stdout:
        fail("different valid output content left acceptance identity unchanged")
    if (
        substituted["pairs_sha256"] != acceptance["pairs_sha256"]
        or [case["comparison"] for case in substituted["cases"]]
        != [case["comparison"] for case in acceptance["cases"]]
    ):
        fail("content-substitution fixture did not preserve pairs and comparison metrics")
    original_case = next(
        case for case in acceptance["cases"] if case["case_id"] == content_case["id"]
    )
    substituted_case = next(
        case for case in substituted["cases"] if case["case_id"] == content_case["id"]
    )
    if (
        substituted_case["ferric_output"] == original_case["ferric_output"]
        or substituted_case["reference_output"] == original_case["reference_output"]
        or substituted_case["runner_transcript_sha256"]
        != original_case["runner_transcript_sha256"]
    ):
        fail("acceptance result did not isolate output content from runner identity")
    for path, contents in original_contents.items():
        path.write_bytes(contents)

    invoke(
        repo,
        "differential",
        ["check-acceptance", str(plan_path), str(pairs_path)],
        expected_status=1,
    )

    policy_bytes = policy_path.read_bytes()
    policy = load_canonical(policy_bytes, "differential acceptance policy")
    policy["cases"][0]["maximum_logit_ulp_error"] = 1
    write(policy_path, policy)
    invoke(
        repo,
        "differential",
        [
            "check-acceptance",
            str(plan_path),
            str(pairs_path),
            str(policy_path),
        ],
        expected_status=1,
    )
    policy_path.write_bytes(policy_bytes)

    first_case = plan["cases"][0]
    first_manifest_path = output_manifests[f"{first_case['id']}:ferric"]
    first_manifest = load_canonical(
        first_manifest_path.read_bytes(), "first Ferric output manifest"
    )
    first_logits_path = scratch / first_manifest["logits"]["path"]
    first_logits = first_logits_path.read_bytes()
    changed_logits = (0x3F81).to_bytes(2, "little") + first_logits[2:]
    first_logits_path.write_bytes(changed_logits)
    first_manifest["logits"]["sha256"] = hashlib.sha256(changed_logits).hexdigest()
    write(first_manifest_path, first_manifest)
    invoke(
        repo,
        "differential",
        [
            "check-acceptance",
            str(plan_path),
            str(pairs_path),
            str(policy_path),
        ],
        expected_status=1,
    )
    first_logits_path.write_bytes(first_logits)
    first_manifest["logits"]["sha256"] = hashlib.sha256(first_logits).hexdigest()
    write(first_manifest_path, first_manifest)

    token_case = next(
        case for case in plan["cases"] if case["kind"] == "decode-s32-c8192"
    )
    token_manifest_path = output_manifests[f"{token_case['id']}:ferric"]
    token_manifest = load_canonical(
        token_manifest_path.read_bytes(), "token-mutation Ferric output manifest"
    )
    token_logits_path = scratch / token_manifest["logits"]["path"]
    token_payload_path = scratch / token_manifest["tokens"]["path"]
    token_logits = token_logits_path.read_bytes()
    token_payload = token_payload_path.read_bytes()
    changed_logits = token_logits[:2] + (0x4000).to_bytes(2, "little") + token_logits[4:]
    changed_tokens = (1).to_bytes(4, "little") + token_payload[4:]
    token_logits_path.write_bytes(changed_logits)
    token_payload_path.write_bytes(changed_tokens)
    token_manifest["logits"]["sha256"] = hashlib.sha256(changed_logits).hexdigest()
    token_manifest["tokens"]["sha256"] = hashlib.sha256(changed_tokens).hexdigest()
    write(token_manifest_path, token_manifest)
    invoke(
        repo,
        "differential",
        [
            "check-acceptance",
            str(plan_path),
            str(pairs_path),
            str(policy_path),
        ],
        expected_status=1,
    )
    token_logits_path.write_bytes(token_logits)
    token_payload_path.write_bytes(token_payload)
    token_manifest["logits"]["sha256"] = hashlib.sha256(token_logits).hexdigest()
    token_manifest["tokens"]["sha256"] = hashlib.sha256(token_payload).hexdigest()
    write(token_manifest_path, token_manifest)
    transcript_path = scratch / "differential.produced-transcript.json"
    invoke(
        repo,
        "differential",
        ["validate", str(plan_path), str(records_path), str(transcript_path)],
    )
    records_before = records_path.read_bytes()
    invoke(
        repo,
        "differential",
        ["produce", str(plan_path), str(pairs_path), str(output_bundle)],
        expected_status=1,
    )
    if records_path.read_bytes() != records_before:
        fail("no-replace publication modified an existing output bundle")
    require_no_staging(scratch, output_bundle)

    plan_parent_link = scratch / "plan-parent-link"
    os.symlink(scratch, plan_parent_link)
    linked_plan_bundle = scratch / "linked-plan.bundle"
    invoke(
        repo,
        "differential",
        [
            "produce",
            str(plan_parent_link / plan_path.name),
            str(pairs_path),
            str(linked_plan_bundle),
        ],
        expected_status=1,
    )
    if linked_plan_bundle.exists():
        fail("producer published output after a symlinked plan traversal")
    plan_parent_link.unlink()

    with tempfile.TemporaryDirectory(
        prefix="ferric-m1-differential-escape."
    ) as outside_raw:
        outside = Path(outside_raw)
        first_id = plan["cases"][0]["id"]
        escaped_manifest = output_manifests[f"{first_id}:ferric"]
        (outside / escaped_manifest.name).write_bytes(escaped_manifest.read_bytes())
        escape_link = scratch / "manifest-escape"
        os.symlink(outside, escape_link)
        pairs_value["pairs"][0]["ferric_output_manifest"] = (
            f"{escape_link.name}/{escaped_manifest.name}"
        )
        write(pairs_path, pairs_value)
        escaped_bundle = scratch / "escaped-manifest.bundle"
        invoke(
            repo,
            "differential",
            ["produce", str(plan_path), str(pairs_path), str(escaped_bundle)],
            expected_status=1,
        )
        if escaped_bundle.exists():
            fail("producer published output after an intermediate symlink escape")
        escape_link.unlink()
    pairs_value["pairs"][0]["ferric_output_manifest"] = output_manifests[
        f"{plan['cases'][0]['id']}:ferric"
    ].name
    write(pairs_path, pairs_value)

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
            str(scratch / "substituted-runner.bundle"),
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
            str(scratch / "substituted.bundle"),
        ],
        expected_status=1,
    )

    ferric_manifest["producer_sha256"] = plan["identities"]["benchmark-executable"]
    write(ferric_manifest_path, ferric_manifest)
    reference_manifest_path = output_manifests[f"{first['id']}:reference"]
    reference_manifest = load_canonical(
        reference_manifest_path.read_bytes(), "reference output manifest"
    )
    ferric_tokens_path = scratch / ferric_manifest["tokens"]["path"]
    reference_tokens_path = scratch / reference_manifest["tokens"]["path"]
    ferric_tokens = ferric_tokens_path.read_bytes()
    ferric_tokens_path.unlink()
    os.link(reference_tokens_path, ferric_tokens_path)
    aliased_bundle = scratch / "hardlink-alias.bundle"
    invoke(
        repo,
        "differential",
        ["produce", str(plan_path), str(pairs_path), str(aliased_bundle)],
        expected_status=1,
    )
    if aliased_bundle.exists():
        fail("producer published output for hard-linked differential inputs")
    ferric_tokens_path.unlink()
    ferric_tokens_path.write_bytes(ferric_tokens)

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
            str(scratch / "retry.bundle"),
        ],
        expected_status=1,
    )
    retry_bundle = scratch / "retry.bundle"
    if retry_bundle.exists():
        fail("failed comparison published a partial output bundle")
    require_no_staging(scratch, retry_bundle)

    wrong_tokens_path.write_bytes((0).to_bytes(4, "little"))
    ferric_manifest["tokens"]["sha256"] = hashlib.sha256(
        (0).to_bytes(4, "little")
    ).hexdigest()
    write(ferric_manifest_path, ferric_manifest)
    invoke(
        repo,
        "differential",
        ["produce", str(plan_path), str(pairs_path), str(retry_bundle)],
    )
    if len(list((retry_bundle / "raw").iterdir())) != 7:
        fail("retry did not publish the exact differential bundle")

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
            str(scratch / "nonfinite.bundle"),
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
    input_value = plan_input(descriptor)
    policy_path = scratch / "differential.acceptance-policy.json"
    if suite == "differential":
        write(policy_path, differential_acceptance_policy(descriptor))
        input_value["identities"]["differential-acceptance-policy"] = (
            hashlib.sha256(policy_path.read_bytes()).hexdigest()
        )
    write(input_path, input_value)
    invoke(repo, suite, ["plan", str(input_path), str(plan_a)])
    invoke(repo, suite, ["plan", str(input_path), str(plan_b)])
    if plan_a.read_bytes() != plan_b.read_bytes():
        fail(f"{suite} plans are nondeterministic")
    plan_raw = plan_a.read_bytes()
    plan = load_canonical(plan_raw, f"{suite} plan")
    if suite == "adversarial":
        exercise_adversarial_producer(repo, scratch, descriptor)
    if suite == "differential":
        exercise_differential_producer(
            repo, scratch, plan_a, plan, plan_raw, policy_path
        )

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
