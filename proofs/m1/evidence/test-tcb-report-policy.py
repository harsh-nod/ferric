#!/usr/bin/env python3
"""Exercise canonical and hostile M1 TCB reports."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.tcb-report.v1"
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
Mutation = Callable[[Path, dict[str, Any], dict[str, Any]], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        fail(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def make_sources(module: Any, requirements: dict[str, Any]) -> list[dict[str, Any]]:
    return [
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
            "base_commit": module.FERRIC_BASE_COMMIT,
            "commit": digest_bytes(b"ferric commit")[:40],
            "id": "source.ferric",
            "repository": "ferric",
            "source_closure_artifact_id": "artifact.source.ferric",
            "source_closure_sha256": digest_bytes(b"ferric source closure"),
            "tree": digest_bytes(b"ferric tree")[:40],
        },
    ]


def refresh_report(
    report_path: Path, context: dict[str, Any], report: dict[str, Any]
) -> None:
    data = canonical_bytes(report)
    report_path.write_bytes(data)
    digest = digest_bytes(data)
    context["artifact"]["sha256"] = digest
    context["artifact"]["size_bytes"] = len(data)
    subject_id = context["tcb_record"]["id"]
    subject = next(item for item in context["tcb"] if item["id"] == subject_id)
    subject["identity_sha256"] = digest
    context["tcb_record"] = copy.deepcopy(subject)


def make_fixture(
    repo: Path, module: Any, root: Path, subject_id: str = TCB_IDS[0]
) -> tuple[Path, dict[str, Any], dict[str, Any]]:
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="ascii"))
    sources = make_sources(module, requirements)
    tcb = [
        {
            "artifact_id": f"artifact.{identifier}",
            "id": identifier,
            "identity_sha256": digest_bytes(identifier.encode("ascii")),
            "kind": TCB_KINDS[identifier],
        }
        for identifier in TCB_IDS
    ]
    subject = next(item for item in tcb if item["id"] == subject_id)
    artifact_id = subject["artifact_id"]
    relative = f"artifacts/{artifact_id}.tcb-report.json"
    report_path = root / relative
    report_path.parent.mkdir(parents=True, exist_ok=True)
    context = {
        "artifact": {
            "id": artifact_id,
            "kind": "TcbReport",
            "path": relative,
            "sha256": digest_bytes(b"pending report"),
            "size_bytes": 1,
        },
        "artifact_absolute_path": str(report_path),
        "format": "ferric.m1-evidence-index.v1",
        "requirements_sha256": digest_file(requirements_path),
        "sources": sources,
        "subject": f"tcb:{subject_id}",
        "tcb": tcb,
        "tcb_record": copy.deepcopy(subject),
    }
    report = {
        "authority": module.AUTHORITY,
        "component_roster": module.expected_components(repo, sources),
        "evidence_kind": "tcb-report",
        "format": module.REPORT_FORMAT,
        "milestone": "M1",
        "nonclaim": module.NONCLAIM,
        "obligation_roster": module.expected_obligations(requirements),
        "obligation_state": "Open",
        "path_roster": module.expected_paths(requirements),
        "profile_roster": module.expected_profiles(requirements),
        "requirements_sha256": context["requirements_sha256"],
        "source_roster": copy.deepcopy(sources),
        "subject_tcb_id": subject_id,
        "subject_tcb_kind": subject["kind"],
        "target": module.REPORT_TARGET,
        "tcb_structure_roster": [
            {"artifact_id": item["artifact_id"], "id": item["id"], "kind": item["kind"]}
            for item in tcb
        ],
        "validator_roster": module.expected_validators(repo),
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


def canonical_cases(repo: Path, module: Any, validator: Path, root: Path) -> None:
    for subject_id in TCB_IDS:
        _, context, _ = make_fixture(repo, module, root / subject_id, subject_id)
        result = invoke(validator, context)
        payload = json.dumps(
            context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
        expected = (
            f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
            f"context_sha256={digest_bytes(payload)}\n"
        ).encode("ascii")
        if result.returncode != 0 or result.stdout != expected:
            fail(
                f"canonical {subject_id} report rejected: "
                f"exit={result.returncode}, output={result.stdout!r}"
            )


def hostile_cases(repo: Path, module: Any, validator: Path, root: Path) -> int:
    def report_field(key: str, value: Any) -> Mutation:
        def mutate(
            report_path: Path, context: dict[str, Any], report: dict[str, Any]
        ) -> None:
            report[key] = copy.deepcopy(value)
            refresh_report(report_path, context, report)

        return mutate

    def report_edit(edit: Callable[[dict[str, Any]], None]) -> Mutation:
        def mutate(
            report_path: Path, context: dict[str, Any], report: dict[str, Any]
        ) -> None:
            edit(report)
            refresh_report(report_path, context, report)

        return mutate

    report_mutations: list[tuple[str, Mutation]] = [
        ("format-drift", report_field("format", "FERRIC-M1-TCB-REPORT-V2")),
        ("authority-promotion", report_field("authority", "qualification-authority")),
        ("nonclaim-weakening", report_field("nonclaim", "Identities are trusted.")),
        ("evidence-kind", report_field("evidence_kind", "independent-validator")),
        ("milestone", report_field("milestone", "M2")),
        ("status-promotion", report_field("obligation_state", "Closed")),
        ("target-drift", report_field("target", "gfx950:xnack-")),
        (
            "requirements-replay",
            report_field("requirements_sha256", digest_bytes(b"other")),
        ),
        ("subject-replay", report_field("subject_tcb_id", "tcb.runtime")),
        ("subject-kind", report_field("subject_tcb_kind", "Runtime")),
        ("component-omission", report_edit(lambda r: r["component_roster"].pop())),
        (
            "component-duplicate",
            report_edit(
                lambda r: r["component_roster"].append(
                    copy.deepcopy(r["component_roster"][0])
                )
            ),
        ),
        ("component-reorder", report_edit(lambda r: r["component_roster"].reverse())),
        (
            "component-id",
            report_edit(
                lambda r: r["component_roster"][0].__setitem__("id", "compiler.other")
            ),
        ),
        (
            "component-kind",
            report_edit(
                lambda r: r["component_roster"][0].__setitem__("kind", "Runtime")
            ),
        ),
        (
            "component-version",
            report_edit(
                lambda r: r["component_roster"][1].__setitem__("version", "1.98.0")
            ),
        ),
        (
            "component-status",
            report_edit(
                lambda r: r["component_roster"][0].__setitem__("status", "Validated")
            ),
        ),
        (
            "component-authority",
            report_edit(
                lambda r: r["component_roster"][0].__setitem__(
                    "authority", "machine-refinement"
                )
            ),
        ),
        (
            "component-identity",
            report_edit(
                lambda r: r["component_roster"][0].__setitem__(
                    "identity_sha256", digest_bytes(b"component")
                )
            ),
        ),
        ("source-omission", report_edit(lambda r: r["source_roster"].pop())),
        (
            "source-duplicate",
            report_edit(
                lambda r: r["source_roster"].__setitem__(
                    1, copy.deepcopy(r["source_roster"][0])
                )
            ),
        ),
        ("source-reorder", report_edit(lambda r: r["source_roster"].reverse())),
        (
            "source-commit",
            report_edit(
                lambda r: r["source_roster"][0].__setitem__(
                    "commit", digest_bytes(b"commit")[:40]
                )
            ),
        ),
        (
            "source-closure",
            report_edit(
                lambda r: r["source_roster"][0].__setitem__(
                    "source_closure_sha256", digest_bytes(b"closure")
                )
            ),
        ),
        ("tcb-omission", report_edit(lambda r: r["tcb_structure_roster"].pop())),
        (
            "tcb-duplicate",
            report_edit(
                lambda r: r["tcb_structure_roster"].__setitem__(
                    1, copy.deepcopy(r["tcb_structure_roster"][0])
                )
            ),
        ),
        ("tcb-reorder", report_edit(lambda r: r["tcb_structure_roster"].reverse())),
        (
            "tcb-kind",
            report_edit(
                lambda r: r["tcb_structure_roster"][0].__setitem__("kind", "Runtime")
            ),
        ),
        (
            "tcb-artifact",
            report_edit(
                lambda r: r["tcb_structure_roster"][0].__setitem__(
                    "artifact_id", "artifact.other"
                )
            ),
        ),
        ("validator-omission", report_edit(lambda r: r["validator_roster"].pop())),
        (
            "validator-duplicate",
            report_edit(
                lambda r: r["validator_roster"].append(
                    copy.deepcopy(r["validator_roster"][0])
                )
            ),
        ),
        ("validator-reorder", report_edit(lambda r: r["validator_roster"].reverse())),
        (
            "validator-path",
            report_edit(
                lambda r: r["validator_roster"][0].__setitem__(
                    "path", "self-selected.py"
                )
            ),
        ),
        (
            "validator-protocol",
            report_edit(
                lambda r: r["validator_roster"][0].__setitem__(
                    "protocol", "ferric.m1-validator.other.v1"
                )
            ),
        ),
        (
            "validator-availability",
            report_edit(
                lambda r: r["validator_roster"][0].__setitem__(
                    "availability", "RequiredFuture"
                )
            ),
        ),
        (
            "validator-source",
            report_edit(
                lambda r: r["validator_roster"][0].__setitem__(
                    "source_sha256", digest_bytes(b"validator")
                )
            ),
        ),
        ("obligation-omission", report_edit(lambda r: r["obligation_roster"].pop())),
        (
            "obligation-duplicate",
            report_edit(
                lambda r: r["obligation_roster"].append(
                    copy.deepcopy(r["obligation_roster"][0])
                )
            ),
        ),
        ("obligation-reorder", report_edit(lambda r: r["obligation_roster"].reverse())),
        (
            "obligation-status",
            report_edit(
                lambda r: r["obligation_roster"][0].__setitem__("status", "Closed")
            ),
        ),
        (
            "obligation-path",
            report_edit(lambda r: r["obligation_roster"][0]["path_ids"].reverse()),
        ),
        (
            "obligation-profile",
            report_edit(lambda r: r["obligation_roster"][0]["profile_ids"].pop()),
        ),
        (
            "obligation-statement",
            report_edit(
                lambda r: r["obligation_roster"][0].__setitem__(
                    "statement_sha256", digest_bytes(b"statement")
                )
            ),
        ),
        ("path-omission", report_edit(lambda r: r["path_roster"].pop())),
        (
            "path-duplicate",
            report_edit(
                lambda r: r["path_roster"].append(copy.deepcopy(r["path_roster"][0]))
            ),
        ),
        ("path-reorder", report_edit(lambda r: r["path_roster"].reverse())),
        (
            "path-status",
            report_edit(lambda r: r["path_roster"][0].__setitem__("status", "Closed")),
        ),
        (
            "path-availability",
            report_edit(
                lambda r: r["path_roster"][0].__setitem__(
                    "availability", "ExistingFoundation"
                )
            ),
        ),
        (
            "path-replay",
            report_edit(
                lambda r: r["path_roster"][0].__setitem__("path", "docs/ASSURANCE.md")
            ),
        ),
        (
            "path-source",
            report_edit(
                lambda r: r["path_roster"][0].__setitem__(
                    "source_identity_id", "source.fe2o3"
                )
            ),
        ),
        ("profile-omission", report_edit(lambda r: r["profile_roster"].pop())),
        (
            "profile-duplicate",
            report_edit(
                lambda r: r["profile_roster"].append(
                    copy.deepcopy(r["profile_roster"][0])
                )
            ),
        ),
        ("profile-reorder", report_edit(lambda r: r["profile_roster"].reverse())),
        (
            "profile-id",
            report_edit(lambda r: r["profile_roster"][0].__setitem__("id", "other")),
        ),
        (
            "profile-kind-omission",
            report_edit(
                lambda r: r["profile_roster"][0]["evidence_kinds"].remove("tcb-report")
            ),
        ),
        (
            "profile-kind-reorder",
            report_edit(lambda r: r["profile_roster"][0]["evidence_kinds"].reverse()),
        ),
    ]
    for name, mutation in report_mutations:
        report_path, context, report = make_fixture(repo, module, root / name)
        mutation(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile TCB report was accepted: {name}")

    def direct(name: str, mutation: Mutation) -> None:
        report_path, context, report = make_fixture(repo, module, root / name)
        mutation(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile TCB context was accepted: {name}")

    direct_cases: list[tuple[str, Mutation]] = [
        (
            "outer-kind",
            lambda _p, c, _r: c["artifact"].__setitem__("kind", "ContractDocument"),
        ),
        (
            "outer-id",
            lambda _p, c, _r: c["artifact"].__setitem__("id", "artifact.other"),
        ),
        (
            "outer-path",
            lambda _p, c, _r: c["artifact"].__setitem__("path", "artifacts/other.json"),
        ),
        (
            "outer-traversal",
            lambda _p, c, _r: c["artifact"].__setitem__("path", "../report.json"),
        ),
        (
            "outer-sha",
            lambda _p, c, _r: c["artifact"].__setitem__(
                "sha256", digest_bytes(b"other")
            ),
        ),
        ("outer-size", lambda _p, c, _r: c["artifact"].__setitem__("size_bytes", 1)),
        (
            "outer-size-bool",
            lambda _p, c, _r: c["artifact"].__setitem__("size_bytes", True),
        ),
        ("subject", lambda _p, c, _r: c.__setitem__("subject", "tcb:tcb.runtime")),
        (
            "record-kind",
            lambda _p, c, _r: c["tcb_record"].__setitem__("kind", "Runtime"),
        ),
        (
            "record-artifact",
            lambda _p, c, _r: c["tcb_record"].__setitem__(
                "artifact_id", "artifact.other"
            ),
        ),
        (
            "record-identity",
            lambda _p, c, _r: c["tcb_record"].__setitem__(
                "identity_sha256", digest_bytes(b"other")
            ),
        ),
        ("tcb-order", lambda _p, c, _r: c["tcb"].reverse()),
        (
            "tcb-duplicate",
            lambda _p, c, _r: c["tcb"].__setitem__(1, copy.deepcopy(c["tcb"][0])),
        ),
        (
            "tcb-kind-context",
            lambda _p, c, _r: c["tcb"][0].__setitem__("kind", "Runtime"),
        ),
        ("source-order-context", lambda _p, c, _r: c["sources"].reverse()),
        (
            "source-duplicate-context",
            lambda _p, c, _r: c["sources"].__setitem__(
                1, copy.deepcopy(c["sources"][0])
            ),
        ),
        (
            "source-base-context",
            lambda _p, c, _r: c["sources"][0].__setitem__(
                "base_commit", digest_bytes(b"base")[:40]
            ),
        ),
        (
            "source-tree-context",
            lambda _p, c, _r: c["sources"][0].__setitem__(
                "tree", digest_bytes(b"tree")[:40]
            ),
        ),
        (
            "source-closure-context",
            lambda _p, c, _r: c["sources"][0].__setitem__(
                "source_closure_sha256", digest_bytes(b"closure")
            ),
        ),
        (
            "requirements-context",
            lambda _p, c, _r: c.__setitem__(
                "requirements_sha256", digest_bytes(b"requirements")
            ),
        ),
        (
            "format-context",
            lambda _p, c, _r: c.__setitem__("format", "ferric.m1-evidence-index.v2"),
        ),
    ]
    for name, mutation in direct_cases:
        direct(name, mutation)

    report_path, context, _ = make_fixture(repo, module, root / "report-symlink")
    target = report_path.parent / "target.json"
    report_path.rename(target)
    report_path.symlink_to(target.name)
    if invoke(validator, context).returncode == 0:
        fail("symlink TCB report was accepted")

    report_path, context, _ = make_fixture(repo, module, root / "parent-symlink")
    report_dir = report_path.parent
    target_dir = report_dir.parent / "artifact-target"
    report_dir.rename(target_dir)
    report_dir.symlink_to(target_dir.name, target_is_directory=True)
    if invoke(validator, context).returncode == 0:
        fail("TCB report below a symlink directory was accepted")

    report_path, context, report = make_fixture(repo, module, root / "noncanonical")
    raw = (json.dumps(report, ensure_ascii=True, sort_keys=True) + "\n").encode("ascii")
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    context["tcb"][0]["identity_sha256"] = digest_bytes(raw)
    context["tcb_record"] = copy.deepcopy(context["tcb"][0])
    if invoke(validator, context).returncode == 0:
        fail("noncanonical TCB report was accepted")

    report_path, context, report = make_fixture(repo, module, root / "duplicate-key")
    raw = canonical_bytes(report).replace(
        b'{\n  "authority":',
        b'{\n  "format": "duplicate",\n  "authority":',
        1,
    )
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    context["tcb"][0]["identity_sha256"] = digest_bytes(raw)
    context["tcb_record"] = copy.deepcopy(context["tcb"][0])
    if invoke(validator, context).returncode == 0:
        fail("duplicate-key TCB report was accepted")

    report_path, context, report = make_fixture(repo, module, root / "extra-field")
    report["qualified"] = True
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("TCB report with an extra authority field was accepted")

    _, context, _ = make_fixture(repo, module, root / "raw-context")
    compact = json.dumps(
        context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    noncanonical = (
        json.dumps(context, ensure_ascii=True, sort_keys=True) + "\n"
    ).encode("ascii")
    duplicate = (
        (compact + "\n")
        .encode("ascii")
        .replace(b'{"artifact":', b'{"format":"duplicate","artifact":', 1)
    )
    if invoke(validator, context, raw_context=noncanonical).returncode == 0:
        fail("noncanonical TCB context was accepted")
    if invoke(validator, context, raw_context=duplicate).returncode == 0:
        fail("duplicate-key TCB context was accepted")
    extra = copy.deepcopy(context)
    extra["validator_path"] = "self-selected.py"
    if invoke(validator, extra).returncode == 0:
        fail("index-selected TCB validator path was accepted")
    if invoke(validator, context, protocol=PROTOCOL + ".drift").returncode == 0:
        fail("wrong TCB validator protocol was accepted")
    if invoke(validator, context, raw_context=b"").returncode == 0:
        fail("empty TCB context was accepted")
    if (
        invoke(
            validator, context, raw_context=(compact + "\n\n").encode("ascii")
        ).returncode
        == 0
    ):
        fail("TCB context with trailing data was accepted")

    report_path, _, _ = make_fixture(repo, module, root / "toctou-simulation")
    original = module.file_identity
    calls = 0

    def changed(metadata: Any) -> tuple[int, int, int, int, int, int]:
        nonlocal calls
        calls += 1
        value = original(metadata)
        return value if calls == 1 else (*value[:-1], value[-1] + 1)

    module.file_identity = changed
    try:
        with contextlib.redirect_stderr(io.StringIO()):
            try:
                module.read_bounded(
                    report_path, module.MAX_REPORT_BYTES, "racing report"
                )
            except SystemExit:
                pass
            else:
                fail("simulated report replacement during read was accepted")
    finally:
        module.file_identity = original

    return len(report_mutations) + len(direct_cases) + 10


def audit_checker_pin(repo: Path, validator: Path) -> None:
    checker = load_module(
        repo / "proofs/check-m1-evidence-index.py", "ferric_m1_evidence_checker"
    )
    expected = (
        "proofs/m1/evidence/validate-tcb-report.py",
        PROTOCOL,
        digest_file(validator),
    )
    if checker.TRUSTED_VALIDATORS.get("tcb-report") != expected:
        fail("checker-owned TCB-report path, protocol, or source pin drifted")


def audit_open_requirements(repo: Path) -> None:
    requirements = json.loads(
        (repo / "proofs/M1_REQUIREMENTS.json").read_text(encoding="ascii")
    )
    if (
        len(requirements["roadmap_requirements"]) != 33
        or len(requirements["assurance_properties"]) != 17
        or len(requirements["path_obligations"]) != 39
        or any(
            record["obligation_state"] != "Open"
            for key in (
                "roadmap_requirements",
                "assurance_properties",
                "path_obligations",
            )
            for record in requirements[key]
        )
    ):
        fail("M1 status was changed by TCB-report validation")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: test-tcb-report-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-tcb-report.py"
    module = load_module(validator, "ferric_m1_tcb_validator")
    audit_checker_pin(repo, validator)
    audit_open_requirements(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-tcb-report.") as raw:
        root = Path(raw)
        canonical_cases(repo, module, validator, root / "canonical")
        hostile_count = hostile_cases(repo, module, validator, root / "hostile")
    print(
        "PASS: M1 TCB-report validator accepted 3 canonical reports and rejected "
        f"{hostile_count} hostile fixtures"
    )


if __name__ == "__main__":
    main()
