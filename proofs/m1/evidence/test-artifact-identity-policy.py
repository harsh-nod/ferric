#!/usr/bin/env python3
"""Exercise canonical and hostile inputs against the M1 identity validator."""

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


PROTOCOL = "ferric.m1-validator.artifact-identity.v1"
REPORT_FORMAT = "FERRIC-M1-ARTIFACT-IDENTITY-V1"
ARTIFACT_KIND = "M1ImmutablePayload"
ARTIFACT_TARGET = "gfx942:xnack-"
AUTHORITY = "identity-and-structure-only"
NONCLAIM = (
    "This report authenticates byte identity and canonical structure only. "
    "It grants no semantic correctness, theorem, machine, load, launch, "
    "hardware, performance, or qualification authority."
)
FERRIC_BASE = "c5a86fd56c1c817664593df25c04bbed30e84971"
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = ("Compiler", "Hardware", "Runtime")
Case = tuple[str, str, str, str]
CASES: tuple[Case, ...] = (
    ("roadmap", "Roadmap", "m1.r01", "bundle-auth"),
    ("assurance", "Assurance", "artifact_authenticated", "identity-closure"),
    ("ferric", "Assurance", "operator_refined", "ferric-gemm"),
)
Mutation = Callable[[Path, Path, dict[str, Any], dict[str, Any]], None]


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


def refresh_report(
    report_path: Path, context: dict[str, Any], report: dict[str, Any]
) -> None:
    data = canonical_bytes(report)
    report_path.write_bytes(data)
    context["artifact"]["sha256"] = digest_bytes(data)
    context["artifact"]["size_bytes"] = len(data)


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


