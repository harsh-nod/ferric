#!/usr/bin/env python3
"""Exercise canonical and hostile M1 external-contract reports."""

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


PROTOCOL = "ferric.m1-validator.external-contract.v1"
REPORT_FORMAT = "FERRIC-M1-EXTERNAL-CONTRACT-V1"
PROFILE_ID = "runtime"
CONTRACT_SCOPE = "external-compiler-runtime-hardware-assumptions"
CONTRACT_TARGET = "gfx942:xnack-"
AUTHORITY = "declared-assumptions-only"
ASSUMPTION_IDS = [
    "compiler-object-emission-conforms-to-declared-target",
    "runtime-load-and-dispatch-conform-to-amdhsa-contract",
    "driver-firmware-memory-queue-completion-conform-to-declared-abi",
    "gfx942-execution-conforms-to-declared-isa-and-memory-model",
]
NONCLAIM = (
    "This report authenticates a declaration of external assumptions only. "
    "It does not establish that an assumption is implemented or satisfied and "
    "grants no theorem, machine-refinement, load, launch, hardware, performance, "
    "or qualification authority."
)
FERRIC_BASE = "c5a86fd56c1c817664593df25c04bbed30e84971"
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = ("Compiler", "Hardware", "Runtime")
Case = tuple[str, str, str, str]
CASES: tuple[Case, ...] = (
    ("roadmap-ferric", "Roadmap", "m1.r18", "device-cache"),
    ("assurance-ferric", "Assurance", "rollback_refined", "speculative-graph"),
    (
        "assurance-fe2o3",
        "Assurance",
        "lifetime_safe",
        "fe2o3-aql-foundation",
    ),
)
Mutation = Callable[[Path, dict[str, Any], dict[str, Any]], None]


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
) -> tuple[Path, dict[str, Any], dict[str, Any]]:
    name, obligation_class, obligation_id, path_id = case
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="ascii"))
    spec, statement, assurance_properties = requirement_spec(
        requirements, obligation_class, obligation_id
    )
    path_record = next(
        item for item in requirements["path_obligations"] if item["id"] == path_id
    )
    artifact_id = f"artifact.external-contract.{name}"
    binding_id = f"binding.external-contract.{name}"
    report_relative = f"artifacts/{artifact_id}.external-contract.json"
    report_path = root / report_relative
    report_path.parent.mkdir(parents=True, exist_ok=True)

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
    source_identity_id = f"source.{path_record['repository']}"
    binding = {
        "artifact_id": artifact_id,
        "binding_sha256": "",
        "evidence_kind": "external-contract",
        "id": binding_id,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "path_id": path_id,
        "profile_id": PROFILE_ID,
        "source_identity_id": source_identity_id,
        "statement_sha256": digest_bytes(statement.encode("utf-8")),
        "tcb_ids": list(TCB_IDS),
    }
    context = {
        "artifact": {
            "id": artifact_id,
            "kind": "ContractDocument",
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
    source_by_id = {item["id"]: item for item in sources}
    report = {
        "assumption_ids": copy.deepcopy(ASSUMPTION_IDS),
        "assurance_property_ids": copy.deepcopy(assurance_properties),
        "authority": AUTHORITY,
        "binding_sha256": binding["binding_sha256"],
        "bound_source_identity_sha256": canonical_digest(
            source_by_id[source_identity_id]
        ),
        "contract_scope": CONTRACT_SCOPE,
        "contract_target": CONTRACT_TARGET,
        "evidence_kind": "external-contract",
        "format": REPORT_FORMAT,
        "nonclaim": NONCLAIM,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "obligation_state": "Open",
        "path_id": path_id,
        "path_resolution_sha256": canonical_digest(context["path_resolution"]),
        "profile_id": PROFILE_ID,
        "requirements_sha256": context["requirements_sha256"],
        "source_identity_id": source_identity_id,
        "source_roster_sha256": canonical_digest(sources),
        "statement_sha256": binding["statement_sha256"],
        "tcb_identity_sha256s": {item["id"]: item["identity_sha256"] for item in tcb},
        "tcb_roster_sha256": canonical_digest(tcb),
    }
    refresh_report(report_path, context, report)
    return report_path, context, report


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
        _, context, _ = make_fixture(repo, root / case[0], case)
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
            context: dict[str, Any],
            report: dict[str, Any],
        ) -> None:
            report[key] = copy.deepcopy(value)
            refresh_report(report_path, context, report)

        return mutate

    report_mutations: list[tuple[str, Mutation]] = [
        ("format-drift", report_field("format", "FERRIC-M1-EXTERNAL-CONTRACT-V2")),
        ("authority-promotion", report_field("authority", "runtime-authority")),
        ("nonclaim-weakening", report_field("nonclaim", "Assumptions only.")),
        ("evidence-kind", report_field("evidence_kind", "fe2o3-contract")),
        ("scope-drift", report_field("contract_scope", "whole-system-correctness")),
        ("target-drift", report_field("contract_target", "gfx950:xnack-")),
        ("profile-drift", report_field("profile_id", "composition")),
        ("assumption-omission", report_field("assumption_ids", ASSUMPTION_IDS[:-1])),
        (
            "assumption-injection",
            report_field("assumption_ids", ASSUMPTION_IDS + ["proof-is-assumed"]),
        ),
        (
            "assumption-reorder",
            report_field("assumption_ids", list(reversed(ASSUMPTION_IDS))),
        ),
        ("binding-replay", report_field("binding_sha256", digest_bytes(b"binding"))),
        ("obligation-class", report_field("obligation_class", "Assurance")),
        ("obligation-replay", report_field("obligation_id", "m1.r17")),
        ("status-promotion", report_field("obligation_state", "Closed")),
        ("property-omission", report_field("assurance_property_ids", [])),
        (
            "property-duplicate",
            report_field(
                "assurance_property_ids",
                ["kv_refined", "kv_refined", "scheduler_refined"],
            ),
        ),
        ("path-replay", report_field("path_id", "physical-runner")),
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
            "bound-source-replay",
            report_field("bound_source_identity_sha256", digest_bytes(b"source")),
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
    for name, mutation in report_mutations:
        report_path, context, report = make_fixture(repo, root / name)
        mutation(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile external-contract report was accepted: {name}")

    def direct(name: str, mutate: Mutation, case: Case = CASES[0]) -> None:
        report_path, context, report = make_fixture(repo, root / name, case)
        mutate(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile external-contract context was accepted: {name}")

    direct_cases: list[tuple[str, Mutation]] = [
        (
            "outer-kind",
            lambda _p, c, _r: c["artifact"].__setitem__(
                "kind", "ArtifactIdentityReport"
            ),
        ),
        (
            "outer-id",
            lambda _p, c, _r: c["artifact"].__setitem__(
                "id", "artifact.external-contract.other"
            ),
        ),
        (
            "outer-relative-path",
            lambda _p, c, _r: c["artifact"].__setitem__("path", "artifacts/other.json"),
        ),
        (
            "outer-path-traversal",
            lambda _p, c, _r: c["artifact"].__setitem__("path", "../contract.json"),
        ),
        (
            "outer-sha",
            lambda _p, c, _r: c["artifact"].__setitem__(
                "sha256", digest_bytes(b"other")
            ),
        ),
        (
            "outer-size",
            lambda _p, c, _r: c["artifact"].__setitem__("size_bytes", 1),
        ),
        (
            "outer-size-bool",
            lambda _p, c, _r: c["artifact"].__setitem__("size_bytes", True),
        ),
        ("subject-replay", lambda _p, c, _r: c.__setitem__("subject", "binding:other")),
        (
            "binding-kind",
            lambda _p, c, _r: c["binding"].__setitem__(
                "evidence_kind", "fe2o3-contract"
            ),
        ),
        (
            "binding-artifact",
            lambda _p, c, _r: c["binding"].__setitem__("artifact_id", "artifact.other"),
        ),
        (
            "binding-digest",
            lambda _p, c, _r: c["binding"].__setitem__(
                "binding_sha256", digest_bytes(b"other")
            ),
        ),
        (
            "binding-tcb-order",
            lambda _p, c, _r: c["binding"]["tcb_ids"].reverse(),
        ),
        (
            "binding-profile",
            lambda _p, c, _r: c["binding"].__setitem__("profile_id", "composition"),
        ),
        (
            "binding-path",
            lambda _p, c, _r: c["binding"].__setitem__("path_id", "physical-runner"),
        ),
        (
            "binding-statement",
            lambda _p, c, _r: c["binding"].__setitem__(
                "statement_sha256", digest_bytes(b"other")
            ),
        ),
        (
            "path-availability",
            lambda _p, c, _r: c["path_resolution"].__setitem__(
                "availability", "ExistingFoundation"
            ),
        ),
        (
            "path-file",
            lambda _p, c, _r: c["path_resolution"].__setitem__(
                "path", "docs/ASSURANCE.md"
            ),
        ),
        (
            "path-repository",
            lambda _p, c, _r: c["path_resolution"].__setitem__("repository", "fe2o3"),
        ),
        (
            "path-source",
            lambda _p, c, _r: c["path_resolution"].__setitem__(
                "source_identity_id", "source.fe2o3"
            ),
        ),
        ("source-order", lambda _p, c, _r: c["sources"].reverse()),
        (
            "source-duplicate",
            lambda _p, c, _r: c["sources"].__setitem__(
                1, copy.deepcopy(c["sources"][0])
            ),
        ),
        (
            "source-commit-drift",
            lambda _p, c, _r: c["sources"][1].__setitem__(
                "commit", digest_bytes(b"other commit")[:40]
            ),
        ),
        (
            "source-tree-drift",
            lambda _p, c, _r: c["sources"][0].__setitem__(
                "tree", digest_bytes(b"other tree")[:40]
            ),
        ),
        (
            "source-closure-drift",
            lambda _p, c, _r: c["sources"][0].__setitem__(
                "source_closure_sha256", digest_bytes(b"other closure")
            ),
        ),
        (
            "source-base",
            lambda _p, c, _r: c["sources"][1].__setitem__(
                "base_commit", digest_bytes(b"other base")[:40]
            ),
        ),
        (
            "source-repository",
            lambda _p, c, _r: c["sources"][0].__setitem__("repository", "ferric"),
        ),
        ("tcb-order", lambda _p, c, _r: c["tcb"].reverse()),
        (
            "tcb-duplicate",
            lambda _p, c, _r: c["tcb"].__setitem__(1, copy.deepcopy(c["tcb"][0])),
        ),
        (
            "tcb-kind",
            lambda _p, c, _r: c["tcb"][0].__setitem__("kind", "Runtime"),
        ),
        (
            "tcb-identity-drift",
            lambda _p, c, _r: c["tcb"][0].__setitem__(
                "identity_sha256", digest_bytes(b"other tcb")
            ),
        ),
        (
            "requirements-context",
            lambda _p, c, _r: c.__setitem__(
                "requirements_sha256", digest_bytes(b"other requirements")
            ),
        ),
        (
            "context-format",
            lambda _p, c, _r: c.__setitem__("format", "ferric.m1-evidence-index.v2"),
        ),
    ]
    for name, mutation in direct_cases:
        direct(name, mutation)

    non_runtime = ("non-runtime", "Roadmap", "m1.r01", "bundle-auth")
    direct(
        "non-runtime-profile",
        lambda _p, _c, _r: None,
        non_runtime,
    )

    report_path, context, _ = make_fixture(repo, root / "report-symlink")
    target = report_path.parent / "target.json"
    report_path.rename(target)
    report_path.symlink_to(target.name)
    if invoke(validator, context).returncode == 0:
        fail("symlink external-contract report was accepted")

    report_path, context, _ = make_fixture(repo, root / "report-parent-symlink")
    report_dir = report_path.parent
    target_dir = report_dir.parent / "artifact-target"
    report_dir.rename(target_dir)
    report_dir.symlink_to(target_dir.name, target_is_directory=True)
    if invoke(validator, context).returncode == 0:
        fail("external-contract report below a symlink directory was accepted")

    report_path, context, report = make_fixture(repo, root / "noncanonical-report")
    raw = (json.dumps(report, ensure_ascii=True, sort_keys=True) + "\n").encode("ascii")
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, context).returncode == 0:
        fail("noncanonical external-contract report was accepted")

    report_path, context, report = make_fixture(repo, root / "duplicate-report-key")
    raw = canonical_bytes(report).replace(
        b'{\n  "assumption_ids":',
        b'{\n  "format": "duplicate",\n  "assumption_ids":',
        1,
    )
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, context).returncode == 0:
        fail("duplicate-key external-contract report was accepted")

    report_path, context, report = make_fixture(repo, root / "extra-report-field")
    report["contract_satisfied"] = True
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("external-contract report with extra authority was accepted")

    first_path, _, _ = make_fixture(repo, root / "replay-a", CASES[0])
    second_path, second_context, _ = make_fixture(repo, root / "replay-b", CASES[1])
    raw = first_path.read_bytes()
    second_path.write_bytes(raw)
    second_context["artifact"]["sha256"] = digest_bytes(raw)
    second_context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, second_context).returncode == 0:
        fail("external-contract report replay across bindings was accepted")

    _, context, _ = make_fixture(repo, root / "raw-context")
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
        fail("noncanonical external-contract context was accepted")
    if invoke(validator, context, raw_context=duplicate).returncode == 0:
        fail("duplicate-key external-contract context was accepted")
    extra = copy.deepcopy(context)
    extra["validator_path"] = "self-selected.py"
    if invoke(validator, extra).returncode == 0:
        fail("index-selected validator path was accepted")
    if invoke(validator, context, protocol=PROTOCOL + ".drift").returncode == 0:
        fail("wrong external-contract validator protocol was accepted")
    if invoke(validator, context, raw_context=b"").returncode == 0:
        fail("empty external-contract context was accepted")
    extra_newline = (
        json.dumps(context, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n\n"
    ).encode("ascii")
    if invoke(validator, context, raw_context=extra_newline).returncode == 0:
        fail("external-contract context with extra trailing data was accepted")

    return len(report_mutations) + len(direct_cases) + 11


def audit_checker_pin(repo: Path, validator: Path) -> None:
    checker_path = repo / "proofs/check-m1-evidence-index.py"
    checker = load_module(checker_path, "ferric_m1_evidence_checker")
    expected = (
        "proofs/m1/evidence/validate-external-contract.py",
        PROTOCOL,
        digest_file(validator),
    )
    if checker.TRUSTED_VALIDATORS.get("external-contract") != expected:
        fail("checker-owned external-contract path, protocol, or source pin drifted")


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
        fail("M1 roadmap or assurance state was changed by contract validation")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: test-external-contract-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-external-contract.py"
    audit_checker_pin(repo, validator)
    audit_open_requirements(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-external-contract.") as raw:
        root = Path(raw)
        canonical_cases(repo, validator, root / "canonical")
        hostile_count = hostile_cases(repo, validator, root / "hostile")
    print(
        "PASS: M1 external-contract validator accepted 3 canonical reports "
        f"and rejected {hostile_count} hostile fixtures"
    )


if __name__ == "__main__":
    main()
