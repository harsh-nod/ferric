#!/usr/bin/env python3
"""Exercise canonical and hostile M1 MI300X hardware transcripts."""

from __future__ import annotations

import copy
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.hardware-transcript.v1"
REPORT_FORMAT = "FERRIC-M1-HARDWARE-TRANSCRIPT-REPORT-V1"
TRANSCRIPT_FORMAT = "FERRIC-M1-MI300X-HARDWARE-RUN-V1"
ROSTER_FORMAT = "FERRIC-M1-HARDWARE-CASE-ROSTER-V1"
TEST_PROTOCOL = "ferric.m1.mi300x-hardware-test.v1"
ARTIFACT_TARGET = "gfx942:xnack-"
AUTHORITY = "hardware-observation-only"
NONCLAIM = (
    "This report authenticates one bounded binding-local observation from the "
    "exact named MI300X hardware run. It does not establish path-specific "
    "semantics, reproducible binary provenance, independently attest "
    "operator-declared environment identities, prove machine refinement, or "
    "establish performance or M1 qualification."
)
FERRIC_BASE = "c5a86fd56c1c817664593df25c04bbed30e84971"
DEVICE_UUID = "123e4567-e89b-42d3-a456-426614174000"
TOOL_SOURCE_PATHS = {
    "cargo_lock": "Cargo.lock",
    "hardware_harness": "crates/ferric-engine/src/bin/ferric-m1-hardware-harness.rs",
    "package_manifest": "crates/ferric-engine/Cargo.toml",
    "packet_execution": "crates/ferric-engine/src/m1_packet_diagnostic_execution.rs",
    "persisted_kernel_artifacts": "crates/ferric-engine/src/persisted_kernel_artifacts.rs",
}
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = ("Compiler", "Hardware", "Runtime")
Case = tuple[str, str, str, str, str]
CASES: tuple[Case, ...] = (
    ("roadmap-composition", "Roadmap", "m1.r05", "composition", "generated-runner"),
    ("roadmap-kernel-ferric", "Roadmap", "m1.r06", "kernel", "ferric-gemm"),
    (
        "roadmap-runtime-fe2o3",
        "Roadmap",
        "m1.r14",
        "runtime",
        "fe2o3-aql-foundation",
    ),
    (
        "assurance-composition",
        "Assurance",
        "graph_refined",
        "composition",
        "generated-runner",
    ),
    (
        "assurance-runtime",
        "Assurance",
        "kv_refined",
        "runtime",
        "device-cache",
    ),
    (
        "assurance-qualification",
        "Assurance",
        "target_conforming",
        "qualification",
        "d10-bench",
    ),
)
Fixture = tuple[
    Path,
    Path,
    Path,
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
]
Mutation = Callable[[Fixture], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def canonical_digest(value: Any) -> str:
    return digest_bytes(
        json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
    )


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        fail(f"cannot load {path}")
    sys.dont_write_bytecode = True
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def refresh_binding(context: dict[str, Any]) -> None:
    binding = context["binding"]
    binding["binding_sha256"] = canonical_digest(
        {key: value for key, value in binding.items() if key != "binding_sha256"}
    )


def refresh_report(fixture: Fixture) -> None:
    report_path, _, _, context, report, _, _ = fixture
    raw = canonical_bytes(report)
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)


def refresh_transcript(fixture: Fixture) -> None:
    _, transcript_path, _, _, report, transcript, _ = fixture
    raw = canonical_bytes(transcript)
    transcript_path.write_bytes(raw)
    report["transcript_sha256"] = digest_bytes(raw)
    report["transcript_size_bytes"] = len(raw)
    report["device_identity_sha256"] = canonical_digest(transcript["device"])
    report["environment_identity_sha256"] = canonical_digest(transcript["environment"])
    report["kernel_manifest_sha256"] = transcript["kernel_manifest_sha256"]
    report["kernel_catalog_sha256"] = transcript["kernel_catalog_sha256"]
    refresh_report(fixture)


def refresh_roster(fixture: Fixture) -> None:
    _, _, roster_path, _, report, transcript, roster = fixture
    raw = canonical_bytes(roster)
    roster_path.write_bytes(raw)
    transcript["case_roster_sha256"] = digest_bytes(raw)
    transcript["case_roster_size_bytes"] = len(raw)
    report["case_roster_sha256"] = digest_bytes(raw)
    report["case_roster_size_bytes"] = len(raw)
    refresh_transcript(fixture)


def requirement_spec(
    requirements: dict[str, Any], obligation_class: str, obligation_id: str
) -> tuple[str, list[str]]:
    if obligation_class == "Roadmap":
        record = next(
            item
            for item in requirements["roadmap_requirements"]
            if item["id"] == obligation_id
        )
        return record["title"], record["assurance_properties"]
    record = next(
        item
        for item in requirements["assurance_properties"]
        if item["name"] == obligation_id
    )
    return record["boundary"], [obligation_id]


