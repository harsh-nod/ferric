#!/usr/bin/env python3
"""Exercise canonical and hostile M1 canonical-structure transcripts."""

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


PROTOCOL = "ferric.m1-validator.canonical-structure.v1"
REPORT_FORMAT = "FERRIC-M1-CANONICAL-STRUCTURE-V1"
PAYLOAD_FORMAT = "FERRIC-M1-CANONICAL-RECORDS-V1"
PAYLOAD_SCHEMA_ID = "ferric.m1-canonical-records.v1"
ARTIFACT_TARGET = "gfx942:xnack-"
AUTHORITY = "canonical-structure-only"
NONCLAIM = (
    "This transcript establishes only that the referenced bytes conform to "
    "the checker-owned canonical record schema and exact evidence binding. "
    "It grants no semantic correctness, theorem, machine, load, launch, "
    "hardware, performance, or qualification authority."
)
FERRIC_BASE = "c5a86fd56c1c817664593df25c04bbed30e84971"
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = ("Compiler", "Hardware", "Runtime")
PAYLOAD_SCHEMA = {
    "format": PAYLOAD_FORMAT,
    "record_fields": ["name", "type", "value"],
    "record_types": ["boolean", "count", "identifier", "sha256", "text"],
    "required_fields": [
        "binding_sha256",
        "format",
        "obligation_class",
        "obligation_id",
        "path_id",
        "profile_id",
        "records",
        "source_identity_id",
        "target",
    ],
    "schema_id": PAYLOAD_SCHEMA_ID,
    "target": ARTIFACT_TARGET,
}
Case = tuple[str, str, str, str, str]
CASES: tuple[Case, ...] = (
    ("roadmap-admission", "Roadmap", "m1.r01", "admission", "bundle-auth"),
    (
        "roadmap-authentication",
        "Roadmap",
        "m1.r05",
        "authentication",
        "generated-runner",
    ),
    (
        "assurance-admission-fe2o3",
        "Assurance",
        "resource_bounded",
        "admission",
        "fe2o3-aql-foundation",
    ),
    (
        "assurance-authentication",
        "Assurance",
        "artifact_authenticated",
        "authentication",
        "identity-closure",
    ),
    (
        "target-authentication",
        "Assurance",
        "target_conforming",
        "authentication",
        "d10-bench",
    ),
)
Fixture = tuple[Path, Path, dict[str, Any], dict[str, Any], dict[str, Any]]
Mutation = Callable[[Path, Path, dict[str, Any], dict[str, Any], dict[str, Any]], None]


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


def refresh_payload(
    payload_path: Path, report: dict[str, Any], payload: dict[str, Any]
) -> None:
    raw = canonical_bytes(payload)
    payload_path.write_bytes(raw)
    report["canonical_payload_sha256"] = digest_bytes(raw)
    report["canonical_payload_size_bytes"] = len(raw)
    records = payload.get("records")
    report["record_count"] = len(records) if isinstance(records, list) else 0


def refresh_report(
    report_path: Path, context: dict[str, Any], report: dict[str, Any]
) -> None:
    raw = canonical_bytes(report)
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)


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


