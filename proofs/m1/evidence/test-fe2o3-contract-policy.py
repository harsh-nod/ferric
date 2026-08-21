#!/usr/bin/env python3
"""Exercise canonical and hostile M1 fe2o3-contract reports."""

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


PROTOCOL = "ferric.m1-validator.fe2o3-contract.v1"
REPORT_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-V1"
CONTRACT_BODY_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-BODY-V1"
CONTRACT_SET_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-SET-V1"
CONTRACT_SET_SCHEMA = "fe2o3-proof-contracts::ContractSetV1"
CONTRACT_SET_SOURCE_PATH = "crates/fe2o3-proof-contracts/src/model.rs"
CONTRACT_SET_VALIDATION = "ContractSetV1::validate_closed-structural-only"
PROPERTY_KIND_NAMESPACE = "harsh-nod.ferric.m1.fe2o3-contract-binding.v1"
PROPERTY_KIND_CODE = 1
CONTRACT_TARGET = "gfx942:xnack-"
AUTHORITY = "contract-declaration-structure-only"
NONCLAIM = (
    "This report authenticates an exact fe2o3 ContractSetV1 and Contracted "
    "property declaration only. A contract is not implementation or proof and "
    "grants no machine-refinement, load, launch, hardware, performance, or "
    "qualification authority."
)
FERRIC_BASE = "c5a86fd56c1c817664593df25c04bbed30e84971"
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = ("Compiler", "Hardware", "Runtime")
Case = tuple[str, str, str, str, str]
CASES: tuple[Case, ...] = (
    ("composition-ferric", "Roadmap", "m1.r17", "physical-runner", "composition"),
    ("kernel-fe2o3", "Roadmap", "m1.r06", "fe2o3-gemm", "kernel"),
    (
        "runtime-fe2o3",
        "Assurance",
        "lifetime_safe",
        "fe2o3-aql-foundation",
        "runtime",
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


def domain_digest(domain: str, parts: list[bytes]) -> str:
    hasher = hashlib.sha256()
    encoded_domain = domain.encode("ascii")
    hasher.update(len(encoded_domain).to_bytes(8, "big"))
    hasher.update(encoded_domain)
    for part in parts:
        hasher.update(len(part).to_bytes(8, "big"))
        hasher.update(part)
    return hasher.hexdigest()


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


def assurance_declarations(
    requirements: dict[str, Any], assurance_property_ids: list[str]
) -> list[dict[str, Any]]:
    by_name = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    return [
        {
            "boundary_sha256": digest_bytes(
                by_name[identifier]["boundary"].encode("utf-8")
            ),
            "fe2o3_kind": by_name[identifier]["fe2o3_kind"],
            "name": identifier,
            "obligation_state": "Open",
            "required_status_at_closure": by_name[identifier][
                "required_status_at_closure"
            ],
        }
        for identifier in assurance_property_ids
    ]


def contract_set_declaration(
    binding: dict[str, Any], contract_body_sha256: str
) -> dict[str, Any]:
    identity_parts = [
        binding["obligation_class"].encode("ascii"),
        binding["obligation_id"].encode("ascii"),
        binding["path_id"].encode("ascii"),
        binding["profile_id"].encode("ascii"),
        binding["binding_sha256"].encode("ascii"),
    ]
    property_identity = domain_digest(
        "ferric.m1.fe2o3-contract.property-identity.v1", identity_parts
    )
    evidence_identity = domain_digest(
        "ferric.m1.fe2o3-contract.evidence-identity.v1",
        identity_parts + [bytes.fromhex(contract_body_sha256)],
    )
    obligation_identity = domain_digest(
        "ferric.m1.fe2o3-contract.obligation-identity.v1", identity_parts
    )
    return {
        "correspondences": [],
        "format": CONTRACT_SET_FORMAT,
        "obligations": [
            {
                "identity_sha256": obligation_identity,
                "property_identity_sha256": property_identity,
                "required_status": "Contracted",
                "satisfaction": {
                    "evidence_identity_sha256": evidence_identity,
                    "property_identity_sha256": property_identity,
                    "statement_identity_sha256": binding["statement_sha256"],
                    "status": "Contracted",
                },
                "statement_identity_sha256": binding["statement_sha256"],
            }
        ],
        "properties": [
            {
                "evidence": {
                    "binding": {
                        "identity_sha256": evidence_identity,
                        "property_identity_sha256": property_identity,
                        "statement_identity_sha256": binding["statement_sha256"],
                    },
                    "contract_artifact": {
                        "bytes_sha256": contract_body_sha256,
                        "format_sha256": domain_digest(
                            "ferric.artifact-format.v1",
                            [CONTRACT_BODY_FORMAT.encode("ascii")],
                        ),
                    },
                    "variant": "ContractedEvidenceV1",
                },
                "identity_sha256": property_identity,
                "kind": {
                    "code": PROPERTY_KIND_CODE,
                    "namespace_sha256": domain_digest(
                        "ferric.property-kind.extension.v1",
                        [PROPERTY_KIND_NAMESPACE.encode("ascii")],
                    ),
                    "variant": "Extension",
                },
                "statement_identity_sha256": binding["statement_sha256"],
                "status": "Contracted",
            }
        ],
        "schema": CONTRACT_SET_SCHEMA,
        "schema_source_path": CONTRACT_SET_SOURCE_PATH,
        "trusted_computing_base": [],
        "validation": CONTRACT_SET_VALIDATION,
    }


def make_fixture(
    repo: Path, root: Path, case: Case = CASES[0]
) -> tuple[Path, dict[str, Any], dict[str, Any]]:
    name, obligation_class, obligation_id, path_id, profile_id = case
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="ascii"))
    spec, statement, assurance_properties = requirement_spec(
        requirements, obligation_class, obligation_id
    )
    path_record = next(
        item for item in requirements["path_obligations"] if item["id"] == path_id
    )
    artifact_id = f"artifact.fe2o3-contract.{name}"
    binding_id = f"binding.fe2o3-contract.{name}"
    report_relative = f"artifacts/{artifact_id}.fe2o3-contract.json"
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
        "evidence_kind": "fe2o3-contract",
        "id": binding_id,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "path_id": path_id,
        "profile_id": profile_id,
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
    declarations = assurance_declarations(requirements, assurance_properties)
    source_roster_sha256 = canonical_digest(sources)
    tcb_roster_sha256 = canonical_digest(tcb)
    contract_body_relative = f"contracts/{artifact_id}.fe2o3-contract-body.json"
    contract_body_path = root / contract_body_relative
    contract_body_path.parent.mkdir(parents=True, exist_ok=True)
    contract_body = {
        "assurance_property_declarations": declarations,
        "binding_sha256": binding["binding_sha256"],
        "format": CONTRACT_BODY_FORMAT,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "path_id": path_id,
        "profile_id": profile_id,
        "requirements_sha256": context["requirements_sha256"],
        "source_roster_sha256": source_roster_sha256,
        "statement_sha256": binding["statement_sha256"],
        "target": CONTRACT_TARGET,
        "tcb_roster_sha256": tcb_roster_sha256,
    }
    contract_body_bytes = canonical_bytes(contract_body)
    contract_body_path.write_bytes(contract_body_bytes)
    contract_body_sha256 = digest_bytes(contract_body_bytes)

    contract_set_relative = f"contract-sets/{artifact_id}.fe2o3-contract-set.json"
    contract_set_path = root / contract_set_relative
    contract_set_path.parent.mkdir(parents=True, exist_ok=True)
    contract_set = contract_set_declaration(binding, contract_body_sha256)
    contract_set_bytes = canonical_bytes(contract_set)
    contract_set_path.write_bytes(contract_set_bytes)
    report = {
        "assurance_property_declarations": copy.deepcopy(declarations),
        "authority": AUTHORITY,
        "binding_sha256": binding["binding_sha256"],
        "bound_source_identity_sha256": canonical_digest(
            source_by_id[source_identity_id]
        ),
        "contract_body_path": contract_body_relative,
        "contract_body_sha256": contract_body_sha256,
        "contract_body_size_bytes": len(contract_body_bytes),
        "contract_set_path": contract_set_relative,
        "contract_set_schema": CONTRACT_SET_SCHEMA,
        "contract_set_sha256": digest_bytes(contract_set_bytes),
        "contract_set_size_bytes": len(contract_set_bytes),
        "contract_set_source_path": CONTRACT_SET_SOURCE_PATH,
        "contract_set_validation": CONTRACT_SET_VALIDATION,
        "contract_target": CONTRACT_TARGET,
        "evidence_kind": "fe2o3-contract",
        "format": REPORT_FORMAT,
        "nonclaim": NONCLAIM,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "obligation_state": "Open",
        "path_id": path_id,
        "path_resolution_sha256": canonical_digest(context["path_resolution"]),
        "profile_id": profile_id,
        "requirements_sha256": context["requirements_sha256"],
        "source_identity_id": source_identity_id,
        "source_roster_sha256": source_roster_sha256,
        "statement_sha256": binding["statement_sha256"],
        "tcb_identity_sha256s": {item["id"]: item["identity_sha256"] for item in tcb},
        "tcb_roster_sha256": tcb_roster_sha256,
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

    def declaration_field(key: str, value: Any) -> Mutation:
        def mutate(
            report_path: Path,
            context: dict[str, Any],
            report: dict[str, Any],
        ) -> None:
            report["assurance_property_declarations"][0][key] = copy.deepcopy(value)
            refresh_report(report_path, context, report)

        return mutate

    report_mutations: list[tuple[str, Mutation]] = [
        ("format-drift", report_field("format", "FERRIC-M1-FE2O3-CONTRACT-V2")),
        ("authority-promotion", report_field("authority", "runtime-authority")),
        ("nonclaim-weakening", report_field("nonclaim", "Contract only.")),
        ("evidence-kind", report_field("evidence_kind", "external-contract")),
        (
            "contract-set-schema",
            report_field("contract_set_schema", "fe2o3::ContractSetV2"),
        ),
        (
            "contract-set-source",
            report_field("contract_set_source_path", "crates/other/src/model.rs"),
        ),
        (
            "contract-set-validation",
            report_field("contract_set_validation", "semantic-proof"),
        ),
        ("target-drift", report_field("contract_target", "gfx950:xnack-")),
        ("profile-drift", report_field("profile_id", "runtime")),
        (
            "contract-body-path",
            report_field("contract_body_path", "contracts/other.json"),
        ),
        (
            "contract-body-traversal",
            report_field("contract_body_path", "../contract-body.json"),
        ),
        (
            "contract-body-sha",
            report_field("contract_body_sha256", digest_bytes(b"body")),
        ),
        ("contract-body-size", report_field("contract_body_size_bytes", 1)),
        ("contract-body-size-bool", report_field("contract_body_size_bytes", True)),
        (
            "contract-set-path",
            report_field("contract_set_path", "contract-sets/other.json"),
        ),
        (
            "contract-set-traversal",
            report_field("contract_set_path", "../contract-set.json"),
        ),
        ("contract-set-sha", report_field("contract_set_sha256", digest_bytes(b"set"))),
        ("contract-set-size", report_field("contract_set_size_bytes", 1)),
        ("contract-set-size-bool", report_field("contract_set_size_bytes", True)),
        ("binding-replay", report_field("binding_sha256", digest_bytes(b"binding"))),
        ("obligation-class", report_field("obligation_class", "Assurance")),
        ("obligation-replay", report_field("obligation_id", "m1.r18")),
        ("status-promotion", report_field("obligation_state", "Closed")),
        ("property-omission", report_field("assurance_property_declarations", [])),
        (
            "property-kind-drift",
            declaration_field("fe2o3_kind", "FunctionalCorrectness"),
        ),
        ("property-status-promotion", declaration_field("obligation_state", "Closed")),
        (
            "property-closure-status",
            declaration_field("required_status_at_closure", "Validated"),
        ),
        (
            "property-boundary",
            declaration_field("boundary_sha256", digest_bytes(b"boundary")),
        ),
        ("path-replay", report_field("path_id", "fe2o3-batch")),
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
            fail(f"hostile fe2o3-contract report was accepted: {name}")

    def direct(name: str, mutate: Mutation, case: Case = CASES[0]) -> None:
        report_path, context, report = make_fixture(repo, root / name, case)
        mutate(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile fe2o3-contract context was accepted: {name}")

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
                "id", "artifact.fe2o3-contract.other"
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
                "evidence_kind", "external-contract"
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
            lambda _p, c, _r: c["binding"].__setitem__("profile_id", "qualification"),
        ),
        (
            "binding-path",
            lambda _p, c, _r: c["binding"].__setitem__("path_id", "fe2o3-batch"),
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

    non_runtime = ("non-runtime", "Roadmap", "m1.r01", "bundle-auth", "admission")
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
        fail("symlink fe2o3-contract report was accepted")

    report_path, context, _ = make_fixture(repo, root / "report-parent-symlink")
    report_dir = report_path.parent
    target_dir = report_dir.parent / "artifact-target"
    report_dir.rename(target_dir)
    report_dir.symlink_to(target_dir.name, target_is_directory=True)
    if invoke(validator, context).returncode == 0:
        fail("fe2o3-contract report below a symlink directory was accepted")

    companion_cases = [
        ("contract-body", "contract_body_path"),
        ("contract-set", "contract_set_path"),
    ]
    for name, field in companion_cases:
        report_path, context, report = make_fixture(repo, root / f"{name}-symlink")
        companion = report_path.parents[1] / report[field]
        target = companion.parent / "target.json"
        companion.rename(target)
        companion.symlink_to(target.name)
        if invoke(validator, context).returncode == 0:
            fail(f"symlink fe2o3 {name} was accepted")

    body_mutations: list[tuple[str, Callable[[dict[str, Any]], None]]] = [
        ("body-format", lambda value: value.__setitem__("format", "V2")),
        ("body-target", lambda value: value.__setitem__("target", "gfx950:xnack-")),
        (
            "body-binding",
            lambda value: value.__setitem__("binding_sha256", digest_bytes(b"binding")),
        ),
        ("body-obligation", lambda value: value.__setitem__("obligation_id", "m1.r18")),
        ("body-path", lambda value: value.__setitem__("path_id", "fe2o3-batch")),
        ("body-profile", lambda value: value.__setitem__("profile_id", "runtime")),
        (
            "body-requirements",
            lambda value: value.__setitem__(
                "requirements_sha256", digest_bytes(b"requirements")
            ),
        ),
        (
            "body-sources",
            lambda value: value.__setitem__(
                "source_roster_sha256", digest_bytes(b"sources")
            ),
        ),
        (
            "body-statement",
            lambda value: value.__setitem__(
                "statement_sha256", digest_bytes(b"statement")
            ),
        ),
        (
            "body-tcb",
            lambda value: value.__setitem__("tcb_roster_sha256", digest_bytes(b"tcb")),
        ),
        (
            "body-properties",
            lambda value: value.__setitem__("assurance_property_declarations", []),
        ),
    ]
    for name, mutation in body_mutations:
        report_path, context, report = make_fixture(repo, root / name)
        companion = report_path.parents[1] / report["contract_body_path"]
        value = json.loads(companion.read_text(encoding="ascii"))
        mutation(value)
        raw = canonical_bytes(value)
        companion.write_bytes(raw)
        report["contract_body_sha256"] = digest_bytes(raw)
        report["contract_body_size_bytes"] = len(raw)
        refresh_report(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile fe2o3 contract body was accepted: {name}")

    set_mutations: list[tuple[str, Callable[[dict[str, Any]], None]]] = [
        ("set-format", lambda value: value.__setitem__("format", "V2")),
        ("set-schema", lambda value: value.__setitem__("schema", "ContractSetV2")),
        (
            "set-source",
            lambda value: value.__setitem__("schema_source_path", "other.rs"),
        ),
        (
            "set-validation",
            lambda value: value.__setitem__("validation", "semantic-proof"),
        ),
        ("set-property-omission", lambda value: value.__setitem__("properties", [])),
        (
            "set-property-status",
            lambda value: value["properties"][0].__setitem__("status", "Proved"),
        ),
        (
            "set-property-identity",
            lambda value: value["properties"][0].__setitem__(
                "identity_sha256", digest_bytes(b"property")
            ),
        ),
        (
            "set-kind-code",
            lambda value: value["properties"][0]["kind"].__setitem__("code", 2),
        ),
        (
            "set-kind-namespace",
            lambda value: value["properties"][0]["kind"].__setitem__(
                "namespace_sha256", digest_bytes(b"namespace")
            ),
        ),
        (
            "set-evidence-variant",
            lambda value: value["properties"][0]["evidence"].__setitem__(
                "variant", "ProvedEvidenceV1"
            ),
        ),
        (
            "set-evidence-binding",
            lambda value: value["properties"][0]["evidence"]["binding"].__setitem__(
                "identity_sha256", digest_bytes(b"evidence")
            ),
        ),
        (
            "set-contract-artifact",
            lambda value: value["properties"][0]["evidence"][
                "contract_artifact"
            ].__setitem__("bytes_sha256", digest_bytes(b"artifact")),
        ),
        ("set-obligation-omission", lambda value: value.__setitem__("obligations", [])),
        (
            "set-required-status",
            lambda value: value["obligations"][0].__setitem__(
                "required_status", "Proved"
            ),
        ),
        (
            "set-satisfaction-status",
            lambda value: value["obligations"][0]["satisfaction"].__setitem__(
                "status", "Proved"
            ),
        ),
        (
            "set-satisfaction-evidence",
            lambda value: value["obligations"][0]["satisfaction"].__setitem__(
                "evidence_identity_sha256", digest_bytes(b"evidence")
            ),
        ),
        (
            "set-tcb-injection",
            lambda value: value.__setitem__(
                "trusted_computing_base", [{"authority": "self"}]
            ),
        ),
        (
            "set-correspondence-injection",
            lambda value: value.__setitem__("correspondences", [{"claim": "machine"}]),
        ),
    ]
    for name, mutation in set_mutations:
        report_path, context, report = make_fixture(repo, root / name)
        companion = report_path.parents[1] / report["contract_set_path"]
        value = json.loads(companion.read_text(encoding="ascii"))
        mutation(value)
        raw = canonical_bytes(value)
        companion.write_bytes(raw)
        report["contract_set_sha256"] = digest_bytes(raw)
        report["contract_set_size_bytes"] = len(raw)
        refresh_report(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"hostile fe2o3 ContractSet was accepted: {name}")

    for name, field, sha_field, size_field in (
        (
            "body",
            "contract_body_path",
            "contract_body_sha256",
            "contract_body_size_bytes",
        ),
        (
            "set",
            "contract_set_path",
            "contract_set_sha256",
            "contract_set_size_bytes",
        ),
    ):
        report_path, context, report = make_fixture(repo, root / f"noncanonical-{name}")
        companion = report_path.parents[1] / report[field]
        value = json.loads(companion.read_text(encoding="ascii"))
        raw = (json.dumps(value, ensure_ascii=True, sort_keys=True) + "\n").encode(
            "ascii"
        )
        companion.write_bytes(raw)
        report[sha_field] = digest_bytes(raw)
        report[size_field] = len(raw)
        refresh_report(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"noncanonical fe2o3 {name} companion was accepted")

        report_path, context, report = make_fixture(
            repo, root / f"duplicate-key-{name}"
        )
        companion = report_path.parents[1] / report[field]
        raw = companion.read_bytes().replace(
            b"{\n",
            b'{\n  "format": "duplicate",\n',
            1,
        )
        companion.write_bytes(raw)
        report[sha_field] = digest_bytes(raw)
        report[size_field] = len(raw)
        refresh_report(report_path, context, report)
        if invoke(validator, context).returncode == 0:
            fail(f"duplicate-key fe2o3 {name} companion was accepted")

    report_path, context, report = make_fixture(repo, root / "noncanonical-report")
    raw = (json.dumps(report, ensure_ascii=True, sort_keys=True) + "\n").encode("ascii")
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, context).returncode == 0:
        fail("noncanonical fe2o3-contract report was accepted")

    report_path, context, report = make_fixture(repo, root / "duplicate-report-key")
    raw = canonical_bytes(report).replace(
        b'{\n  "assurance_property_declarations":',
        b'{\n  "format": "duplicate",\n  "assurance_property_declarations":',
        1,
    )
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, context).returncode == 0:
        fail("duplicate-key fe2o3-contract report was accepted")

    report_path, context, report = make_fixture(repo, root / "extra-report-field")
    report["contract_satisfied"] = True
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("fe2o3-contract report with extra authority was accepted")

    first_path, _, _ = make_fixture(repo, root / "replay-a", CASES[0])
    second_path, second_context, _ = make_fixture(repo, root / "replay-b", CASES[1])
    raw = first_path.read_bytes()
    second_path.write_bytes(raw)
    second_context["artifact"]["sha256"] = digest_bytes(raw)
    second_context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, second_context).returncode == 0:
        fail("fe2o3-contract report replay across bindings was accepted")

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
        fail("noncanonical fe2o3-contract context was accepted")
    if invoke(validator, context, raw_context=duplicate).returncode == 0:
        fail("duplicate-key fe2o3-contract context was accepted")
    extra = copy.deepcopy(context)
    extra["validator_path"] = "self-selected.py"
    if invoke(validator, extra).returncode == 0:
        fail("index-selected validator path was accepted")
    if invoke(validator, context, protocol=PROTOCOL + ".drift").returncode == 0:
        fail("wrong fe2o3-contract validator protocol was accepted")
    if invoke(validator, context, raw_context=b"").returncode == 0:
        fail("empty fe2o3-contract context was accepted")
    extra_newline = (
        json.dumps(context, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n\n"
    ).encode("ascii")
    if invoke(validator, context, raw_context=extra_newline).returncode == 0:
        fail("fe2o3-contract context with extra trailing data was accepted")

    return (
        len(report_mutations)
        + len(direct_cases)
        + len(body_mutations)
        + len(set_mutations)
        + len(companion_cases)
        + 17
    )


def audit_checker_pin(repo: Path, validator: Path) -> None:
    checker_path = repo / "proofs/check-m1-evidence-index.py"
    checker = load_module(checker_path, "ferric_m1_evidence_checker")
    expected = (
        "proofs/m1/evidence/validate-fe2o3-contract.py",
        PROTOCOL,
        digest_file(validator),
    )
    if checker.TRUSTED_VALIDATORS.get("fe2o3-contract") != expected:
        fail("checker-owned fe2o3-contract path, protocol, or source pin drifted")


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
        fail("usage: test-fe2o3-contract-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-fe2o3-contract.py"
    audit_checker_pin(repo, validator)
    audit_open_requirements(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-fe2o3-contract.") as raw:
        root = Path(raw)
        canonical_cases(repo, validator, root / "canonical")
        hostile_count = hostile_cases(repo, validator, root / "hostile")
    print(
        "PASS: M1 fe2o3-contract validator accepted 3 canonical reports "
        f"and rejected {hostile_count} hostile fixtures"
    )


if __name__ == "__main__":
    main()