def make_fixture(repo: Path, root: Path, case: Case = CASES[0]) -> Fixture:
    name, obligation_class, obligation_id, profile, path_id = case
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="ascii"))
    procedure_path = repo / "proofs/m1-qualification/hardware-k7-procedure.json"
    procedure = json.loads(procedure_path.read_text(encoding="ascii"))
    statement, assurance_properties = requirement_spec(
        requirements, obligation_class, obligation_id
    )
    path_record = next(
        item for item in requirements["path_obligations"] if item["id"] == path_id
    )
    artifact_id = f"artifact.hardware.{name}"
    binding_id = f"binding.hardware.{name}"
    report_relative = f"artifacts/{artifact_id}.hardware-transcript.json"
    transcript_relative = f"hardware-transcripts/{artifact_id}.json"
    roster_relative = f"hardware-rosters/{artifact_id}.json"
    report_path = root / report_relative
    transcript_path = root / transcript_relative
    roster_path = root / roster_relative
    report_path.parent.mkdir(parents=True, exist_ok=True)
    transcript_path.parent.mkdir(parents=True, exist_ok=True)
    roster_path.parent.mkdir(parents=True, exist_ok=True)

    sources = [
        {
            "base_commit": requirements["m1_upstream_base_commit"],
            "commit": digest_bytes(b"fe2o3 commit")[:40],
            "id": "source.fe2o3",
            "repository": "fe2o3",
            "source_closure_artifact_id": "artifact.source.fe2o3",
            "source_closure_sha256": digest_bytes(b"fe2o3 source closure"),
            "tree": digest_bytes(b"fe2o3 tree")[:40],
        },
        {
            "base_commit": FERRIC_BASE,
            "commit": digest_bytes(b"ferric commit")[:40],
            "id": "source.ferric",
            "repository": "ferric",
            "source_closure_artifact_id": "artifact.source.ferric",
            "source_closure_sha256": digest_bytes(b"ferric source closure"),
            "tree": digest_bytes(b"ferric tree")[:40],
        },
    ]
    source_closures = [
        {
            "commit": record["commit"],
            "id": record["id"],
            "repository": record["repository"],
            "source_closure_sha256": record["source_closure_sha256"],
            "tree": record["tree"],
        }
        for record in sources
    ]
    tcb = [
        {
            "artifact_id": f"artifact.{identifier}",
            "id": identifier,
            "identity_sha256": digest_bytes(identifier.encode("ascii")),
            "kind": kind,
        }
        for identifier, kind in zip(TCB_IDS, TCB_KINDS, strict=True)
    ]
    tcb_identities = {item["id"]: item["identity_sha256"] for item in tcb}
    binding = {
        "artifact_id": artifact_id,
        "binding_sha256": "",
        "evidence_kind": "hardware-test",
        "id": binding_id,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "path_id": path_id,
        "profile_id": profile,
        "source_identity_id": f"source.{path_record['repository']}",
        "statement_sha256": digest_bytes(statement.encode("utf-8")),
        "tcb_ids": list(TCB_IDS),
    }
    context = {
        "artifact": {
            "id": artifact_id,
            "kind": "HardwareTranscript",
            "path": report_relative,
            "sha256": "",
            "size_bytes": 0,
        },
        "artifact_absolute_path": str(report_path),
        "binding": binding,
        "format": "ferric.m1-evidence-index.v1",
        "path_resolution": {
            "availability": path_record["availability"],
            "id": path_id,
            "path": path_record["path"],
            "repository": path_record["repository"],
            "source_identity_id": f"source.{path_record['repository']}",
        },
        "requirements_sha256": digest_file(requirements_path),
        "sources": sources,
        "subject": f"binding:{binding_id}",
        "tcb": tcb,
    }
    refresh_binding(context)
    cases = [
        {
            "assurance_property_ids": copy.deepcopy(assurance_properties),
            "id": f"case.k7.{binding_id.replace('.', '-')}",
            "obligation_class": obligation_class,
            "obligation_id": obligation_id,
            "path_id": path_id,
            "procedure_sha256": digest_file(procedure_path),
            "profile_id": profile,
            "requires_gpu_work": True,
        }
    ]
    roster = {
        "binding_sha256": binding["binding_sha256"],
        "cases": cases,
        "device_uuid": DEVICE_UUID,
        "format": ROSTER_FORMAT,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "path_id": path_id,
        "profile_id": profile,
        "protocol": TEST_PROTOCOL,
        "requirements_sha256": context["requirements_sha256"],
        "source_closures": copy.deepcopy(source_closures),
        "source_identity_id": binding["source_identity_id"],
        "target": ARTIFACT_TARGET,
        "tcb_identity_sha256s": copy.deepcopy(tcb_identities),
    }
    device = {
        "device_count": 1,
        "device_uuid": DEVICE_UUID,
        "marketing_name": "AMD Instinct MI300X",
        "pci_bdf": "0000:41:00.0",
        "processor": "gfx942",
        "vendor_id": "1002",
        "xnack": "disabled",
    }
    environment = {
        "driver": {
            "module_sha256": digest_bytes(b"amdgpu module"),
            "name": "amdgpu",
            "version": "6.12.20",
        },
        "firmware": {
            "bundle_sha256": digest_bytes(b"amdgpu firmware bundle"),
            "package_version": "20260801",
        },
        "rocm": {
            "installation_sha256": digest_bytes(b"rocm installation closure"),
            "version": "7.1.0",
        },
        "tool": {
            "binary_sha256": procedure["harness_binary"]["sha256"],
            "binary_size_bytes": procedure["harness_binary"]["size_bytes"],
            "name": "ferric-m1-hardware-harness",
            "protocol": TEST_PROTOCOL,
            "source_sha256s": {
                key: digest_file(repo / relative)
                for key, relative in TOOL_SOURCE_PATHS.items()
            },
            "version": "1.0.0",
        },
    }
    roster_raw = canonical_bytes(roster)
    roster_path.write_bytes(roster_raw)
    kernel_manifest_sha256 = digest_bytes(b"authenticated kernel manifest")
    kernel_catalog_sha256 = digest_bytes(b"authenticated kernel catalog")
    generation = 7
    observation = (
        "ferric-m1-k7-observation-v1|"
        f"{binding['binding_sha256']}|{cases[0]['id']}|"
        f"{cases[0]['procedure_sha256']}|{kernel_manifest_sha256}|"
        f"{kernel_catalog_sha256}|{DEVICE_UUID}|0000:41:00.0|{generation}|"
        "10,11,12,13,14\n"
    ).encode("ascii")
    case_results = [
        {
            "binding_sha256": binding["binding_sha256"],
            "case_id": cases[0]["id"],
            "completion_count": 1,
            "generation": generation,
            "gpu_observation_sha256": digest_bytes(observation),
            "grid": [64, 1, 1],
            "launch_count": 1,
            "output_tokens": [10, 11, 12, 13, 14],
            "output_verified": True,
            "procedure_sha256": cases[0]["procedure_sha256"],
            "program": "k7-speculative-token-assembly-s1k4",
            "queue_released": True,
            "result": "pass",
            "workgroup": [64, 1, 1],
        }
    ]
    transcript = {
        "binding_sha256": binding["binding_sha256"],
        "case_results": case_results,
        "case_roster_sha256": digest_bytes(roster_raw),
        "case_roster_size_bytes": len(roster_raw),
        "device": device,
        "environment": environment,
        "finished_at_utc": "2026-08-21T20:01:00Z",
        "format": TRANSCRIPT_FORMAT,
        "gpu_work_completed": True,
        "gpu_work_submitted": True,
        "kernel_catalog_sha256": kernel_catalog_sha256,
        "kernel_manifest_sha256": kernel_manifest_sha256,
        "no_gpu_work": False,
        "protocol": TEST_PROTOCOL,
        "requirements_sha256": context["requirements_sha256"],
        "result": "pass",
        "run_id": f"run.{name}",
        "source_closures": copy.deepcopy(source_closures),
        "started_at_utc": "2026-08-21T20:00:00Z",
        "target": ARTIFACT_TARGET,
        "tcb_identity_sha256s": copy.deepcopy(tcb_identities),
    }
    transcript_raw = canonical_bytes(transcript)
    transcript_path.write_bytes(transcript_raw)
    source_digests = {
        record["id"]: record["source_closure_sha256"] for record in sources
    }
    report = {
        "assurance_property_ids": copy.deepcopy(assurance_properties),
        "authority": AUTHORITY,
        "binding_sha256": binding["binding_sha256"],
        "case_count": len(cases),
        "case_roster_relative_path": roster_relative,
        "case_roster_sha256": digest_bytes(roster_raw),
        "case_roster_size_bytes": len(roster_raw),
        "device_identity_sha256": canonical_digest(device),
        "evidence_kind": "hardware-test",
        "environment_identity_sha256": canonical_digest(environment),
        "format": REPORT_FORMAT,
        "gpu_work_observed": True,
        "kernel_catalog_sha256": kernel_catalog_sha256,
        "kernel_manifest_sha256": kernel_manifest_sha256,
        "nonclaim": NONCLAIM,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "obligation_state": "Open",
        "passed_case_count": len(cases),
        "path_id": path_id,
        "path_resolution_sha256": canonical_digest(context["path_resolution"]),
        "profile_id": profile,
        "requirements_sha256": context["requirements_sha256"],
        "result": "observed-pass",
        "source_closure_sha256s": source_digests,
        "source_identity_id": binding["source_identity_id"],
        "source_roster_sha256": canonical_digest(sources),
        "statement_sha256": binding["statement_sha256"],
        "target": ARTIFACT_TARGET,
        "tcb_identity_sha256s": copy.deepcopy(tcb_identities),
        "tcb_roster_sha256": canonical_digest(tcb),
        "test_protocol": TEST_PROTOCOL,
        "total_gpu_completions": 1,
        "total_gpu_launches": 1,
        "transcript_relative_path": transcript_relative,
        "transcript_sha256": digest_bytes(transcript_raw),
        "transcript_size_bytes": len(transcript_raw),
    }
    fixture = (
        report_path,
        transcript_path,
        roster_path,
        context,
        report,
        transcript,
        roster,
    )
    refresh_report(fixture)
    return fixture


