#!/usr/bin/env python3
"""Exercise canonical and hostile M1 independent-validator reports."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.independent-validator.v1"
REPORT_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-REPORT-V1"
ROSTER_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-ROSTER-V1"
TRANSCRIPT_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-TRANSCRIPT-V1"
VALIDATOR_PROTOCOL = "ferric.external-independent-validation.v1"
TARGET = "gfx942:xnack-"
AUTHORITY = "independent-validation-observation-only"
INDEPENDENCE_ATTESTATION = (
    "The named checker organization, repository, source closure, and executable "
    "are independent of the Ferric and fe2o3 subject source closures."
)
NONCLAIM = (
    "This report authenticates an independent checker identity and its exact "
    "case observations only. Observations are not a theorem, machine refinement, "
    "load, launch, hardware, performance, or qualification authority."
)
FERRIC_BASE = "c5a86fd56c1c817664593df25c04bbed30e84971"
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = ("Compiler", "Hardware", "Runtime")
CASE_MATRIX = (
    ("canonical-subject", "PASS"),
    ("boundary-conforming-subject", "PASS"),
    ("obligation-substitution", "EXPECTED_FAIL"),
    ("property-substitution", "EXPECTED_FAIL"),
    ("path-substitution", "EXPECTED_FAIL"),
    ("profile-substitution", "EXPECTED_FAIL"),
    ("source-closure-substitution", "EXPECTED_FAIL"),
    ("target-substitution", "EXPECTED_FAIL"),
    ("tcb-substitution", "EXPECTED_FAIL"),
    ("malformed-status", "EXPECTED_FAIL"),
)
CASE_COUNTS = {"expected_fail": 8, "pass": 2, "total": 10}
Case = tuple[str, str, str, str, str]
CASES: tuple[Case, ...] = (
    ("runner-ferric", "Roadmap", "m1.r05", "generated-runner", "authentication"),
    ("kernel-fe2o3", "Roadmap", "m1.r06", "fe2o3-gemm", "kernel"),
    (
        "target-ferric",
        "Assurance",
        "target_conforming",
        "identity-closure",
        "authentication",
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
    payload = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    return digest_bytes(payload)


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


def requirement_spec(
    requirements: dict[str, Any], obligation_class: str, obligation_id: str
) -> tuple[dict[str, Any], str, list[str]]:
    if obligation_class == "Roadmap":
        record = next(
            item
            for item in requirements["roadmap_requirements"]
            if item["id"] == obligation_id
        )
        return record, record["title"], record["assurance_properties"]
    record = next(
        item
        for item in requirements["assurance_properties"]
        if item["name"] == obligation_id
    )
    return record, record["boundary"], [obligation_id]


def property_bindings(
    requirements: dict[str, Any], identifiers: list[str]
) -> list[dict[str, Any]]:
    by_name = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    return [
        {
            "boundary_sha256": digest_bytes(by_name[identifier]["boundary"].encode()),
            "fe2o3_kind": by_name[identifier]["fe2o3_kind"],
            "name": identifier,
            "obligation_state": "Open",
            "required_status_at_closure": by_name[identifier][
                "required_status_at_closure"
            ],
        }
        for identifier in identifiers
    ]


def refresh_fixture(fixture: Fixture) -> None:
    report_path, roster_path, transcript_path, context, report, roster, transcript = (
        fixture
    )
    roster_raw = canonical_bytes(roster)
    roster_path.write_bytes(roster_raw)
    transcript["roster_sha256"] = digest_bytes(roster_raw)
    transcript_raw = canonical_bytes(transcript)
    transcript_path.write_bytes(transcript_raw)
    report["roster_sha256"] = digest_bytes(roster_raw)
    report["roster_size_bytes"] = len(roster_raw)
    report["transcript_sha256"] = digest_bytes(transcript_raw)
    report["transcript_size_bytes"] = len(transcript_raw)
    report_raw = canonical_bytes(report)
    report_path.write_bytes(report_raw)
    context["artifact"]["sha256"] = digest_bytes(report_raw)
    context["artifact"]["size_bytes"] = len(report_raw)


def refresh_report(fixture: Fixture) -> None:
    report_path, _, _, context, report, _, _ = fixture
    report_raw = canonical_bytes(report)
    report_path.write_bytes(report_raw)
    context["artifact"]["sha256"] = digest_bytes(report_raw)
    context["artifact"]["size_bytes"] = len(report_raw)


def refresh_transcript(fixture: Fixture) -> None:
    _, _, transcript_path, _, report, _, transcript = fixture
    transcript_raw = canonical_bytes(transcript)
    transcript_path.write_bytes(transcript_raw)
    report["transcript_sha256"] = digest_bytes(transcript_raw)
    report["transcript_size_bytes"] = len(transcript_raw)
    refresh_report(fixture)


def make_fixture(repo: Path, root: Path, case: Case = CASES[0]) -> Fixture:
    name, obligation_class, obligation_id, path_id, profile_id = case
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="ascii"))
    _, statement, property_ids = requirement_spec(
        requirements, obligation_class, obligation_id
    )
    path_record = next(
        item for item in requirements["path_obligations"] if item["id"] == path_id
    )
    artifact_id = f"artifact.independent-validator.{name}"
    binding_id = f"binding.independent-validator.{name}"
    report_relative = f"artifacts/{artifact_id}.independent-validator.json"
    roster_relative = f"validator-runs/{artifact_id}.independent-validator.roster.json"
    transcript_relative = (
        f"validator-runs/{artifact_id}.independent-validator.transcript.json"
    )
    report_path = root / report_relative
    roster_path = root / roster_relative
    transcript_path = root / transcript_relative
    report_path.parent.mkdir(parents=True, exist_ok=True)
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
    tcb = [
        {
            "artifact_id": f"artifact.{identifier}",
            "id": identifier,
            "identity_sha256": digest_bytes(identifier.encode()),
            "kind": kind,
        }
        for identifier, kind in zip(TCB_IDS, TCB_KINDS, strict=True)
    ]
    source_identity_id = f"source.{path_record['repository']}"
    binding = {
        "artifact_id": artifact_id,
        "binding_sha256": "",
        "evidence_kind": "independent-validator",
        "id": binding_id,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "path_id": path_id,
        "profile_id": profile_id,
        "source_identity_id": source_identity_id,
        "statement_sha256": digest_bytes(statement.encode()),
        "tcb_ids": list(TCB_IDS),
    }
    context = {
        "artifact": {
            "id": artifact_id,
            "kind": "ValidatorTranscript",
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
            "source_identity_id": source_identity_id,
        },
        "requirements_sha256": digest_file(requirements_path),
        "sources": sources,
        "subject": f"binding:{binding_id}",
        "tcb": tcb,
    }
    refresh_binding(context)
    properties = property_bindings(requirements, property_ids)
    checker = {
        "commit": digest_bytes(b"independent checker commit")[:40],
        "executable_path": "bin/ferric-independent-validator",
        "executable_sha256": digest_bytes(b"independent checker executable"),
        "id": "outside-lab.ferric-m1-checker",
        "input_schema_sha256": digest_bytes(b"independent input schema"),
        "organization": "outside-lab",
        "output_schema_sha256": digest_bytes(b"independent output schema"),
        "protocol": VALIDATOR_PROTOCOL,
        "repository": "ferric-independent-checker",
        "source_closure_sha256": digest_bytes(b"independent source closure"),
        "tree": digest_bytes(b"independent checker tree")[:40],
        "version": "1.0.0",
    }
    cases = [
        {
            "expected_status": expected,
            "id": identifier,
            "input_sha256": digest_bytes(f"{name}:{identifier}:input".encode()),
            "output_sha256": digest_bytes(f"{name}:{identifier}:output".encode()),
        }
        for identifier, expected in CASE_MATRIX
    ]
    roster = {
        "assurance_property_bindings_sha256": canonical_digest(properties),
        "binding_sha256": binding["binding_sha256"],
        "cases": cases,
        "checker": checker,
        "format": ROSTER_FORMAT,
        "path_resolution_sha256": canonical_digest(context["path_resolution"]),
        "profile_id": profile_id,
        "requirements_sha256": context["requirements_sha256"],
        "source_roster_sha256": canonical_digest(sources),
        "target": TARGET,
        "tcb_roster_sha256": canonical_digest(tcb),
    }
    transcript = {
        "binding_sha256": binding["binding_sha256"],
        "case_counts": copy.deepcopy(CASE_COUNTS),
        "checker_identity_sha256": canonical_digest(checker),
        "completed_at_utc": "2026-08-21T12:01:00Z",
        "format": TRANSCRIPT_FORMAT,
        "results": [
            {
                "exit_code": 0 if record["expected_status"] == "PASS" else 1,
                "expected_status": record["expected_status"],
                "id": record["id"],
                "input_sha256": record["input_sha256"],
                "observed_status": record["expected_status"],
                "output_sha256": record["output_sha256"],
            }
            for record in cases
        ],
        "roster_sha256": "",
        "started_at_utc": "2026-08-21T12:00:00Z",
        "validation_status": "PASS",
    }
    report = {
        "assurance_property_bindings": properties,
        "authority": AUTHORITY,
        "binding_sha256": binding["binding_sha256"],
        "case_counts": copy.deepcopy(CASE_COUNTS),
        "checker_id": checker["id"],
        "checker_identity_sha256": canonical_digest(checker),
        "checker_organization": checker["organization"],
        "evidence_kind": "independent-validator",
        "format": REPORT_FORMAT,
        "independence_attestation": INDEPENDENCE_ATTESTATION,
        "nonclaim": NONCLAIM,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "obligation_state": "Open",
        "path_id": path_id,
        "path_resolution_sha256": canonical_digest(context["path_resolution"]),
        "profile_id": profile_id,
        "requirements_sha256": context["requirements_sha256"],
        "roster_path": roster_relative,
        "roster_sha256": "",
        "roster_size_bytes": 0,
        "source_closure_sha256s": {
            record["id"]: record["source_closure_sha256"] for record in sources
        },
        "source_identity_id": source_identity_id,
        "source_identity_sha256s": {
            record["id"]: canonical_digest(record) for record in sources
        },
        "source_roster_sha256": canonical_digest(sources),
        "statement_sha256": binding["statement_sha256"],
        "target": TARGET,
        "tcb_identity_sha256s": {
            record["id"]: record["identity_sha256"] for record in tcb
        },
        "tcb_roster_sha256": canonical_digest(tcb),
        "transcript_path": transcript_relative,
        "transcript_sha256": "",
        "transcript_size_bytes": 0,
        "validation_status": "PASS",
    }
    fixture = (
        report_path,
        roster_path,
        transcript_path,
        context,
        report,
        roster,
        transcript,
    )
    refresh_fixture(fixture)
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


def assert_rejected(validator: Path, fixture: Fixture, name: str) -> None:
    if invoke(validator, fixture[3]).returncode == 0:
        fail(f"hostile independent-validator fixture was accepted: {name}")


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
                f"canonical {case[0]} report rejected: "
                f"exit={result.returncode}, output={result.stdout!r}"
            )


def hostile_cases(repo: Path, validator: Path, root: Path) -> int:
    report_mutations: list[tuple[str, str, Any]] = [
        ("format", "format", REPORT_FORMAT + "-DRIFT"),
        ("evidence-kind", "evidence_kind", "hardware-test"),
        ("authority-promotion", "authority", "qualification-authority"),
        ("nonclaim-weakening", "nonclaim", "Observed pass."),
        ("attestation-weakening", "independence_attestation", "Independent."),
        ("obligation-class", "obligation_class", "Assurance"),
        ("obligation-replay", "obligation_id", "m1.r06"),
        ("status-promotion", "obligation_state", "Closed"),
        ("statement-replay", "statement_sha256", digest_bytes(b"statement")),
        ("property-omission", "assurance_property_bindings", []),
        ("path-replay", "path_id", "bundle-parser"),
        ("path-identity", "path_resolution_sha256", digest_bytes(b"path")),
        ("profile-replay", "profile_id", "runtime"),
        ("requirements-replay", "requirements_sha256", digest_bytes(b"req")),
        ("source-replay", "source_identity_id", "source.fe2o3"),
        ("source-roster", "source_roster_sha256", digest_bytes(b"sources")),
        ("source-map-omission", "source_closure_sha256s", {}),
        ("source-identity-map", "source_identity_sha256s", {}),
        ("target", "target", "gfx950:xnack-"),
        ("tcb-roster", "tcb_roster_sha256", digest_bytes(b"tcb")),
        ("tcb-map-omission", "tcb_identity_sha256s", {}),
        ("checker-id", "checker_id", "substituted.checker"),
        ("checker-org", "checker_organization", "substituted-org"),
        ("checker-identity", "checker_identity_sha256", digest_bytes(b"checker")),
        ("case-count", "case_counts", {"expected_fail": 7, "pass": 3, "total": 10}),
        ("malformed-status", "validation_status", "SUCCESS"),
        ("roster-path", "roster_path", "../roster.json"),
        ("transcript-path", "transcript_path", "/tmp/transcript.json"),
        ("roster-sha", "roster_sha256", digest_bytes(b"roster")),
        ("transcript-sha", "transcript_sha256", digest_bytes(b"transcript")),
    ]
    for name, field, value in report_mutations:
        fixture = make_fixture(repo, root / f"report-{name}")
        fixture[4][field] = copy.deepcopy(value)
        refresh_report(fixture)
        assert_rejected(validator, fixture, f"report-{name}")

    checker_mutations: list[tuple[str, str, Any]] = [
        ("self-org-harsh", "organization", "harsh-nod"),
        ("self-org-ferric", "organization", "ferric"),
        ("self-repository", "repository", "fe2o3"),
        ("self-id", "id", "source.ferric"),
        ("self-commit", "commit", digest_bytes(b"ferric commit")[:40]),
        ("self-tree", "tree", digest_bytes(b"fe2o3 tree")[:40]),
        (
            "self-closure",
            "source_closure_sha256",
            digest_bytes(b"ferric source closure"),
        ),
        (
            "trusted-checker-path",
            "executable_path",
            "proofs/m1/evidence/validate-independent-validator.py",
        ),
        ("protocol", "protocol", VALIDATOR_PROTOCOL + ".drift"),
        ("version", "version", "latest"),
        ("executable", "executable_sha256", digest_bytes(b"ferric source closure")),
        ("input-schema", "input_schema_sha256", digest_bytes(b"other input")),
        ("output-schema", "output_schema_sha256", digest_bytes(b"other output")),
    ]
    for name, field, value in checker_mutations:
        fixture = make_fixture(repo, root / f"checker-{name}")
        fixture[5]["checker"][field] = value
        refresh_fixture(fixture)
        assert_rejected(validator, fixture, f"checker-{name}")

    roster_mutations: list[tuple[str, Mutation]] = [
        ("format", lambda f: f[5].__setitem__("format", ROSTER_FORMAT + "-DRIFT")),
        (
            "binding",
            lambda f: f[5].__setitem__("binding_sha256", digest_bytes(b"bind")),
        ),
        (
            "properties",
            lambda f: f[5].__setitem__(
                "assurance_property_bindings_sha256", digest_bytes(b"properties")
            ),
        ),
        (
            "path",
            lambda f: f[5].__setitem__("path_resolution_sha256", digest_bytes(b"path")),
        ),
        ("profile", lambda f: f[5].__setitem__("profile_id", "runtime")),
        (
            "requirements",
            lambda f: f[5].__setitem__(
                "requirements_sha256", digest_bytes(b"requirements")
            ),
        ),
        (
            "sources",
            lambda f: f[5].__setitem__(
                "source_roster_sha256", digest_bytes(b"sources")
            ),
        ),
        ("target", lambda f: f[5].__setitem__("target", "gfx950:xnack-")),
        ("tcb", lambda f: f[5].__setitem__("tcb_roster_sha256", digest_bytes(b"tcb"))),
        ("skip-case", lambda f: f[5]["cases"].pop()),
        (
            "duplicate-case",
            lambda f: f[5]["cases"].__setitem__(1, copy.deepcopy(f[5]["cases"][0])),
        ),
        ("case-order", lambda f: f[5]["cases"].reverse()),
        (
            "case-status",
            lambda f: f[5]["cases"][0].__setitem__("expected_status", "EXPECTED_FAIL"),
        ),
        (
            "case-input-reuse",
            lambda f: f[5]["cases"][1].__setitem__(
                "input_sha256", f[5]["cases"][0]["input_sha256"]
            ),
        ),
        (
            "case-output-reuse",
            lambda f: f[5]["cases"][1].__setitem__(
                "output_sha256", f[5]["cases"][0]["output_sha256"]
            ),
        ),
    ]
    for name, mutation in roster_mutations:
        fixture = make_fixture(repo, root / f"roster-{name}")
        mutation(fixture)
        refresh_fixture(fixture)
        assert_rejected(validator, fixture, f"roster-{name}")

    transcript_mutations: list[tuple[str, Mutation]] = [
        ("format", lambda f: f[6].__setitem__("format", TRANSCRIPT_FORMAT + "-DRIFT")),
        (
            "binding",
            lambda f: f[6].__setitem__("binding_sha256", digest_bytes(b"binding")),
        ),
        (
            "checker",
            lambda f: f[6].__setitem__(
                "checker_identity_sha256", digest_bytes(b"checker")
            ),
        ),
        (
            "counts",
            lambda f: f[6].__setitem__(
                "case_counts", {"expected_fail": 8, "pass": 1, "total": 9}
            ),
        ),
        ("status", lambda f: f[6].__setitem__("validation_status", "FAIL")),
        (
            "time-order",
            lambda f: f[6].__setitem__("completed_at_utc", "2026-08-21T11:59:59Z"),
        ),
        ("time-format", lambda f: f[6].__setitem__("started_at_utc", "today")),
        ("skip-result", lambda f: f[6]["results"].pop()),
        ("result-order", lambda f: f[6]["results"].reverse()),
        (
            "result-status",
            lambda f: f[6]["results"][0].__setitem__("observed_status", "SUCCESS"),
        ),
        (
            "result-expected",
            lambda f: f[6]["results"][2].__setitem__("expected_status", "PASS"),
        ),
        ("result-exit", lambda f: f[6]["results"][2].__setitem__("exit_code", 0)),
        (
            "result-input",
            lambda f: f[6]["results"][0].__setitem__(
                "input_sha256", digest_bytes(b"other input")
            ),
        ),
        (
            "result-output",
            lambda f: f[6]["results"][0].__setitem__(
                "output_sha256", digest_bytes(b"other output")
            ),
        ),
    ]
    for name, mutation in transcript_mutations:
        fixture = make_fixture(repo, root / f"transcript-{name}")
        mutation(fixture)
        refresh_transcript(fixture)
        assert_rejected(validator, fixture, f"transcript-{name}")

    context_mutations: list[tuple[str, Mutation]] = [
        ("kind", lambda f: f[3]["artifact"].__setitem__("kind", "HardwareTranscript")),
        ("artifact-id", lambda f: f[3]["artifact"].__setitem__("id", "artifact.other")),
        (
            "artifact-path",
            lambda f: f[3]["artifact"].__setitem__("path", "artifacts/other.json"),
        ),
        (
            "artifact-sha",
            lambda f: f[3]["artifact"].__setitem__("sha256", digest_bytes(b"other")),
        ),
        ("artifact-size", lambda f: f[3]["artifact"].__setitem__("size_bytes", 1)),
        ("subject", lambda f: f[3].__setitem__("subject", "binding:other")),
        (
            "binding-kind",
            lambda f: f[3]["binding"].__setitem__("evidence_kind", "hardware-test"),
        ),
        (
            "binding-artifact",
            lambda f: f[3]["binding"].__setitem__("artifact_id", "artifact.other"),
        ),
        (
            "binding-digest",
            lambda f: f[3]["binding"].__setitem__(
                "binding_sha256", digest_bytes(b"binding")
            ),
        ),
        (
            "binding-profile",
            lambda f: f[3]["binding"].__setitem__("profile_id", "runtime"),
        ),
        (
            "binding-path",
            lambda f: f[3]["binding"].__setitem__("path_id", "bundle-parser"),
        ),
        (
            "binding-statement",
            lambda f: f[3]["binding"].__setitem__(
                "statement_sha256", digest_bytes(b"statement")
            ),
        ),
        ("binding-tcb-order", lambda f: f[3]["binding"]["tcb_ids"].reverse()),
        (
            "path-availability",
            lambda f: f[3]["path_resolution"].__setitem__(
                "availability", "ExistingFoundation"
            ),
        ),
        (
            "path-file",
            lambda f: f[3]["path_resolution"].__setitem__("path", "docs/ASSURANCE.md"),
        ),
        (
            "path-repository",
            lambda f: f[3]["path_resolution"].__setitem__("repository", "fe2o3"),
        ),
        (
            "path-source",
            lambda f: f[3]["path_resolution"].__setitem__(
                "source_identity_id", "source.fe2o3"
            ),
        ),
        ("source-order", lambda f: f[3]["sources"].reverse()),
        (
            "source-duplicate",
            lambda f: f[3]["sources"].__setitem__(1, copy.deepcopy(f[3]["sources"][0])),
        ),
        (
            "source-closure",
            lambda f: f[3]["sources"][0].__setitem__(
                "source_closure_sha256", digest_bytes(b"other closure")
            ),
        ),
        (
            "source-base",
            lambda f: f[3]["sources"][1].__setitem__(
                "base_commit", digest_bytes(b"other base")[:40]
            ),
        ),
        ("tcb-order", lambda f: f[3]["tcb"].reverse()),
        (
            "tcb-duplicate",
            lambda f: f[3]["tcb"].__setitem__(1, copy.deepcopy(f[3]["tcb"][0])),
        ),
        ("tcb-kind", lambda f: f[3]["tcb"][0].__setitem__("kind", "Runtime")),
        (
            "requirements",
            lambda f: f[3].__setitem__(
                "requirements_sha256", digest_bytes(b"requirements")
            ),
        ),
        (
            "context-format",
            lambda f: f[3].__setitem__("format", "ferric.m1-evidence-index.v2"),
        ),
    ]
    for name, mutation in context_mutations:
        fixture = make_fixture(repo, root / f"context-{name}")
        mutation(fixture)
        assert_rejected(validator, fixture, f"context-{name}")

    special_count = 0
    for component, index, label in (
        ("report", 0, "report-symlink"),
        ("roster", 1, "roster-symlink"),
        ("transcript", 2, "transcript-symlink"),
    ):
        fixture = make_fixture(repo, root / label)
        path = fixture[index]
        target = path.parent / f"{component}-target.json"
        path.rename(target)
        path.symlink_to(target)
        assert_rejected(validator, fixture, label)
        special_count += 1

    fixture = make_fixture(repo, root / "companion-parent-symlink")
    directory = fixture[1].parent
    target_directory = directory.parent / "validator-runs-target"
    directory.rename(target_directory)
    directory.symlink_to(target_directory, target_is_directory=True)
    assert_rejected(validator, fixture, "companion-parent-symlink")
    special_count += 1

    for index, label in (
        (0, "noncanonical-report"),
        (1, "noncanonical-roster"),
        (2, "noncanonical-transcript"),
    ):
        fixture = make_fixture(repo, root / label)
        path = fixture[index]
        value = (fixture[4], fixture[5], fixture[6])[index]
        raw = (json.dumps(value, ensure_ascii=True, sort_keys=True) + "\n").encode(
            "ascii"
        )
        path.write_bytes(raw)
        if index == 0:
            fixture[3]["artifact"]["sha256"] = digest_bytes(raw)
            fixture[3]["artifact"]["size_bytes"] = len(raw)
        elif index == 1:
            fixture[4]["roster_sha256"] = digest_bytes(raw)
            fixture[4]["roster_size_bytes"] = len(raw)
            refresh_report(fixture)
        else:
            fixture[4]["transcript_sha256"] = digest_bytes(raw)
            fixture[4]["transcript_size_bytes"] = len(raw)
            refresh_report(fixture)
        assert_rejected(validator, fixture, label)
        special_count += 1

    fixture = make_fixture(repo, root / "extra-report-field")
    fixture[4]["qualification_authority"] = True
    refresh_report(fixture)
    assert_rejected(validator, fixture, "extra-report-field")
    special_count += 1

    fixture = make_fixture(repo, root / "duplicate-roster-key")
    raw = canonical_bytes(fixture[5]).replace(
        b'{\n  "assurance_property_bindings_sha256":',
        b'{\n  "format": "duplicate",\n  "assurance_property_bindings_sha256":',
        1,
    )
    fixture[1].write_bytes(raw)
    fixture[4]["roster_sha256"] = digest_bytes(raw)
    fixture[4]["roster_size_bytes"] = len(raw)
    fixture[6]["roster_sha256"] = digest_bytes(raw)
    refresh_transcript(fixture)
    assert_rejected(validator, fixture, "duplicate-roster-key")
    special_count += 1

    replay_a = make_fixture(repo, root / "replay-a", CASES[0])
    replay_b = make_fixture(repo, root / "replay-b", CASES[1])
    for source, destination in zip(replay_a[:3], replay_b[:3], strict=True):
        destination.write_bytes(source.read_bytes())
    replay_b[3]["artifact"]["sha256"] = digest_file(replay_b[0])
    replay_b[3]["artifact"]["size_bytes"] = replay_b[0].stat().st_size
    assert_rejected(validator, replay_b, "cross-binding-replay")
    special_count += 1

    fixture = make_fixture(repo, root / "raw-context")
    context = fixture[3]
    noncanonical = (
        json.dumps(context, ensure_ascii=True, sort_keys=True) + "\n"
    ).encode("ascii")
    duplicate = (
        (
            json.dumps(
                context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
            )
            + "\n"
        )
        .encode("ascii")
        .replace(b'{"artifact":', b'{"format":"duplicate","artifact":', 1)
    )
    raw_cases = (
        ("noncanonical-context", noncanonical, PROTOCOL),
        ("duplicate-context", duplicate, PROTOCOL),
        ("empty-context", b"", PROTOCOL),
        ("extra-trailing-context", noncanonical + b"\n", PROTOCOL),
        ("protocol-drift", None, PROTOCOL + ".drift"),
    )
    for label, raw, protocol in raw_cases:
        if (
            invoke(validator, context, raw_context=raw, protocol=protocol).returncode
            == 0
        ):
            fail(f"hostile independent-validator invocation was accepted: {label}")
        special_count += 1
    extra = copy.deepcopy(context)
    extra["validator_path"] = "self-selected.py"
    if invoke(validator, extra).returncode == 0:
        fail("index-selected validator field was accepted")
    special_count += 1

    return (
        len(report_mutations)
        + len(checker_mutations)
        + len(roster_mutations)
        + len(transcript_mutations)
        + len(context_mutations)
        + special_count
    )


def audit_checker_pin(repo: Path, validator: Path) -> None:
    checker_path = repo / "proofs/check-m1-evidence-index.py"
    checker = load_module(checker_path, "ferric_m1_evidence_checker")
    expected = (
        "proofs/m1/evidence/validate-independent-validator.py",
        PROTOCOL,
        digest_file(validator),
    )
    if checker.TRUSTED_VALIDATORS.get("independent-validator") != expected:
        fail(
            "checker-owned independent-validator path, protocol, or source pin drifted"
        )


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
        fail("M1 roadmap or assurance state changed by independent validation")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: test-independent-validator-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-independent-validator.py"
    audit_checker_pin(repo, validator)
    audit_open_requirements(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-independent-validator.") as raw:
        root = Path(raw)
        canonical_cases(repo, validator, root / "canonical")
        hostile_count = hostile_cases(repo, validator, root / "hostile")
    print(
        "PASS: M1 independent-validator accepted 3 canonical reports and rejected "
        f"{hostile_count} hostile fixtures"
    )


if __name__ == "__main__":
    main()