def make_fixture(
    repo: Path, root: Path, case: Case = CASES[0]
) -> tuple[Path, Path, dict[str, Any], dict[str, Any]]:
    name, obligation_class, obligation_id, path_id = case
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="ascii"))
    spec, statement, assurance_properties = requirement_spec(
        requirements, obligation_class, obligation_id
    )
    path_record = next(
        item for item in requirements["path_obligations"] if item["id"] == path_id
    )
    profiles = {item["id"]: item["kinds"] for item in requirements["evidence_profiles"]}
    profile = next(
        profile_id
        for profile_id in spec["evidence_profiles"]
        if "artifact-identity" in profiles[profile_id]
    )
    artifact_id = f"artifact.identity.{name}"
    binding_id = f"binding.identity.{name}"
    report_relative = f"artifacts/{artifact_id}.artifact-identity.json"
    payload_relative = f"identified-artifacts/{artifact_id}.bin"
    report_path = root / report_relative
    payload_path = root / payload_relative
    report_path.parent.mkdir(parents=True, exist_ok=True)
    payload_path.parent.mkdir(parents=True, exist_ok=True)
    payload = f"canonical immutable payload for {name}\n".encode("ascii") + bytes(
        range(32)
    )
    payload_path.write_bytes(payload)

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
        "evidence_kind": "artifact-identity",
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
            "kind": "ArtifactIdentityReport",
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
    report = {
        "artifact_kind": ARTIFACT_KIND,
        "artifact_profile_id": profile,
        "artifact_relative_path": payload_relative,
        "artifact_sha256": digest_bytes(payload),
        "artifact_size_bytes": len(payload),
        "artifact_target": ARTIFACT_TARGET,
        "assurance_property_ids": copy.deepcopy(assurance_properties),
        "authority": AUTHORITY,
        "binding_sha256": binding["binding_sha256"],
        "evidence_kind": "artifact-identity",
        "format": REPORT_FORMAT,
        "nonclaim": NONCLAIM,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "obligation_state": "Open",
        "path_id": path_id,
        "path_resolution_sha256": canonical_digest(context["path_resolution"]),
        "requirements_sha256": context["requirements_sha256"],
        "source_identity_id": binding["source_identity_id"],
        "source_roster_sha256": canonical_digest(sources),
        "statement_sha256": binding["statement_sha256"],
        "tcb_identity_sha256s": {item["id"]: item["identity_sha256"] for item in tcb},
        "tcb_roster_sha256": canonical_digest(tcb),
    }
    refresh_report(report_path, context, report)
    return report_path, payload_path, context, report


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
        case_root = root / case[0]
        _, _, context, _ = make_fixture(repo, case_root, case)
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
    def report_field(key: str, value: Any) -> Mutation:
        def mutate(
            report_path: Path,
            _payload_path: Path,
            context: dict[str, Any],
            report: dict[str, Any],
        ) -> None:
            report[key] = copy.deepcopy(value)
            refresh_report(report_path, context, report)

        return mutate

    mutations: list[tuple[str, Mutation]] = [
        ("format-drift", report_field("format", "FERRIC-M1-ARTIFACT-IDENTITY-V2")),
        ("authority-promotion", report_field("authority", "semantic-authority")),
        ("nonclaim-weakening", report_field("nonclaim", "Identity only.")),
        ("evidence-kind", report_field("evidence_kind", "independent-validator")),
        ("artifact-kind", report_field("artifact_kind", "Executable")),
        ("target-drift", report_field("artifact_target", "gfx950:xnack-")),
        ("profile-drift", report_field("artifact_profile_id", "runtime")),
        (
            "payload-path-traversal",
            report_field("artifact_relative_path", "../payload.bin"),
        ),
        (
            "payload-path-absolute",
            report_field("artifact_relative_path", "/tmp/payload.bin"),
        ),
        (
            "payload-path-substitution",
            report_field(
                "artifact_relative_path", "identified-artifacts/substitute.bin"
            ),
        ),
        ("payload-sha", report_field("artifact_sha256", digest_bytes(b"other"))),
        ("payload-size", report_field("artifact_size_bytes", 1)),
        ("payload-size-bool", report_field("artifact_size_bytes", True)),
        ("binding-replay", report_field("binding_sha256", digest_bytes(b"binding"))),
        ("obligation-class", report_field("obligation_class", "Assurance")),
        ("obligation-replay", report_field("obligation_id", "m1.r02")),
        ("status-promotion", report_field("obligation_state", "Closed")),
        ("property-omission", report_field("assurance_property_ids", [])),
        (
            "property-duplicate",
            report_field(
                "assurance_property_ids",
                [
                    "model_bundle_well_formed",
                    "model_bundle_well_formed",
                    "resource_bounded",
                ],
            ),
        ),
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
            "source-roster-replay",
            report_field("source_roster_sha256", digest_bytes(b"sources")),
        ),
        (
            "statement-replay",
            report_field("statement_sha256", digest_bytes(b"statement")),
        ),
        ("tcb-roster-replay", report_field("tcb_roster_sha256", digest_bytes(b"tcb"))),
        (
            "tcb-identity-omission",
            report_field(
                "tcb_identity_sha256s", {"tcb.compiler": digest_bytes(b"compiler")}
            ),
        ),
    ]

    for name, mutation in mutations:
        case_root = root / name
        report_path, payload_path, context, report = make_fixture(repo, case_root)
        mutation(report_path, payload_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile artifact-identity report was accepted: {name}")

    def direct(name: str, mutate: Mutation, case: Case = CASES[0]) -> None:
        case_root = root / name
        report_path, payload_path, context, report = make_fixture(repo, case_root, case)
        mutate(report_path, payload_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile artifact-identity context was accepted: {name}")

    direct_cases: list[tuple[str, Mutation]] = [
        (
            "outer-kind",
            lambda _r, _p, c, _a: c["artifact"].__setitem__(
                "kind", "TheoremTranscript"
            ),
        ),
        (
            "outer-id",
            lambda _r, _p, c, _a: c["artifact"].__setitem__(
                "id", "artifact.identity.other"
            ),
        ),
        (
            "outer-relative-path",
            lambda _r, _p, c, _a: c["artifact"].__setitem__(
                "path", "artifacts/other.json"
            ),
        ),
        (
            "outer-sha",
            lambda _r, _p, c, _a: c["artifact"].__setitem__(
                "sha256", digest_bytes(b"other")
            ),
        ),
        (
            "outer-size",
            lambda _r, _p, c, _a: c["artifact"].__setitem__("size_bytes", 1),
        ),
        (
            "subject-replay",
            lambda _r, _p, c, _a: c.__setitem__("subject", "binding:other"),
        ),
        (
            "binding-kind",
            lambda _r, _p, c, _a: c["binding"].__setitem__(
                "evidence_kind", "verus-theorem"
            ),
        ),
        (
            "binding-artifact",
            lambda _r, _p, c, _a: c["binding"].__setitem__(
                "artifact_id", "artifact.other"
            ),
        ),
        (
            "binding-digest",
            lambda _r, _p, c, _a: c["binding"].__setitem__(
                "binding_sha256", digest_bytes(b"other")
            ),
        ),
        ("binding-tcb-order", lambda _r, _p, c, _a: c["binding"]["tcb_ids"].reverse()),
        (
            "binding-profile",
            lambda _r, _p, c, _a: c["binding"].__setitem__("profile_id", "nonclaim"),
        ),
        (
            "binding-path",
            lambda _r, _p, c, _a: c["binding"].__setitem__("path_id", "m1-tcb"),
        ),
        (
            "binding-statement",
            lambda _r, _p, c, _a: c["binding"].__setitem__(
                "statement_sha256", digest_bytes(b"other")
            ),
        ),
        (
            "path-availability",
            lambda _r, _p, c, _a: c["path_resolution"].__setitem__(
                "availability", "ExistingFoundation"
            ),
        ),
        (
            "path-file",
            lambda _r, _p, c, _a: c["path_resolution"].__setitem__(
                "path", "docs/ASSURANCE.md"
            ),
        ),
        (
            "path-repository",
            lambda _r, _p, c, _a: c["path_resolution"].__setitem__(
                "repository", "fe2o3"
            ),
        ),
        (
            "path-source",
            lambda _r, _p, c, _a: c["path_resolution"].__setitem__(
                "source_identity_id", "source.fe2o3"
            ),
        ),
        ("source-order", lambda _r, _p, c, _a: c["sources"].reverse()),
        (
            "source-duplicate",
            lambda _r, _p, c, _a: c["sources"].__setitem__(
                1, copy.deepcopy(c["sources"][0])
            ),
        ),
        (
            "source-commit-drift",
            lambda _r, _p, c, _a: c["sources"][1].__setitem__(
                "commit", digest_bytes(b"other commit")[:40]
            ),
        ),
        (
            "source-tree-drift",
            lambda _r, _p, c, _a: c["sources"][0].__setitem__(
                "tree", digest_bytes(b"other tree")[:40]
            ),
        ),
        (
            "source-closure-drift",
            lambda _r, _p, c, _a: c["sources"][0].__setitem__(
                "source_closure_sha256", digest_bytes(b"other closure")
            ),
        ),
        (
            "source-base",
            lambda _r, _p, c, _a: c["sources"][1].__setitem__(
                "base_commit", digest_bytes(b"other base")[:40]
            ),
        ),
        (
            "source-repository",
            lambda _r, _p, c, _a: c["sources"][0].__setitem__("repository", "ferric"),
        ),
        ("tcb-order", lambda _r, _p, c, _a: c["tcb"].reverse()),
        (
            "tcb-duplicate",
            lambda _r, _p, c, _a: c["tcb"].__setitem__(1, copy.deepcopy(c["tcb"][0])),
        ),
        ("tcb-kind", lambda _r, _p, c, _a: c["tcb"][0].__setitem__("kind", "Runtime")),
        (
            "tcb-identity-drift",
            lambda _r, _p, c, _a: c["tcb"][0].__setitem__(
                "identity_sha256", digest_bytes(b"other tcb")
            ),
        ),
        (
            "requirements-context",
            lambda _r, _p, c, _a: c.__setitem__(
                "requirements_sha256", digest_bytes(b"other requirements")
            ),
        ),
        (
            "context-format",
            lambda _r, _p, c, _a: c.__setitem__(
                "format", "ferric.m1-evidence-index.v2"
            ),
        ),
    ]
    for name, mutation in direct_cases:
        direct(name, mutation)

    case_root = root / "payload-tamper"
    report_path, payload_path, context, _ = make_fixture(repo, case_root)
    payload_path.write_bytes(b"substituted payload\n")
    if invoke(validator, context).returncode == 0:
        fail("substituted payload bytes were accepted")

    case_root = root / "payload-symlink"
    report_path, payload_path, context, _ = make_fixture(repo, case_root)
    target = case_root / "payload-target.bin"
    payload_path.rename(target)
    payload_path.symlink_to(target)
    if invoke(validator, context).returncode == 0:
        fail("symlink payload was accepted")

    case_root = root / "payload-parent-symlink"
    report_path, payload_path, context, _ = make_fixture(repo, case_root)
    payload_dir = payload_path.parent
    target_dir = case_root / "payload-directory-target"
    payload_dir.rename(target_dir)
    payload_dir.symlink_to(target_dir, target_is_directory=True)
    if invoke(validator, context).returncode == 0:
        fail("payload below a symlink directory was accepted")

    case_root = root / "report-symlink"
    report_path, _, context, _ = make_fixture(repo, case_root)
    target = case_root / "report-target.json"
    report_path.rename(target)
    report_path.symlink_to(target)
    if invoke(validator, context).returncode == 0:
        fail("symlink report was accepted")

    case_root = root / "noncanonical-report"
    report_path, _, context, report = make_fixture(repo, case_root)
    raw = (json.dumps(report, ensure_ascii=True, sort_keys=True) + "\n").encode("ascii")
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, context).returncode == 0:
        fail("noncanonical report was accepted")

    case_root = root / "duplicate-report-key"
    report_path, _, context, report = make_fixture(repo, case_root)
    raw = canonical_bytes(report).replace(
        b'{\n  "artifact_kind":', b'{\n  "format": "duplicate",\n  "artifact_kind":', 1
    )
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, context).returncode == 0:
        fail("duplicate-key report was accepted")

    case_root = root / "extra-report-field"
    report_path, _, context, report = make_fixture(repo, case_root)
    report["semantic_correctness"] = True
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("report with an extra authority field was accepted")

    replay_a = root / "replay-a"
    replay_b = root / "replay-b"
    first_report, _, _, _ = make_fixture(repo, replay_a, CASES[0])
    second_report, _, second_context, _ = make_fixture(repo, replay_b, CASES[1])
    raw = first_report.read_bytes()
    second_report.write_bytes(raw)
    second_context["artifact"]["sha256"] = digest_bytes(raw)
    second_context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, second_context).returncode == 0:
        fail("report replay across bindings was accepted")

    raw_root = root / "raw-context"
    _, _, context, _ = make_fixture(repo, raw_root)
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
    if invoke(validator, context, raw_context=noncanonical).returncode == 0:
        fail("noncanonical validator context was accepted")
    if invoke(validator, context, raw_context=duplicate).returncode == 0:
        fail("duplicate-key validator context was accepted")
    extra = copy.deepcopy(context)
    extra["validator_path"] = "self-selected.py"
    if invoke(validator, extra).returncode == 0:
        fail("index-selected validator field was accepted")
    if invoke(validator, context, protocol=PROTOCOL + ".drift").returncode == 0:
        fail("wrong validator protocol was accepted")
    if invoke(validator, context, raw_context=b"").returncode == 0:
        fail("empty validator context was accepted")
    canonical = (
        json.dumps(context, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n\n"
    ).encode("ascii")
    if invoke(validator, context, raw_context=canonical).returncode == 0:
        fail("extra trailing validator context was accepted")

    return len(mutations) + len(direct_cases) + 14


def audit_checker_pin(repo: Path, validator: Path) -> None:
    checker_path = repo / "proofs/check-m1-evidence-index.py"
    checker = load_module(checker_path, "ferric_m1_evidence_checker")
    expected = (
        "proofs/m1/evidence/validate-artifact-identity.py",
        PROTOCOL,
        digest_file(validator),
    )
    if checker.TRUSTED_VALIDATORS.get("artifact-identity") != expected:
        fail("checker-owned artifact-identity path, protocol, or source pin drifted")


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
        fail("M1 roadmap or assurance state was changed by identity validation")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: test-artifact-identity-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-artifact-identity.py"
    audit_checker_pin(repo, validator)
    audit_open_requirements(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-artifact-identity.") as raw:
        root = Path(raw)
        canonical_cases(repo, validator, root / "canonical")
        hostile_count = hostile_cases(repo, validator, root / "hostile")
    print(
        "PASS: M1 artifact-identity validator accepted 3 canonical reports "
        f"and rejected {hostile_count} hostile fixtures"
    )


if __name__ == "__main__":
    main()