def invoke(
    validator: Path,
    context: dict[str, Any],
    *,
    protocol: str = PROTOCOL,
    raw_context: bytes | None = None,
) -> subprocess.CompletedProcess[bytes]:
    if raw_context is None:
        raw_context = (
            json.dumps(
                context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
            )
            + "\n"
        ).encode("ascii")
    return subprocess.run(
        [sys.executable, "-I", str(validator), protocol],
        check=False,
        input=raw_context,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=15,
    )


def canonical_cases(repo: Path, validator: Path, root: Path) -> None:
    for case in CASES:
        fixture = make_fixture(repo, root / case[0], case)
        context = fixture[3]
        result = invoke(validator, context)
        context_payload = json.dumps(
            context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
        expected = (
            f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
            f"context_sha256={digest_bytes(context_payload)}\n"
        ).encode("ascii")
        if result.returncode != 0 or result.stdout != expected:
            fail(
                f"canonical {case[0]} transcript rejected: "
                f"exit={result.returncode}, output={result.stdout!r}"
            )


def hostile_cases(repo: Path, validator: Path, root: Path) -> int:
    count = 0

    def run_hostile(name: str, mutation: Mutation, case: Case = CASES[0]) -> None:
        nonlocal count
        fixture = make_fixture(repo, root / name, case)
        mutation(fixture)
        if invoke(validator, fixture[3]).returncode == 0:
            fail(f"hostile hardware-transcript input was accepted: {name}")
        count += 1

    def report_field(key: str, value: Any) -> Mutation:
        def mutate(fixture: Fixture) -> None:
            fixture[4][key] = copy.deepcopy(value)
            refresh_report(fixture)

        return mutate

    report_mutations: list[tuple[str, Mutation]] = [
        ("report-format", report_field("format", REPORT_FORMAT + "-DRIFT")),
        ("authority-proof", report_field("authority", "proof-authority")),
        ("authority-machine", report_field("authority", "machine-refinement")),
        ("nonclaim-weak", report_field("nonclaim", "Hardware passed.")),
        ("evidence-kind", report_field("evidence_kind", "performance-gate")),
        ("result-promotion", report_field("result", "qualified")),
        ("target", report_field("target", "gfx950:xnack-")),
        ("test-protocol", report_field("test_protocol", "self.selected.v1")),
        ("gpu-observed-false", report_field("gpu_work_observed", False)),
        ("gpu-observed-string", report_field("gpu_work_observed", "true")),
        ("binding", report_field("binding_sha256", digest_bytes(b"binding"))),
        ("obligation-class", report_field("obligation_class", "Assurance")),
        ("obligation-id", report_field("obligation_id", "m1.r06")),
        ("status-promotion", report_field("obligation_state", "Closed")),
        ("property-omission", report_field("assurance_property_ids", [])),
        (
            "property-reorder",
            report_field(
                "assurance_property_ids",
                ["resource_bounded", "artifact_authenticated", "graph_refined"],
            ),
        ),
        ("profile", report_field("profile_id", "runtime")),
        ("path", report_field("path_id", "physical-runner")),
        ("path-resolution", report_field("path_resolution_sha256", digest_bytes(b"p"))),
        ("requirements", report_field("requirements_sha256", digest_bytes(b"r"))),
        ("source-id", report_field("source_identity_id", "source.fe2o3")),
        (
            "source-closure-omission",
            report_field(
                "source_closure_sha256s",
                {"source.ferric": digest_bytes(b"ferric")},
            ),
        ),
        ("source-roster", report_field("source_roster_sha256", digest_bytes(b"s"))),
        ("statement", report_field("statement_sha256", digest_bytes(b"statement"))),
        (
            "tcb-omission",
            report_field("tcb_identity_sha256s", {"tcb.compiler": digest_bytes(b"x")}),
        ),
        ("tcb-roster", report_field("tcb_roster_sha256", digest_bytes(b"tcb"))),
        ("case-count", report_field("case_count", 2)),
        ("case-count-bool", report_field("case_count", True)),
        ("passed-count", report_field("passed_case_count", 2)),
        ("launch-count", report_field("total_gpu_launches", 2)),
        ("launch-count-bool", report_field("total_gpu_launches", True)),
        ("completion-count", report_field("total_gpu_completions", 2)),
        ("device-digest", report_field("device_identity_sha256", digest_bytes(b"d"))),
        (
            "kernel-manifest-report",
            report_field("kernel_manifest_sha256", digest_bytes(b"manifest")),
        ),
        (
            "kernel-catalog-report",
            report_field("kernel_catalog_sha256", digest_bytes(b"catalog")),
        ),
        (
            "environment-digest",
            report_field("environment_identity_sha256", digest_bytes(b"e")),
        ),
        (
            "roster-traversal",
            report_field("case_roster_relative_path", "../roster.json"),
        ),
        (
            "roster-substitution",
            report_field("case_roster_relative_path", "hardware-rosters/other.json"),
        ),
        ("roster-sha", report_field("case_roster_sha256", digest_bytes(b"roster"))),
        ("roster-size", report_field("case_roster_size_bytes", 1)),
        ("roster-size-bool", report_field("case_roster_size_bytes", True)),
        (
            "transcript-traversal",
            report_field("transcript_relative_path", "../transcript.json"),
        ),
        (
            "transcript-substitution",
            report_field("transcript_relative_path", "hardware-transcripts/other.json"),
        ),
        ("transcript-sha", report_field("transcript_sha256", digest_bytes(b"run"))),
        ("transcript-size", report_field("transcript_size_bytes", 1)),
        ("transcript-size-bool", report_field("transcript_size_bytes", True)),
    ]
    for name, mutation in report_mutations:
        run_hostile(name, mutation)

    def context_mutation(function: Callable[[dict[str, Any]], None]) -> Mutation:
        def mutate(fixture: Fixture) -> None:
            function(fixture[3])

        return mutate

    context_mutations: list[tuple[str, Mutation]] = [
        (
            "outer-kind",
            context_mutation(lambda c: c["artifact"].__setitem__("kind", "TcbReport")),
        ),
        (
            "outer-id",
            context_mutation(lambda c: c["artifact"].__setitem__("id", "artifact.x")),
        ),
        (
            "outer-path",
            context_mutation(
                lambda c: c["artifact"].__setitem__("path", "artifacts/x.json")
            ),
        ),
        (
            "outer-sha",
            context_mutation(
                lambda c: c["artifact"].__setitem__("sha256", digest_bytes(b"x"))
            ),
        ),
        (
            "outer-size",
            context_mutation(lambda c: c["artifact"].__setitem__("size_bytes", 1)),
        ),
        ("subject", context_mutation(lambda c: c.__setitem__("subject", "binding:x"))),
        (
            "binding-kind",
            context_mutation(
                lambda c: c["binding"].__setitem__("evidence_kind", "verus-theorem")
            ),
        ),
        (
            "binding-artifact",
            context_mutation(
                lambda c: c["binding"].__setitem__("artifact_id", "artifact.x")
            ),
        ),
        (
            "binding-digest",
            context_mutation(
                lambda c: c["binding"].__setitem__(
                    "binding_sha256", digest_bytes(b"binding")
                )
            ),
        ),
        (
            "binding-profile",
            context_mutation(
                lambda c: c["binding"].__setitem__("profile_id", "runtime")
            ),
        ),
        (
            "binding-path",
            context_mutation(
                lambda c: c["binding"].__setitem__("path_id", "physical-runner")
            ),
        ),
        (
            "binding-statement",
            context_mutation(
                lambda c: c["binding"].__setitem__(
                    "statement_sha256", digest_bytes(b"statement")
                )
            ),
        ),
        (
            "binding-tcb-order",
            context_mutation(lambda c: c["binding"]["tcb_ids"].reverse()),
        ),
        (
            "path-availability",
            context_mutation(
                lambda c: c["path_resolution"].__setitem__(
                    "availability", "ExistingFoundation"
                )
            ),
        ),
        (
            "path-file",
            context_mutation(
                lambda c: c["path_resolution"].__setitem__("path", "docs/ROADMAP.md")
            ),
        ),
        (
            "path-repository",
            context_mutation(
                lambda c: c["path_resolution"].__setitem__("repository", "fe2o3")
            ),
        ),
        (
            "path-source",
            context_mutation(
                lambda c: c["path_resolution"].__setitem__(
                    "source_identity_id", "source.fe2o3"
                )
            ),
        ),
        ("source-order", context_mutation(lambda c: c["sources"].reverse())),
        (
            "source-duplicate",
            context_mutation(
                lambda c: c["sources"].__setitem__(1, copy.deepcopy(c["sources"][0]))
            ),
        ),
        (
            "source-base",
            context_mutation(
                lambda c: c["sources"][1].__setitem__(
                    "base_commit", digest_bytes(b"base")[:40]
                )
            ),
        ),
        (
            "source-commit-placeholder",
            context_mutation(lambda c: c["sources"][0].__setitem__("commit", "a" * 40)),
        ),
        (
            "source-repository",
            context_mutation(
                lambda c: c["sources"][0].__setitem__("repository", "ferric")
            ),
        ),
        (
            "source-closure",
            context_mutation(
                lambda c: c["sources"][0].__setitem__(
                    "source_closure_sha256", digest_bytes(b"closure")
                )
            ),
        ),
        ("tcb-order", context_mutation(lambda c: c["tcb"].reverse())),
        (
            "tcb-duplicate",
            context_mutation(
                lambda c: c["tcb"].__setitem__(1, copy.deepcopy(c["tcb"][0]))
            ),
        ),
        (
            "tcb-kind",
            context_mutation(lambda c: c["tcb"][0].__setitem__("kind", "Runtime")),
        ),
        (
            "tcb-identity",
            context_mutation(
                lambda c: c["tcb"][0].__setitem__(
                    "identity_sha256", digest_bytes(b"tcb")
                )
            ),
        ),
        (
            "requirements-context",
            context_mutation(
                lambda c: c.__setitem__("requirements_sha256", digest_bytes(b"r"))
            ),
        ),
        (
            "context-format",
            context_mutation(
                lambda c: c.__setitem__("format", "ferric.m1-evidence-index.v2")
            ),
        ),
    ]
    for name, mutation in context_mutations:
        run_hostile(name, mutation)

    def roster_mutation(function: Callable[[dict[str, Any]], None]) -> Mutation:
        def mutate(fixture: Fixture) -> None:
            function(fixture[6])
            refresh_roster(fixture)

        return mutate

    roster_mutations: list[tuple[str, Mutation]] = [
        ("roster-format", roster_mutation(lambda r: r.__setitem__("format", "other"))),
        (
            "roster-protocol",
            roster_mutation(lambda r: r.__setitem__("protocol", "self.selected.v1")),
        ),
        (
            "roster-target",
            roster_mutation(lambda r: r.__setitem__("target", "gfx942:xnack+")),
        ),
        (
            "roster-binding",
            roster_mutation(
                lambda r: r.__setitem__("binding_sha256", digest_bytes(b"binding"))
            ),
        ),
        (
            "roster-requirements",
            roster_mutation(
                lambda r: r.__setitem__("requirements_sha256", digest_bytes(b"r"))
            ),
        ),
        (
            "roster-obligation",
            roster_mutation(lambda r: r.__setitem__("obligation_id", "m1.r06")),
        ),
        (
            "roster-profile",
            roster_mutation(lambda r: r.__setitem__("profile_id", "runtime")),
        ),
        (
            "roster-path",
            roster_mutation(lambda r: r.__setitem__("path_id", "physical-runner")),
        ),
        (
            "roster-source-id",
            roster_mutation(
                lambda r: r.__setitem__("source_identity_id", "source.fe2o3")
            ),
        ),
        (
            "roster-source-order",
            roster_mutation(lambda r: r["source_closures"].reverse()),
        ),
        (
            "roster-source-commit",
            roster_mutation(
                lambda r: r["source_closures"][0].__setitem__(
                    "commit", digest_bytes(b"other")[:40]
                )
            ),
        ),
        (
            "roster-tcb",
            roster_mutation(
                lambda r: r.__setitem__(
                    "tcb_identity_sha256s", {"tcb.compiler": digest_bytes(b"x")}
                )
            ),
        ),
        (
            "roster-device-uuid",
            roster_mutation(
                lambda r: r.__setitem__(
                    "device_uuid", "00000000-0000-4000-8000-000000000000"
                )
            ),
        ),
        ("cases-empty", roster_mutation(lambda r: r.__setitem__("cases", []))),
        (
            "case-injection",
            roster_mutation(lambda r: r["cases"].append(copy.deepcopy(r["cases"][0]))),
        ),
        (
            "case-duplicate",
            roster_mutation(lambda r: r["cases"].append(copy.deepcopy(r["cases"][0]))),
        ),
        (
            "case-id",
            roster_mutation(lambda r: r["cases"][0].__setitem__("id", "Bad Case")),
        ),
        (
            "case-properties",
            roster_mutation(
                lambda r: r["cases"][0].__setitem__("assurance_property_ids", [])
            ),
        ),
        (
            "case-obligation",
            roster_mutation(
                lambda r: r["cases"][0].__setitem__("obligation_id", "m1.r06")
            ),
        ),
        (
            "case-profile",
            roster_mutation(
                lambda r: r["cases"][0].__setitem__("profile_id", "runtime")
            ),
        ),
        (
            "case-path",
            roster_mutation(
                lambda r: r["cases"][0].__setitem__("path_id", "physical-runner")
            ),
        ),
        (
            "case-no-gpu",
            roster_mutation(
                lambda r: r["cases"][0].__setitem__("requires_gpu_work", False)
            ),
        ),
        (
            "case-procedure-placeholder",
            roster_mutation(
                lambda r: r["cases"][0].__setitem__("procedure_sha256", "a" * 64)
            ),
        ),
        (
            "case-extra-field",
            roster_mutation(lambda r: r["cases"][0].__setitem__("performance", 1.0)),
        ),
        (
            "roster-extra-field",
            roster_mutation(lambda r: r.__setitem__("qualified", True)),
        ),
    ]
    for name, mutation in roster_mutations:
        run_hostile(name, mutation)

    def transcript_mutation(function: Callable[[dict[str, Any]], None]) -> Mutation:
        def mutate(fixture: Fixture) -> None:
            function(fixture[5])
            refresh_transcript(fixture)

        return mutate

    transcript_mutations: list[tuple[str, Mutation]] = [
        (
            "run-format",
            transcript_mutation(lambda t: t.__setitem__("format", "other")),
        ),
        (
            "run-protocol",
            transcript_mutation(
                lambda t: t.__setitem__("protocol", "self.selected.v1")
            ),
        ),
        (
            "run-target",
            transcript_mutation(lambda t: t.__setitem__("target", "gfx950:xnack-")),
        ),
        (
            "run-binding",
            transcript_mutation(
                lambda t: t.__setitem__("binding_sha256", digest_bytes(b"binding"))
            ),
        ),
        (
            "run-requirements",
            transcript_mutation(
                lambda t: t.__setitem__("requirements_sha256", digest_bytes(b"r"))
            ),
        ),
        (
            "run-roster-sha",
            transcript_mutation(
                lambda t: t.__setitem__("case_roster_sha256", digest_bytes(b"roster"))
            ),
        ),
        (
            "run-roster-size",
            transcript_mutation(lambda t: t.__setitem__("case_roster_size_bytes", 1)),
        ),
        (
            "run-roster-size-bool",
            transcript_mutation(
                lambda t: t.__setitem__("case_roster_size_bytes", True)
            ),
        ),
        (
            "run-kernel-manifest",
            transcript_mutation(
                lambda t: t.__setitem__(
                    "kernel_manifest_sha256", digest_bytes(b"manifest drift")
                )
            ),
        ),
        (
            "run-kernel-catalog",
            transcript_mutation(
                lambda t: t.__setitem__(
                    "kernel_catalog_sha256", digest_bytes(b"catalog drift")
                )
            ),
        ),
        (
            "run-source-order",
            transcript_mutation(lambda t: t["source_closures"].reverse()),
        ),
        (
            "run-source-tree",
            transcript_mutation(
                lambda t: t["source_closures"][1].__setitem__(
                    "tree", digest_bytes(b"tree")[:40]
                )
            ),
        ),
        (
            "run-tcb",
            transcript_mutation(
                lambda t: t.__setitem__(
                    "tcb_identity_sha256s", {"tcb.hardware": digest_bytes(b"x")}
                )
            ),
        ),
        (
            "run-id",
            transcript_mutation(lambda t: t.__setitem__("run_id", "Bad Run")),
        ),
        (
            "start-timestamp",
            transcript_mutation(
                lambda t: t.__setitem__("started_at_utc", "2026-08-21 20:00:00")
            ),
        ),
        (
            "finish-before-start",
            transcript_mutation(
                lambda t: t.__setitem__("finished_at_utc", "2026-08-21T19:59:00Z")
            ),
        ),
        (
            "no-submit",
            transcript_mutation(lambda t: t.__setitem__("gpu_work_submitted", False)),
        ),
        (
            "no-complete",
            transcript_mutation(lambda t: t.__setitem__("gpu_work_completed", False)),
        ),
        (
            "false-no-gpu-claim",
            transcript_mutation(lambda t: t.__setitem__("no_gpu_work", True)),
        ),
        (
            "gpu-flag-string",
            transcript_mutation(lambda t: t.__setitem__("gpu_work_submitted", "true")),
        ),
        (
            "run-result",
            transcript_mutation(lambda t: t.__setitem__("result", "qualified")),
        ),
        (
            "device-count-zero",
            transcript_mutation(lambda t: t["device"].__setitem__("device_count", 0)),
        ),
        (
            "device-count-two",
            transcript_mutation(lambda t: t["device"].__setitem__("device_count", 2)),
        ),
        (
            "device-count-bool",
            transcript_mutation(
                lambda t: t["device"].__setitem__("device_count", True)
            ),
        ),
        (
            "device-name",
            transcript_mutation(
                lambda t: t["device"].__setitem__("marketing_name", "AMD GPU")
            ),
        ),
        (
            "device-processor",
            transcript_mutation(
                lambda t: t["device"].__setitem__("processor", "gfx950")
            ),
        ),
        (
            "device-vendor",
            transcript_mutation(lambda t: t["device"].__setitem__("vendor_id", "10de")),
        ),
        (
            "device-xnack",
            transcript_mutation(lambda t: t["device"].__setitem__("xnack", "enabled")),
        ),
        (
            "device-uuid",
            transcript_mutation(
                lambda t: t["device"].__setitem__(
                    "device_uuid", "223e4567-e89b-42d3-a456-426614174000"
                )
            ),
        ),
        (
            "device-uuid-placeholder",
            transcript_mutation(
                lambda t: t["device"].__setitem__(
                    "device_uuid", "00000000-0000-4000-8000-000000000000"
                )
            ),
        ),
        (
            "device-bdf",
            transcript_mutation(
                lambda t: t["device"].__setitem__("pci_bdf", "41:00.0")
            ),
        ),
        (
            "rocm-version-empty",
            transcript_mutation(
                lambda t: t["environment"]["rocm"].__setitem__("version", "")
            ),
        ),
        (
            "rocm-identity-placeholder",
            transcript_mutation(
                lambda t: t["environment"]["rocm"].__setitem__(
                    "installation_sha256", "a" * 64
                )
            ),
        ),
        (
            "driver-name",
            transcript_mutation(
                lambda t: t["environment"]["driver"].__setitem__("name", "nouveau")
            ),
        ),
        (
            "driver-sha",
            transcript_mutation(
                lambda t: t["environment"]["driver"].__setitem__(
                    "module_sha256", "b" * 64
                )
            ),
        ),
        (
            "firmware-version-newline",
            transcript_mutation(
                lambda t: t["environment"]["firmware"].__setitem__(
                    "package_version", "one\ntwo"
                )
            ),
        ),
        (
            "firmware-sha",
            transcript_mutation(
                lambda t: t["environment"]["firmware"].__setitem__(
                    "bundle_sha256", "c" * 64
                )
            ),
        ),
        (
            "tool-name",
            transcript_mutation(
                lambda t: t["environment"]["tool"].__setitem__("name", "other")
            ),
        ),
        (
            "tool-protocol",
            transcript_mutation(
                lambda t: t["environment"]["tool"].__setitem__("protocol", "other.v1")
            ),
        ),
        (
            "tool-binary",
            transcript_mutation(
                lambda t: t["environment"]["tool"].__setitem__(
                    "binary_sha256", "d" * 64
                )
            ),
        ),
        (
            "tool-binary-size",
            transcript_mutation(
                lambda t: t["environment"]["tool"].__setitem__("binary_size_bytes", 1)
            ),
        ),
        (
            "tool-version-empty",
            transcript_mutation(
                lambda t: t["environment"]["tool"].__setitem__("version", "")
            ),
        ),
        (
            "tool-source",
            transcript_mutation(
                lambda t: t["environment"]["tool"]["source_sha256s"].__setitem__(
                    "cargo_lock", digest_bytes(b"wrong source")
                )
            ),
        ),
        (
            "results-empty",
            transcript_mutation(lambda t: t.__setitem__("case_results", [])),
        ),
        (
            "result-injection",
            transcript_mutation(
                lambda t: t["case_results"].append(copy.deepcopy(t["case_results"][0]))
            ),
        ),
        (
            "result-duplicate",
            transcript_mutation(
                lambda t: t["case_results"].append(copy.deepcopy(t["case_results"][0]))
            ),
        ),
        (
            "result-case-id",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("case_id", "other.case")
            ),
        ),
        (
            "result-binding",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__(
                    "binding_sha256", digest_bytes(b"wrong binding")
                )
            ),
        ),
        (
            "result-procedure",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__(
                    "procedure_sha256", digest_bytes(b"wrong procedure")
                )
            ),
        ),
        (
            "result-generation-zero",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("generation", 0)
            ),
        ),
        (
            "result-generation-bool",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("generation", True)
            ),
        ),
        (
            "result-program",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("program", "K7")
            ),
        ),
        (
            "result-grid",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("grid", [1, 1, 1])
            ),
        ),
        (
            "result-grid-bool",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("grid", [True, 1, 1])
            ),
        ),
        (
            "result-workgroup",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("workgroup", [32, 1, 1])
            ),
        ),
        (
            "result-output",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__(
                    "output_tokens", [10, 11, 12, 13, 15]
                )
            ),
        ),
        (
            "result-output-unverified",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("output_verified", False)
            ),
        ),
        (
            "result-queue-live",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("queue_released", False)
            ),
        ),
        (
            "case-result-fail",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("result", "fail")
            ),
        ),
        (
            "case-zero-launches",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("launch_count", 0)
            ),
        ),
        (
            "case-launch-bool",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("launch_count", True)
            ),
        ),
        (
            "case-missing-completion",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("completion_count", 0)
            ),
        ),
        (
            "case-extra-completion",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("completion_count", 2)
            ),
        ),
        (
            "case-observation-placeholder",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__(
                    "gpu_observation_sha256", "e" * 64
                )
            ),
        ),
        (
            "result-extra-field",
            transcript_mutation(
                lambda t: t["case_results"][0].__setitem__("latency_ms", 1)
            ),
        ),
        (
            "run-extra-field",
            transcript_mutation(lambda t: t.__setitem__("machine_refined", True)),
        ),
    ]
    for name, mutation in transcript_mutations:
        run_hostile(name, mutation)

    def hostile_file(name: str, function: Callable[[Fixture], None]) -> None:
        run_hostile(name, function)

    def tamper_roster(fixture: Fixture) -> None:
        fixture[2].write_bytes(b"substituted\n")

    hostile_file("roster-tamper", tamper_roster)

    def tamper_transcript(fixture: Fixture) -> None:
        fixture[1].write_bytes(b"substituted\n")

    hostile_file("transcript-tamper", tamper_transcript)

    def noncanonical_report(fixture: Fixture) -> None:
        raw = (json.dumps(fixture[4], ensure_ascii=True, sort_keys=True) + "\n").encode(
            "ascii"
        )
        fixture[0].write_bytes(raw)
        fixture[3]["artifact"]["sha256"] = digest_bytes(raw)
        fixture[3]["artifact"]["size_bytes"] = len(raw)

    hostile_file("report-noncanonical", noncanonical_report)

    def noncanonical_roster(fixture: Fixture) -> None:
        raw = (json.dumps(fixture[6], ensure_ascii=True, sort_keys=True) + "\n").encode(
            "ascii"
        )
        fixture[2].write_bytes(raw)
        fixture[5]["case_roster_sha256"] = digest_bytes(raw)
        fixture[5]["case_roster_size_bytes"] = len(raw)
        fixture[4]["case_roster_sha256"] = digest_bytes(raw)
        fixture[4]["case_roster_size_bytes"] = len(raw)
        refresh_transcript(fixture)

    hostile_file("roster-noncanonical", noncanonical_roster)

    def noncanonical_transcript(fixture: Fixture) -> None:
        raw = (json.dumps(fixture[5], ensure_ascii=True, sort_keys=True) + "\n").encode(
            "ascii"
        )
        fixture[1].write_bytes(raw)
        fixture[4]["transcript_sha256"] = digest_bytes(raw)
        fixture[4]["transcript_size_bytes"] = len(raw)
        refresh_report(fixture)

    hostile_file("transcript-noncanonical", noncanonical_transcript)

    def duplicate_json(path_index: int, target: bytes, replacement: bytes) -> Mutation:
        def mutate(fixture: Fixture) -> None:
            path = fixture[path_index]
            raw = path.read_bytes().replace(target, replacement, 1)
            path.write_bytes(raw)
            if path_index == 0:
                fixture[3]["artifact"]["sha256"] = digest_bytes(raw)
                fixture[3]["artifact"]["size_bytes"] = len(raw)
            elif path_index == 1:
                fixture[4]["transcript_sha256"] = digest_bytes(raw)
                fixture[4]["transcript_size_bytes"] = len(raw)
                refresh_report(fixture)
            else:
                fixture[5]["case_roster_sha256"] = digest_bytes(raw)
                fixture[5]["case_roster_size_bytes"] = len(raw)
                fixture[4]["case_roster_sha256"] = digest_bytes(raw)
                fixture[4]["case_roster_size_bytes"] = len(raw)
                refresh_transcript(fixture)

        return mutate

    hostile_file(
        "report-duplicate-key",
        duplicate_json(
            0,
            b'{\n  "assurance_property_ids":',
            b'{\n  "format": "duplicate",\n  "assurance_property_ids":',
        ),
    )
    hostile_file(
        "transcript-duplicate-key",
        duplicate_json(
            1,
            b'{\n  "binding_sha256":',
            b'{\n  "format": "duplicate",\n  "binding_sha256":',
        ),
    )
    hostile_file(
        "roster-duplicate-key",
        duplicate_json(
            2,
            b'{\n  "binding_sha256":',
            b'{\n  "format": "duplicate",\n  "binding_sha256":',
        ),
    )

    def symlink_file(path_index: int) -> Mutation:
        def mutate(fixture: Fixture) -> None:
            path = fixture[path_index]
            target = path.parent / "target.json"
            path.rename(target)
            path.symlink_to(target)

        return mutate

    hostile_file("report-symlink", symlink_file(0))
    hostile_file("transcript-symlink", symlink_file(1))
    hostile_file("roster-symlink", symlink_file(2))

    def symlink_parent(path_index: int) -> Mutation:
        def mutate(fixture: Fixture) -> None:
            parent = fixture[path_index].parent
            target = parent.with_name(parent.name + "-target")
            parent.rename(target)
            parent.symlink_to(target, target_is_directory=True)

        return mutate

    hostile_file("transcript-parent-symlink", symlink_parent(1))

    def hardlink_file(path_index: int) -> Mutation:
        def mutate(fixture: Fixture) -> None:
            os.link(fixture[path_index], fixture[path_index].with_suffix(".hardlink"))

        return mutate

    hostile_file("report-hardlink", hardlink_file(0))
    hostile_file("transcript-hardlink", hardlink_file(1))
    hostile_file("roster-hardlink", hardlink_file(2))

    def report_extra_field(fixture: Fixture) -> None:
        fixture[4]["proved"] = True
        refresh_report(fixture)

    hostile_file("report-extra-field", report_extra_field)

    fixture = make_fixture(repo, root / "raw-context")
    context = fixture[3]
    canonical = json.dumps(
        context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    raw_cases = {
        "empty": b"",
        "noncanonical": (
            json.dumps(context, ensure_ascii=True, sort_keys=True) + "\n"
        ).encode("ascii"),
        "duplicate": (canonical + "\n")
        .encode("ascii")
        .replace(b'{"artifact":', b'{"format":"duplicate","artifact":', 1),
        "extra-newline": (canonical + "\n\n").encode("ascii"),
        "missing-newline": canonical.encode("ascii"),
        "non-ascii": (canonical + "\n")
        .encode("ascii")
        .replace(b"hardware", b"hard\xffware", 1),
        "oversized": b"{" + b"x" * 1_000_001,
    }
    for name, raw in raw_cases.items():
        if invoke(validator, context, raw_context=raw).returncode == 0:
            fail(f"hostile raw hardware context was accepted: {name}")
        count += 1
    extra = copy.deepcopy(context)
    extra["validator_path"] = "self-selected.py"
    if invoke(validator, extra).returncode == 0:
        fail("index-selected hardware validator path was accepted")
    count += 1
    if invoke(validator, context, protocol=PROTOCOL + ".drift").returncode == 0:
        fail("wrong hardware-transcript protocol was accepted")
    count += 1

    return count


def audit_toctou_guard(repo: Path, validator: Path, root: Path) -> None:
    module = load_module(validator, "ferric_m1_hardware_validator_toctou")
    fixture = make_fixture(repo, root)
    original = module.file_identity
    calls = 0
    custody = module.InputCustody()
    evidence_descriptor = custody.open_absolute_directory(root, "TOCTOU fixture root")

    def drifting_identity(metadata: os.stat_result) -> tuple[int, ...]:
        nonlocal calls
        calls += 1
        identity = original(metadata)
        if calls == 3:
            return (*identity[:-1], identity[-1] + 1)
        return identity

    module.file_identity = drifting_identity
    try:
        try:
            with contextlib.redirect_stderr(io.StringIO()):
                custody.hold_relative_regular(
                    evidence_descriptor,
                    module.safe_relative(
                        fixture[4]["transcript_relative_path"],
                        "TOCTOU transcript path",
                    ),
                    module.MAX_TRANSCRIPT_BYTES,
                    "TOCTOU fixture",
                )
        except SystemExit:
            pass
        else:
            fail("hardware validator did not reject an in-read identity change")
    finally:
        module.file_identity = original
        custody.close()


def audit_parent_rebinding(repo: Path, validator: Path, root: Path) -> None:
    module = load_module(validator, "ferric_m1_hardware_validator_parent_rebinding")
    fixture = make_fixture(repo, root)
    original = module.InputCustody.revalidate
    rebound = False

    def rebind_then_revalidate(custody: Any) -> None:
        nonlocal rebound
        parent = fixture[1].parent
        moved = parent.with_name(parent.name + "-opened")
        parent.rename(moved)
        parent.mkdir()
        rebound = True
        original(custody)

    module.InputCustody.revalidate = rebind_then_revalidate
    try:
        try:
            with contextlib.redirect_stderr(io.StringIO()):
                module.validate(fixture[3])
        except SystemExit:
            pass
        else:
            fail("hardware validator accepted a rebound companion parent")
    finally:
        module.InputCustody.revalidate = original
    if not rebound:
        fail("hardware validator parent-rebinding hook did not execute")


def audit_checker_pin(repo: Path, validator: Path) -> None:
    checker = load_module(
        repo / "proofs/check-m1-evidence-index.py", "ferric_m1_evidence_checker"
    )
    expected = (
        "proofs/m1/evidence/validate-hardware-transcript.py",
        PROTOCOL,
        digest_file(validator),
    )
    if checker.TRUSTED_VALIDATORS.get("hardware-test") != expected:
        fail("checker-owned hardware-transcript path, protocol, or source pin drifted")


def audit_open_requirements(repo: Path) -> None:
    requirements = json.loads(
        (repo / "proofs/M1_REQUIREMENTS.json").read_text(encoding="ascii")
    )
    roadmaps = requirements["roadmap_requirements"]
    properties = requirements["assurance_properties"]
    if (
        len(roadmaps) != 33
        or len(properties) != 17
        or any(item["obligation_state"] != "Open" for item in roadmaps)
        or any(item["obligation_state"] != "Open" for item in properties)
    ):
        fail("M1 roadmap or assurance state was changed by hardware validation")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: test-hardware-transcript-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-hardware-transcript.py"
    audit_checker_pin(repo, validator)
    audit_open_requirements(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-hardware-transcript.") as raw:
        root = Path(raw)
        canonical_cases(repo, validator, root / "canonical")
        hostile_count = hostile_cases(repo, validator, root / "hostile")
        audit_toctou_guard(repo, validator, root / "toctou")
        audit_parent_rebinding(repo, validator, root / "parent-rebinding")
    print(
        "PASS: M1 hardware-transcript validator accepted 6 canonical transcripts "
        f"and rejected {hostile_count} hostile fixtures, an in-read TOCTOU change, "
        "and a parent rebinding"
    )


if __name__ == "__main__":
    main()