def make_fixture(repo: Path, root: Path, case: Case = CASES[0]) -> Fixture:
    name, obligation_class, obligation_id, profile, path_id = case
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="ascii"))
    _, statement, assurance_properties = requirement_spec(
        requirements, obligation_class, obligation_id
    )
    path_record = next(
        item for item in requirements["path_obligations"] if item["id"] == path_id
    )
    artifact_id = f"artifact.canonical.{name}"
    binding_id = f"binding.canonical.{name}"
    report_relative = f"artifacts/{artifact_id}.canonical-structure.json"
    payload_relative = f"canonical-payloads/{artifact_id}.json"
    report_path = root / report_relative
    payload_path = root / payload_relative
    report_path.parent.mkdir(parents=True, exist_ok=True)
    payload_path.parent.mkdir(parents=True, exist_ok=True)

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
            "identity_sha256": digest_bytes(identifier.encode("ascii")),
            "kind": kind,
        }
        for identifier, kind in zip(TCB_IDS, TCB_KINDS, strict=True)
    ]
    binding = {
        "artifact_id": artifact_id,
        "binding_sha256": "",
        "evidence_kind": "canonical-structure-check",
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
            "kind": "CheckerTranscript",
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
    payload = {
        "binding_sha256": binding["binding_sha256"],
        "format": PAYLOAD_FORMAT,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "path_id": path_id,
        "profile_id": profile,
        "records": [
            {"name": "enabled", "type": "boolean", "value": True},
            {"name": "item_count", "type": "count", "value": 3},
            {"name": "object_id", "type": "identifier", "value": name},
            {
                "name": "payload_sha256",
                "type": "sha256",
                "value": digest_bytes(name.encode("ascii")),
            },
            {"name": "summary", "type": "text", "value": "canonical fixture"},
        ],
        "source_identity_id": binding["source_identity_id"],
        "target": ARTIFACT_TARGET,
    }
    report = {
        "assurance_property_ids": copy.deepcopy(assurance_properties),
        "authority": AUTHORITY,
        "binding_sha256": binding["binding_sha256"],
        "canonical_payload_format": PAYLOAD_FORMAT,
        "canonical_payload_relative_path": payload_relative,
        "canonical_payload_sha256": "",
        "canonical_payload_size_bytes": 0,
        "canonical_schema_id": PAYLOAD_SCHEMA_ID,
        "canonical_schema_sha256": canonical_digest(PAYLOAD_SCHEMA),
        "evidence_kind": "canonical-structure-check",
        "format": REPORT_FORMAT,
        "nonclaim": NONCLAIM,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "obligation_state": "Open",
        "path_id": path_id,
        "path_resolution_sha256": canonical_digest(context["path_resolution"]),
        "profile_id": profile,
        "record_count": 0,
        "requirements_sha256": context["requirements_sha256"],
        "result": "canonical",
        "source_identity_id": binding["source_identity_id"],
        "source_roster_sha256": canonical_digest(sources),
        "statement_sha256": binding["statement_sha256"],
        "tcb_identity_sha256s": {item["id"]: item["identity_sha256"] for item in tcb},
        "tcb_roster_sha256": canonical_digest(tcb),
    }
    refresh_payload(payload_path, report, payload)
    refresh_report(report_path, context, report)
    return report_path, payload_path, context, report, payload


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
        _, _, context, _, _ = make_fixture(repo, root / case[0], case)
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
    def report_field(key: str, value: Any) -> Mutation:
        def mutate(
            report_path: Path,
            _payload_path: Path,
            context: dict[str, Any],
            report: dict[str, Any],
            _payload: dict[str, Any],
        ) -> None:
            report[key] = copy.deepcopy(value)
            refresh_report(report_path, context, report)

        return mutate

    report_mutations: list[tuple[str, Mutation]] = [
        ("report-format", report_field("format", REPORT_FORMAT + "-DRIFT")),
        ("authority-promotion", report_field("authority", "semantic-authority")),
        ("nonclaim-weakening", report_field("nonclaim", "Canonical.")),
        ("report-evidence-kind", report_field("evidence_kind", "verus-theorem")),
        ("result-promotion", report_field("result", "proved")),
        ("payload-format", report_field("canonical_payload_format", "other")),
        ("schema-id", report_field("canonical_schema_id", "other.schema.v1")),
        (
            "schema-sha",
            report_field("canonical_schema_sha256", digest_bytes(b"other schema")),
        ),
        (
            "payload-path-traversal",
            report_field("canonical_payload_relative_path", "../payload.json"),
        ),
        (
            "payload-path-substitution",
            report_field(
                "canonical_payload_relative_path", "canonical-payloads/x.json"
            ),
        ),
        (
            "payload-sha",
            report_field("canonical_payload_sha256", digest_bytes(b"other payload")),
        ),
        ("payload-size", report_field("canonical_payload_size_bytes", 1)),
        ("payload-size-bool", report_field("canonical_payload_size_bytes", True)),
        ("record-count", report_field("record_count", 4)),
        ("record-count-bool", report_field("record_count", True)),
        ("binding-replay", report_field("binding_sha256", digest_bytes(b"binding"))),
        ("obligation-class", report_field("obligation_class", "Assurance")),
        ("obligation-replay", report_field("obligation_id", "m1.r02")),
        ("status-promotion", report_field("obligation_state", "Closed")),
        ("property-omission", report_field("assurance_property_ids", [])),
        (
            "property-order",
            report_field(
                "assurance_property_ids",
                [
                    "resource_bounded",
                    "artifact_authenticated",
                    "model_bundle_well_formed",
                ],
            ),
        ),
        ("profile-replay", report_field("profile_id", "authentication")),
        ("path-replay", report_field("path_id", "bundle-parser")),
        (
            "path-resolution",
            report_field("path_resolution_sha256", digest_bytes(b"path")),
        ),
        (
            "requirements-replay",
            report_field("requirements_sha256", digest_bytes(b"requirements")),
        ),
        ("source-substitution", report_field("source_identity_id", "source.fe2o3")),
        (
            "source-roster",
            report_field("source_roster_sha256", digest_bytes(b"sources")),
        ),
        ("statement-replay", report_field("statement_sha256", digest_bytes(b"text"))),
        ("tcb-roster", report_field("tcb_roster_sha256", digest_bytes(b"tcb"))),
        (
            "tcb-identity-omission",
            report_field("tcb_identity_sha256s", {"tcb.compiler": digest_bytes(b"x")}),
        ),
    ]

    def run_hostile(name: str, mutation: Mutation, case: Case = CASES[0]) -> None:
        fixture = make_fixture(repo, root / name, case)
        mutation(*fixture)
        if invoke(validator, fixture[2]).returncode == 0:
            fail(f"hostile canonical-structure input was accepted: {name}")

    for name, mutation in report_mutations:
        run_hostile(name, mutation)

    def context_mutation(function: Callable[[dict[str, Any]], None]) -> Mutation:
        def mutate(
            _report_path: Path,
            _payload_path: Path,
            context: dict[str, Any],
            _report: dict[str, Any],
            _payload: dict[str, Any],
        ) -> None:
            function(context)

        return mutate

    context_mutations: list[tuple[str, Mutation]] = [
        (
            "outer-kind",
            context_mutation(lambda c: c["artifact"].__setitem__("kind", "TcbReport")),
        ),
        (
            "outer-id",
            context_mutation(
                lambda c: c["artifact"].__setitem__("id", "artifact.other")
            ),
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
                lambda c: c["binding"].__setitem__("evidence_kind", "artifact-identity")
            ),
        ),
        (
            "binding-artifact",
            context_mutation(
                lambda c: c["binding"].__setitem__("artifact_id", "artifact.other")
            ),
        ),
        (
            "binding-digest",
            context_mutation(
                lambda c: c["binding"].__setitem__(
                    "binding_sha256", digest_bytes(b"other binding")
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
            context_mutation(lambda c: c["binding"].__setitem__("path_id", "m1-tcb")),
        ),
        (
            "binding-statement",
            context_mutation(
                lambda c: c["binding"].__setitem__(
                    "statement_sha256", digest_bytes(b"other statement")
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
                lambda c: c["path_resolution"].__setitem__("path", "docs/ASSURANCE.md")
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
                    "base_commit", digest_bytes(b"other base")[:40]
                )
            ),
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
                    "source_closure_sha256", digest_bytes(b"other closure")
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
                    "identity_sha256", digest_bytes(b"other tcb")
                )
            ),
        ),
        (
            "requirements-context",
            context_mutation(
                lambda c: c.__setitem__(
                    "requirements_sha256", digest_bytes(b"other requirements")
                )
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

    def payload_mutation(function: Callable[[dict[str, Any]], None]) -> Mutation:
        def mutate(
            report_path: Path,
            payload_path: Path,
            context: dict[str, Any],
            report: dict[str, Any],
            payload: dict[str, Any],
        ) -> None:
            function(payload)
            refresh_payload(payload_path, report, payload)
            refresh_report(report_path, context, report)

        return mutate

    payload_mutations: list[tuple[str, Mutation]] = [
        (
            "payload-format-drift",
            payload_mutation(lambda p: p.__setitem__("format", PAYLOAD_FORMAT + "-X")),
        ),
        (
            "payload-target",
            payload_mutation(lambda p: p.__setitem__("target", "gfx950:xnack-")),
        ),
        (
            "payload-binding",
            payload_mutation(
                lambda p: p.__setitem__("binding_sha256", digest_bytes(b"other"))
            ),
        ),
        (
            "payload-obligation-class",
            payload_mutation(lambda p: p.__setitem__("obligation_class", "Assurance")),
        ),
        (
            "payload-obligation",
            payload_mutation(lambda p: p.__setitem__("obligation_id", "m1.r02")),
        ),
        (
            "payload-path",
            payload_mutation(lambda p: p.__setitem__("path_id", "bundle-parser")),
        ),
        (
            "payload-profile",
            payload_mutation(lambda p: p.__setitem__("profile_id", "authentication")),
        ),
        (
            "payload-source",
            payload_mutation(
                lambda p: p.__setitem__("source_identity_id", "source.fe2o3")
            ),
        ),
        ("records-empty", payload_mutation(lambda p: p.__setitem__("records", []))),
        (
            "record-order",
            payload_mutation(lambda p: p["records"].reverse()),
        ),
        (
            "record-duplicate",
            payload_mutation(
                lambda p: p["records"].append(copy.deepcopy(p["records"][0]))
            ),
        ),
        (
            "record-name",
            payload_mutation(lambda p: p["records"][0].__setitem__("name", "Bad Name")),
        ),
        (
            "record-type",
            payload_mutation(lambda p: p["records"][0].__setitem__("type", "float")),
        ),
        (
            "boolean-string",
            payload_mutation(lambda p: p["records"][0].__setitem__("value", "true")),
        ),
        (
            "count-negative",
            payload_mutation(lambda p: p["records"][1].__setitem__("value", -1)),
        ),
        (
            "count-bool",
            payload_mutation(lambda p: p["records"][1].__setitem__("value", True)),
        ),
        (
            "count-overflow",
            payload_mutation(lambda p: p["records"][1].__setitem__("value", 1 << 63)),
        ),
        (
            "identifier-space",
            payload_mutation(
                lambda p: p["records"][2].__setitem__("value", "bad identifier")
            ),
        ),
        (
            "sha-uppercase",
            payload_mutation(lambda p: p["records"][3].__setitem__("value", "A" * 64)),
        ),
        (
            "sha-placeholder",
            payload_mutation(lambda p: p["records"][3].__setitem__("value", "a" * 64)),
        ),
        (
            "text-newline",
            payload_mutation(
                lambda p: p["records"][4].__setitem__("value", "line one\nline two")
            ),
        ),
        (
            "record-extra-field",
            payload_mutation(lambda p: p["records"][0].__setitem__("unit", "items")),
        ),
        (
            "payload-extra-field",
            payload_mutation(lambda p: p.__setitem__("semantic_correctness", True)),
        ),
    ]
    for name, mutation in payload_mutations:
        run_hostile(name, mutation)

    report_path, payload_path, context, report, payload = make_fixture(
        repo, root / "payload-tamper"
    )
    payload_path.write_bytes(b"substituted\n")
    if invoke(validator, context).returncode == 0:
        fail("tampered canonical payload was accepted")

    report_path, payload_path, context, report, payload = make_fixture(
        repo, root / "payload-noncanonical"
    )
    raw = (json.dumps(payload, ensure_ascii=True, sort_keys=True) + "\n").encode(
        "ascii"
    )
    payload_path.write_bytes(raw)
    report["canonical_payload_sha256"] = digest_bytes(raw)
    report["canonical_payload_size_bytes"] = len(raw)
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("noncanonical payload JSON was accepted")

    report_path, payload_path, context, report, payload = make_fixture(
        repo, root / "payload-duplicate-key"
    )
    raw = canonical_bytes(payload).replace(
        b'{\n  "binding_sha256":',
        b'{\n  "format": "duplicate",\n  "binding_sha256":',
        1,
    )
    payload_path.write_bytes(raw)
    report["canonical_payload_sha256"] = digest_bytes(raw)
    report["canonical_payload_size_bytes"] = len(raw)
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("duplicate-key payload JSON was accepted")

    report_path, payload_path, context, _, _ = make_fixture(
        repo, root / "payload-symlink"
    )
    target = payload_path.parent / "target.json"
    payload_path.rename(target)
    payload_path.symlink_to(target)
    if invoke(validator, context).returncode == 0:
        fail("symlink canonical payload was accepted")

    report_path, _, context, _, _ = make_fixture(repo, root / "report-symlink")
    target = report_path.parent / "target.json"
    report_path.rename(target)
    report_path.symlink_to(target)
    if invoke(validator, context).returncode == 0:
        fail("symlink canonical report was accepted")

    report_path, _, context, report, _ = make_fixture(
        repo, root / "report-noncanonical"
    )
    raw = (json.dumps(report, ensure_ascii=True, sort_keys=True) + "\n").encode("ascii")
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, context).returncode == 0:
        fail("noncanonical report JSON was accepted")

    report_path, _, context, report, _ = make_fixture(repo, root / "report-extra-field")
    report["semantic_correctness"] = True
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("report with extra authority was accepted")

    _, _, context, _, _ = make_fixture(repo, root / "raw-context")
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
        "non-ascii": (canonical + "\n")
        .encode("ascii")
        .replace(b"canonical", b"canon\xffical", 1),
    }
    for name, raw in raw_cases.items():
        if invoke(validator, context, raw_context=raw).returncode == 0:
            fail(f"hostile raw context was accepted: {name}")
    extra = copy.deepcopy(context)
    extra["validator_path"] = "self-selected.py"
    if invoke(validator, extra).returncode == 0:
        fail("index-selected validator path was accepted")
    if invoke(validator, context, protocol=PROTOCOL + ".drift").returncode == 0:
        fail("wrong canonical-structure protocol was accepted")

    return (
        len(report_mutations)
        + len(context_mutations)
        + len(payload_mutations)
        + 7
        + len(raw_cases)
        + 2
    )


def audit_checker_pin(repo: Path, validator: Path) -> None:
    checker = load_module(
        repo / "proofs/check-m1-evidence-index.py", "ferric_m1_evidence_checker"
    )
    expected = (
        "proofs/m1/evidence/validate-canonical-structure.py",
        PROTOCOL,
        digest_file(validator),
    )
    if checker.TRUSTED_VALIDATORS.get("canonical-structure-check") != expected:
        fail("checker-owned canonical-structure path, protocol, or source pin drifted")


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
        fail("M1 roadmap or assurance state was changed by structure validation")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: test-canonical-structure-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-canonical-structure.py"
    audit_checker_pin(repo, validator)
    audit_open_requirements(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-canonical-structure.") as raw:
        root = Path(raw)
        canonical_cases(repo, validator, root / "canonical")
        hostile_count = hostile_cases(repo, validator, root / "hostile")
    print(
        "PASS: M1 canonical-structure validator accepted 5 canonical transcripts "
        f"and rejected {hostile_count} hostile fixtures"
    )


if __name__ == "__main__":
    main()
