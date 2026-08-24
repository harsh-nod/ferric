#!/usr/bin/env python3
"""Create a source-authenticated, planning-only external M1 evidence bundle."""

from __future__ import annotations

import ast
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
import tomllib
from collections import defaultdict
from pathlib import Path
from typing import Any, NoReturn


PLAN_FORMAT = "FERRIC-M1-EVIDENCE-PLAN-V1"
WORK_FORMAT = "FERRIC-M1-EVIDENCE-WORK-QUEUE-V1"
AUTHORITY = "planning-only-no-evidence"
NONCLAIM = (
    "This bundle allocates external M1 evidence work only. It is not an evidence "
    "index, qualification receipt, validation result, or M1 closure claim."
)
TARGET = "gfx942:xnack-"
FE2O3_REPOSITORY = "https://github.com/harsh-nod/fe2o3.git"
FE2O3_DIRECT_PACKAGES = (
    "fe2o3-amdhsa-loader",
    "fe2o3-aql",
    "fe2o3-artifact-transaction",
    "fe2o3-compiler-ffi",
    "fe2o3-hsaco",
    "fe2o3-hsaco-finalize",
    "fe2o3-kfd",
    "fe2o3-llvm-handoff",
    "fe2o3-llvm-text",
    "fe2o3-llvm-worker-handoff",
    "fe2o3-service-host",
    "reserved-fe2o3-symbols",
)
FE2O3_RESOLVED_PACKAGES = (
    ("dialect-amdgcn", "0.1.0"),
    ("fe2o3-amd-target", "0.1.0"),
    ("fe2o3-amdgcn-model", "0.1.0"),
    ("fe2o3-amdhsa-loader", "0.1.0"),
    ("fe2o3-aql", "0.1.0"),
    ("fe2o3-artifact-transaction", "0.1.0"),
    ("fe2o3-artifacts", "0.1.0"),
    ("fe2o3-build-authority", "0.1.0"),
    ("fe2o3-compiler-ffi", "0.1.0"),
    ("fe2o3-contracts", "0.1.0"),
    ("fe2o3-drm-uapi", "0.1.0"),
    ("fe2o3-host-api", "0.1.0"),
    ("fe2o3-hsaco", "0.1.0"),
    ("fe2o3-hsaco-finalize", "0.1.0"),
    ("fe2o3-kernel-descriptor", "0.1.0"),
    ("fe2o3-kernel-ir", "0.1.0"),
    ("fe2o3-kfd", "0.1.0"),
    ("fe2o3-kfd-uapi", "0.1.0"),
    ("fe2o3-llvm-handoff", "0.1.0"),
    ("fe2o3-llvm-text", "0.1.0"),
    ("fe2o3-llvm-worker-handoff", "0.1.0"),
    ("fe2o3-runtime-model", "0.1.0"),
    ("fe2o3-rustc-front", "0.1.0"),
    ("fe2o3-service-host", "0.1.0"),
    ("fe2o3-service-model", "0.1.0"),
    ("fe2o3-verifier", "0.1.0"),
    ("reserved-fe2o3-symbols", "0.1.0"),
)
FE2O3_DEPENDENCY_TOPOLOGY = (
    ("ferric-build", "dependencies", "fe2o3-amdhsa-loader"),
    ("ferric-build", "dependencies", "fe2o3-artifact-transaction"),
    ("ferric-build", "dependencies", "fe2o3-compiler-ffi"),
    ("ferric-build", "dependencies", "fe2o3-hsaco-finalize"),
    ("ferric-build", "dependencies", "fe2o3-llvm-worker-handoff"),
    ("ferric-engine", "dependencies", "fe2o3-amdhsa-loader"),
    ("ferric-engine", "dependencies", "fe2o3-aql"),
    ("ferric-engine", "dependencies", "fe2o3-kfd"),
    ("ferric-engine", "dependencies", "fe2o3-service-host"),
    ("ferric-engine", "dev-dependencies", "fe2o3-hsaco"),
    ("ferric-qwen-kernels", "dependencies", "fe2o3-amdhsa-loader"),
    ("ferric-qwen-kernels", "dependencies", "fe2o3-artifact-transaction"),
    ("ferric-qwen-kernels", "dependencies", "fe2o3-compiler-ffi"),
    ("ferric-qwen-kernels", "dependencies", "fe2o3-hsaco"),
    ("ferric-qwen-kernels", "dependencies", "fe2o3-hsaco-finalize"),
    ("ferric-qwen-kernels", "dependencies", "fe2o3-llvm-handoff"),
    ("ferric-qwen-kernels", "dependencies", "fe2o3-llvm-text"),
    ("ferric-qwen-kernels", "dependencies", "fe2o3-llvm-worker-handoff"),
    ("ferric-qwen-kernels", "dependencies", "reserved-fe2o3-symbols"),
)
TCB = (
    ("tcb.compiler", "Compiler"),
    ("tcb.hardware", "Hardware"),
    ("tcb.runtime", "Runtime"),
)
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")

ARTIFACT_KINDS = {
    "artifact-identity": "ArtifactIdentityReport",
    "canonical-structure-check": "CheckerTranscript",
    "external-contract": "ContractDocument",
    "fe2o3-contract": "ContractDocument",
    "hardware-test": "HardwareTranscript",
    "independent-validator": "ValidatorTranscript",
    "negative-mutation": "MutationTranscript",
    "performance-gate": "PerformanceReport",
    "unsupported-rationale": "UnsupportedRationale",
    "verus-theorem": "TheoremTranscript",
}
REPORT_SUFFIXES = {
    "artifact-identity": "artifact-identity.json",
    "canonical-structure-check": "canonical-structure.json",
    "external-contract": "external-contract.json",
    "fe2o3-contract": "fe2o3-contract.json",
    "hardware-test": "hardware-transcript.json",
    "independent-validator": "independent-validator.json",
    "performance-gate": "performance-report.json",
    "unsupported-rationale": "unsupported-rationale.json",
}
FOUNDATION_FILES = {
    "negative-mutation": (
        "proofs/m1/negative/REQUIRED_FOUNDATIONS",
        "mutation=",
        11,
        "proofs/m1/negative/run-same-source.sh",
    ),
    "verus-theorem": (
        "proofs/m1/theorem/REQUIRED_FOUNDATIONS",
        "theorem=",
        8,
        "proofs/m1/theorem/run-same-source.sh",
    ),
}
MISSING_ROLES = {
    "canonical-structure-check": "external-canonical-structure-reporter",
    "external-contract": "external-contract-owner",
    "fe2o3-contract": "fe2o3-contract-owner",
    "hardware-test": "mi300x-hardware-harness",
    "independent-validator": "independent-validation-organization",
    "performance-gate": "external-performance-harness",
    "unsupported-rationale": "m1-nonclaim-artifact-producer",
}


JsonObject = dict[str, Any]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def canonical_bytes(value: JsonObject) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def canonical_digest(value: JsonObject) -> str:
    encoded = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    return digest_bytes(encoded)


def read_canonical_json(path: Path, description: str) -> JsonObject:
    def unique_object(pairs: list[tuple[str, Any]]) -> JsonObject:
        value: JsonObject = {}
        for key, item in pairs:
            if key in value:
                fail(f"{description} contains a duplicate JSON key: {key}")
            value[key] = item
        return value

    try:
        raw = path.read_bytes()
        value = json.loads(raw, object_pairs_hook=unique_object)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot read {description}: {error}")
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"{description} is not a canonical JSON object")
    return value


def run(arguments: list[str], description: str, *, cwd: Path | None = None) -> str:
    try:
        result = subprocess.run(
            arguments,
            cwd=cwd,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=120,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"{description} could not run: {error}")
    if result.returncode != 0:
        fail(f"{description} failed (status {result.returncode}):\n{result.stdout}")
    return result.stdout


def git(repo: Path, arguments: list[str], description: str) -> str:
    return run(["git", "-C", str(repo), *arguments], description).strip()


def repository_identity(repo: Path, description: str) -> tuple[str, str]:
    try:
        resolved = repo.resolve(strict=True)
    except OSError as error:
        fail(f"{description} repository is unavailable: {error}")
    if not resolved.is_dir():
        fail(f"{description} repository is not a directory")
    status = git(
        resolved,
        ["status", "--porcelain=v1", "--untracked-files=all"],
        f"inspect {description} worktree",
    )
    if status:
        fail(f"{description} repository must be an exact clean worktree")
    commit = git(
        resolved, ["rev-parse", "--verify", "HEAD"], f"resolve {description} commit"
    )
    tree = git(
        resolved,
        ["rev-parse", "--verify", "HEAD^{tree}"],
        f"resolve {description} tree",
    )
    if not GIT_ID.fullmatch(commit) or not GIT_ID.fullmatch(tree):
        fail(f"{description} Git identity is malformed")
    return commit, tree


def require_ancestor(repo: Path, base: str, description: str) -> None:
    if not GIT_ID.fullmatch(base):
        fail(f"{description} base commit is malformed")
    result = subprocess.run(
        ["git", "-C", str(repo), "merge-base", "--is-ancestor", base, "HEAD"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if result.returncode != 0:
        fail(f"{description} HEAD does not descend from its reviewed M1 base")


def literal_assignment(path: Path, name: str) -> Any:
    try:
        tree = ast.parse(path.read_bytes(), filename=str(path))
    except (OSError, SyntaxError) as error:
        fail(f"cannot parse checker-owned authority {path}: {error}")
    values: list[ast.AST] = []
    for node in tree.body:
        if (
            isinstance(node, ast.Assign)
            and len(node.targets) == 1
            and isinstance(node.targets[0], ast.Name)
            and node.targets[0].id == name
        ):
            values.append(node.value)
    if len(values) != 1:
        fail(f"checker-owned authority must define one literal {name}")
    try:
        return ast.literal_eval(values[0])
    except (ValueError, TypeError, SyntaxError) as error:
        fail(f"checker-owned authority {name} is not literal data: {error}")


def read_toml(path: Path, description: str) -> JsonObject:
    try:
        value = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot parse {description}: {error}")
    if not isinstance(value, dict):
        fail(f"{description} is not a TOML table")
    return value


def validate_fe2o3_topology(ferric: Path, workspace: JsonObject) -> list[JsonObject]:
    expected_names = set(FE2O3_DIRECT_PACKAGES)
    members = workspace.get("workspace", {}).get("members")
    if not isinstance(members, list) or not all(
        isinstance(item, str) for item in members
    ):
        fail("Ferric workspace member roster is malformed")
    actual: list[tuple[str, str, str]] = []
    for relative in members:
        manifest = read_toml(
            ferric / relative / "Cargo.toml", f"Ferric workspace manifest {relative}"
        )
        package = manifest.get("package", {})
        owner = package.get("name") if isinstance(package, dict) else None
        if not isinstance(owner, str):
            fail(f"Ferric workspace manifest has no package name: {relative}")
        scopes: list[tuple[str, Any]] = [
            (scope, manifest.get(scope, {}))
            for scope in ("dependencies", "dev-dependencies", "build-dependencies")
        ]
        targets = manifest.get("target", {})
        if isinstance(targets, dict):
            for target, table in targets.items():
                if not isinstance(table, dict):
                    continue
                scopes.extend(
                    (f"target.{target}.{scope}", table.get(scope, {}))
                    for scope in (
                        "dependencies",
                        "dev-dependencies",
                        "build-dependencies",
                    )
                )
        for scope, dependencies in scopes:
            if not isinstance(dependencies, dict):
                fail(f"Ferric dependency table is malformed: {owner}:{scope}")
            for name, declaration in dependencies.items():
                package_name = (
                    declaration.get("package")
                    if isinstance(declaration, dict)
                    else None
                )
                git_url = (
                    str(declaration.get("git", ""))
                    if isinstance(declaration, dict)
                    else ""
                )
                if (
                    name not in expected_names
                    and package_name not in expected_names
                    and "fe2o3" not in git_url.lower()
                ):
                    continue
                if name not in expected_names or declaration != {"workspace": True}:
                    fail(
                        "Ferric fe2o3 dependency edge is not an exact workspace edge: "
                        f"{owner}:{scope}:{name}"
                    )
                actual.append((owner, scope, name))
    if tuple(sorted(actual)) != tuple(sorted(FE2O3_DEPENDENCY_TOPOLOGY)):
        fail("Ferric fe2o3 dependency topology does not equal the admitted root graph")
    return [
        {"owner": owner, "scope": scope, "name": name}
        for owner, scope, name in sorted(actual)
    ]


def validate_fe2o3_pins(ferric: Path, fe2o3_commit: str) -> JsonObject:
    workspace = read_toml(ferric / "Cargo.toml", "Ferric workspace manifest")
    lock = read_toml(ferric / "Cargo.lock", "Ferric lockfile")
    workspace_table = workspace.get("workspace")
    if not isinstance(workspace_table, dict):
        fail("Ferric workspace manifest has no workspace table")
    workspace_dependencies = workspace_table.get("dependencies")
    if not isinstance(workspace_dependencies, dict):
        fail("Ferric workspace dependency table is malformed")
    expected_direct = set(FE2O3_DIRECT_PACKAGES)
    direct: list[dict[str, str]] = []
    for name, value in workspace_dependencies.items():
        git_url = str(value.get("git", "")) if isinstance(value, dict) else ""
        if name not in expected_direct and "fe2o3" not in git_url.lower():
            continue
        if (
            name not in expected_direct
            or not isinstance(value, dict)
            or set(value) != {"git", "rev"}
            or value.get("git") != FE2O3_REPOSITORY
            or value.get("rev") != fe2o3_commit
        ):
            fail(f"Ferric direct fe2o3 dependency declaration drifted: {name}")
        direct.append(
            {
                "name": name,
                "repository": FE2O3_REPOSITORY,
                "revision": fe2o3_commit,
            }
        )
    direct.sort(key=lambda record: record["name"])
    if {record["name"] for record in direct} != expected_direct:
        fail("Ferric direct fe2o3 dependency roster does not equal the admitted roster")

    expected_resolved = set(FE2O3_RESOLVED_PACKAGES)
    expected_source = f"git+{FE2O3_REPOSITORY}?rev={fe2o3_commit}#{fe2o3_commit}"
    resolved: list[dict[str, str]] = []
    for package in lock.get("package", []):
        if not isinstance(package, dict):
            fail("Ferric lockfile package record is malformed")
        name = package.get("name")
        version = package.get("version")
        source = package.get("source") if isinstance(package, dict) else None
        if not isinstance(name, str) or not isinstance(version, str):
            fail("Ferric lockfile package identity is malformed")
        identity = (name, version)
        if identity not in expected_resolved and (
            not isinstance(source, str) or "fe2o3" not in source.lower()
        ):
            continue
        if identity not in expected_resolved or source != expected_source:
            fail(f"Ferric resolved fe2o3 package declaration drifted: {name}")
        resolved.append({"name": name, "source": expected_source, "version": version})
    resolved.sort(key=lambda record: (record["name"], record["version"]))
    actual_resolved = [(record["name"], record["version"]) for record in resolved]
    if len(actual_resolved) != len(set(actual_resolved)) or set(actual_resolved) != (
        expected_resolved
    ):
        fail("Ferric resolved fe2o3 package roster does not equal the admitted roster")
    topology = validate_fe2o3_topology(ferric, workspace)
    return {
        "direct": direct,
        "direct_count": len(direct),
        "resolved": resolved,
        "resolved_count": len(resolved),
        "revision": fe2o3_commit,
        "root_dependency_count": len(topology),
        "root_dependencies": topology,
    }


def foundation_registries(ferric: Path) -> dict[str, dict[str, dict[str, list[str]]]]:
    registries: dict[str, dict[str, dict[str, list[str]]]] = {}
    with tempfile.TemporaryDirectory(prefix="ferric-m1-planner-registries-") as raw:
        temporary = Path(raw)
        for kind, (relative, _, fields, _) in FOUNDATION_FILES.items():
            checker = Path(relative).parent / "check-registry.py"
            output = temporary / kind
            run(
                [
                    sys.executable,
                    "-I",
                    str(ferric / checker),
                    str(ferric),
                    str(ferric / relative),
                    str(output),
                ],
                f"check {kind} foundation registry",
                cwd=ferric,
            )
            rows: dict[str, dict[str, list[str]]] = defaultdict(
                lambda: defaultdict(list)
            )
            try:
                lines = output.read_text(encoding="ascii").splitlines()
            except (OSError, UnicodeError) as error:
                fail(f"cannot read checked {kind} foundation rows: {error}")
            for line in lines:
                values = line.split("|")
                if len(values) != fields or not all(values):
                    fail(f"malformed checked {kind} foundation row")
                selector, property_name, path_id = values[0], values[2], values[3]
                rows[property_name][path_id].append(selector)
            registries[kind] = {
                property_name: {
                    path_id: selectors.copy() for path_id, selectors in paths.items()
                }
                for property_name, paths in rows.items()
            }
    return registries


def safe_component(value: str) -> str:
    component = value.replace("_", "-")
    if not SAFE_ID.fullmatch(component):
        fail(f"cannot derive a safe M1 planning identifier from {value!r}")
    return component


def evidence_pairs(
    record: JsonObject,
    obligation_class: str,
    profiles: dict[str, tuple[str, ...]],
    binding_classes: dict[str, tuple[str, ...]],
) -> list[tuple[str, str]]:
    return [
        (profile, kind)
        for profile in record["evidence_profiles"]
        for kind in profiles[profile]
        if obligation_class in binding_classes[kind]
    ]


def artifact_path(
    binding_id: str,
    artifact_id: str,
    evidence_kind: str,
    selectors: tuple[str, ...],
) -> str:
    if evidence_kind in FOUNDATION_FILES:
        if not selectors:
            fail(f"foundation binding has no selector: {binding_id}")
        return f"foundation-runs/{binding_id}/{selectors[0]}.result"
    return f"artifacts/{artifact_id}.{REPORT_SUFFIXES[evidence_kind]}"


def producer(evidence_kind: str, selectors: tuple[str, ...]) -> JsonObject:
    if evidence_kind == "artifact-identity":
        return {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-artifact-identity.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                "BINDING_ID",
            ],
            "role": "ferric-artifact-identity-reporter",
        }
    foundation = FOUNDATION_FILES.get(evidence_kind)
    if foundation is not None:
        return {
            "availability": "available",
            "command": [
                foundation[3],
                "FERRIC_REPO",
                "VERUS_ROOT",
                "OUTPUT_DIR",
                *selectors,
            ],
            "role": f"ferric-{evidence_kind}-runner",
        }
    return {
        "availability": "missing",
        "command": None,
        "role": MISSING_ROLES[evidence_kind],
    }


def make_slot(
    obligation_class: str,
    record: JsonObject,
    profile: str,
    evidence_kind: str,
    path_id: str,
    path_sources: dict[str, str],
    selectors: tuple[str, ...],
) -> JsonObject:
    obligation_id = record["id"] if obligation_class == "Roadmap" else record["name"]
    statement = record["title"] if obligation_class == "Roadmap" else record["boundary"]
    parts = (
        "binding",
        obligation_class.lower(),
        safe_component(obligation_id),
        safe_component(profile),
        safe_component(evidence_kind),
        safe_component(path_id),
    )
    binding_id = ".".join(parts)
    artifact_id = binding_id.replace("binding.", "artifact.", 1)
    binding = {
        "artifact_id": artifact_id,
        "evidence_kind": evidence_kind,
        "id": binding_id,
        "obligation_class": obligation_class,
        "obligation_id": obligation_id,
        "path_id": path_id,
        "profile_id": profile,
        "source_identity_id": path_sources[path_id],
        "statement_sha256": digest_bytes(statement.encode("utf-8")),
        "tcb_ids": [identifier for identifier, _ in TCB],
    }
    binding["binding_sha256"] = canonical_digest(binding)
    expected_artifact = {
        "id": artifact_id,
        "kind": ARTIFACT_KINDS[evidence_kind],
        "path": artifact_path(binding_id, artifact_id, evidence_kind, selectors),
    }
    return {
        "binding": binding,
        "expected_artifact": expected_artifact,
        "foundation_selectors": list(selectors),
        "producer": producer(evidence_kind, selectors),
        "state": "missing",
    }


def renumber_slots(slots: list[JsonObject]) -> list[JsonObject]:
    for ordinal, slot in enumerate(slots):
        binding_id = f"binding.{ordinal:05d}"
        artifact_id = f"artifact.binding.{ordinal:05d}"
        binding = slot["binding"]
        binding["id"] = binding_id
        binding["artifact_id"] = artifact_id
        binding["binding_sha256"] = canonical_digest(
            {key: value for key, value in binding.items() if key != "binding_sha256"}
        )
        evidence_kind = binding["evidence_kind"]
        selectors = tuple(slot["foundation_selectors"])
        slot["expected_artifact"] = {
            "id": artifact_id,
            "kind": ARTIFACT_KINDS[evidence_kind],
            "path": artifact_path(binding_id, artifact_id, evidence_kind, selectors),
        }
        if evidence_kind in FOUNDATION_FILES:
            slot["expected_artifact"]["path"] = (
                f"foundation-runs/{artifact_id}/{selectors[0]}.result"
            )
            slot["producer"]["command"][3] = f"foundation-runs/{artifact_id}"
        elif evidence_kind == "artifact-identity":
            slot["producer"]["command"][-1] = binding_id
    return slots


def allocate_obligation(
    obligation_class: str,
    record: JsonObject,
    profiles: dict[str, tuple[str, ...]],
    binding_classes: dict[str, tuple[str, ...]],
    path_sources: dict[str, str],
    registries: dict[str, dict[str, dict[str, list[str]]]],
) -> list[JsonObject]:
    obligation_id = record["id"] if obligation_class == "Roadmap" else record["name"]
    paths = tuple(record["path_obligations"])
    pairs = evidence_pairs(record, obligation_class, profiles, binding_classes)
    if not pairs or not paths:
        fail(f"M1 planning obligation has no bindings or paths: {obligation_id}")

    assigned: list[tuple[str, str, str, tuple[str, ...]]] = []
    used_triplets: set[tuple[str, str, str]] = set()
    covered: set[str] = set()
    flexible: list[tuple[str, str]] = []
    foundation_cursors: dict[str, int] = defaultdict(int)

    for profile, kind in pairs:
        if kind == "unsupported-rationale":
            for path_id in paths:
                assigned.append((profile, kind, path_id, ()))
                used_triplets.add((profile, kind, path_id))
                covered.add(path_id)
            continue
        candidates = registries.get(kind, {}).get(obligation_id)
        if candidates is None:
            flexible.append((profile, kind))
            continue
        admissible = [path_id for path_id in paths if path_id in candidates]
        if not admissible:
            fail(
                f"{kind} has no foundation path for Assurance:{obligation_id}:{profile}"
            )
        path_id = admissible[foundation_cursors[kind] % len(admissible)]
        foundation_cursors[kind] += 1
        selectors = tuple(candidates[path_id])
        assigned.append((profile, kind, path_id, selectors))
        used_triplets.add((profile, kind, path_id))
        covered.add(path_id)

    path_cursor = 0
    for profile, kind in flexible:
        uncovered = next(
            (candidate for candidate in paths if candidate not in covered), None
        )
        if uncovered is not None:
            path_id = uncovered
        else:
            path_id = paths[path_cursor % len(paths)]
            path_cursor += 1
        assigned.append((profile, kind, path_id, ()))
        used_triplets.add((profile, kind, path_id))
        covered.add(path_id)

    for path_id in paths:
        if path_id in covered:
            continue
        repeated = next(
            (
                (profile, kind)
                for profile, kind in flexible
                if (profile, kind, path_id) not in used_triplets
            ),
            None,
        )
        if repeated is None:
            fail(
                f"cannot complete M1 path coverage for {obligation_class}:{obligation_id}"
            )
        profile, kind = repeated
        assigned.append((profile, kind, path_id, ()))
        used_triplets.add((profile, kind, path_id))
        covered.add(path_id)

    duplicate_counts: dict[tuple[str, str, str], int] = defaultdict(int)
    slots: list[JsonObject] = []
    for profile, kind, path_id, selectors in assigned:
        key = (profile, kind, path_id)
        duplicate_counts[key] += 1
        ordinal = duplicate_counts[key] - 1
        if ordinal:
            fail(f"allocator produced a duplicate profile-kind-path triple: {key}")
        slots.append(
            make_slot(
                obligation_class,
                record,
                profile,
                kind,
                path_id,
                path_sources,
                selectors,
            )
        )
    return slots


def allocate_bindings(ferric: Path, requirements: JsonObject) -> list[JsonObject]:
    profiles = {
        record["id"]: tuple(record["kinds"])
        for record in requirements["evidence_profiles"]
    }
    binding_classes = {
        record["kind"]: tuple(record["classes"])
        for record in requirements["evidence_kind_binding_classes"]
    }
    path_sources = {
        record["id"]: f"source.{record['repository']}"
        for record in requirements["path_obligations"]
    }
    registries = foundation_registries(ferric)
    slots: list[JsonObject] = []
    for record in requirements["roadmap_requirements"]:
        slots.extend(
            allocate_obligation(
                "Roadmap",
                record,
                profiles,
                binding_classes,
                path_sources,
                registries,
            )
        )
    for record in requirements["assurance_properties"]:
        slots.extend(
            allocate_obligation(
                "Assurance",
                record,
                profiles,
                binding_classes,
                path_sources,
                registries,
            )
        )
    renumber_slots(slots)
    identifiers = [slot["binding"]["id"] for slot in slots]
    artifact_ids = [slot["expected_artifact"]["id"] for slot in slots]
    artifact_paths = [slot["expected_artifact"]["path"] for slot in slots]
    if (
        len(identifiers) != len(set(identifiers))
        or len(artifact_ids) != len(set(artifact_ids))
        or len(artifact_paths) != len(set(artifact_paths))
    ):
        fail("M1 planner produced a reused binding artifact identity or path")
    return slots


def allocation_tsv(slots: list[JsonObject]) -> bytes:
    return "".join(
        "|".join(
            (
                slot["binding"]["obligation_class"],
                slot["binding"]["obligation_id"],
                slot["binding"]["profile_id"],
                slot["binding"]["evidence_kind"],
                slot["binding"]["path_id"],
            )
        )
        + "\n"
        for slot in slots
    ).encode("ascii")


def obligation_slots(
    requirements: JsonObject, slots: list[JsonObject]
) -> list[JsonObject]:
    grouped: dict[tuple[str, str], list[JsonObject]] = defaultdict(list)
    for slot in slots:
        binding = slot["binding"]
        grouped[(binding["obligation_class"], binding["obligation_id"])].append(binding)
    records: list[JsonObject] = []
    specs = [
        ("Roadmap", record["id"], record)
        for record in requirements["roadmap_requirements"]
    ] + [
        ("Assurance", record["name"], record)
        for record in requirements["assurance_properties"]
    ]
    for obligation_class, identifier, record in specs:
        bindings = grouped[(obligation_class, identifier)]
        artifacts_by_kind: dict[str, list[str]] = defaultdict(list)
        for binding in bindings:
            artifacts_by_kind[binding["evidence_kind"]].append(binding["artifact_id"])
        records.append(
            {
                "assurance_dependency_ids": (
                    record["assurance_properties"]
                    if obligation_class == "Roadmap"
                    else []
                ),
                "binding_ids": [binding["id"] for binding in bindings],
                "id": identifier,
                "obligation_class": obligation_class,
                "path_ids": record["path_obligations"],
                "required_artifact_ids": {
                    "mutation": sorted(artifacts_by_kind["negative-mutation"]),
                    "proof": sorted(artifacts_by_kind["verus-theorem"]),
                    "qualification_receipt": (
                        "artifact.qualification.m1"
                        if obligation_class == "Roadmap"
                        else None
                    ),
                    "rationale": sorted(artifacts_by_kind["unsupported-rationale"]),
                    "validator": sorted(artifacts_by_kind["independent-validator"]),
                },
                "required_status": (
                    "Closed"
                    if obligation_class == "Roadmap"
                    else record["required_status_at_closure"]
                ),
                "statement_sha256": bindings[0]["statement_sha256"],
                "tcb_ids": [identifier for identifier, _ in TCB],
            }
        )
    return records


def validate_allocation(requirements: JsonObject, slots: list[JsonObject]) -> None:
    profiles = {
        record["id"]: tuple(record["kinds"])
        for record in requirements["evidence_profiles"]
    }
    binding_classes = {
        record["kind"]: tuple(record["classes"])
        for record in requirements["evidence_kind_binding_classes"]
    }
    grouped: dict[tuple[str, str], list[JsonObject]] = defaultdict(list)
    for slot in slots:
        binding = slot["binding"]
        grouped[(binding["obligation_class"], binding["obligation_id"])].append(slot)
    specs = [
        ("Roadmap", record["id"], record)
        for record in requirements["roadmap_requirements"]
    ] + [
        ("Assurance", record["name"], record)
        for record in requirements["assurance_properties"]
    ]
    for obligation_class, obligation_id, record in specs:
        actual = grouped[(obligation_class, obligation_id)]
        pairs = {
            (slot["binding"]["profile_id"], slot["binding"]["evidence_kind"])
            for slot in actual
        }
        expected = set(
            evidence_pairs(record, obligation_class, profiles, binding_classes)
        )
        paths = {slot["binding"]["path_id"] for slot in actual}
        triplets = [
            (
                slot["binding"]["profile_id"],
                slot["binding"]["evidence_kind"],
                slot["binding"]["path_id"],
            )
            for slot in actual
        ]
        if (
            pairs != expected
            or paths != set(record["path_obligations"])
            or len(triplets) != len(set(triplets))
        ):
            fail(
                f"M1 planner allocation is incomplete: {obligation_class}:{obligation_id}"
            )


def validator_registry(ferric: Path) -> list[JsonObject]:
    raw = literal_assignment(
        ferric / "proofs/check-m1-evidence-index.py", "TRUSTED_VALIDATORS"
    )
    if not isinstance(raw, dict):
        fail("checker-owned trusted validator registry is malformed")
    records = []
    for kind in sorted(raw):
        value = raw[kind]
        if (
            not isinstance(kind, str)
            or not isinstance(value, tuple)
            or len(value) != 3
            or not all(isinstance(item, str) for item in value)
        ):
            fail("checker-owned trusted validator record is malformed")
        path, protocol, expected_digest = value
        actual_digest = digest_file(ferric / path)
        if actual_digest != expected_digest:
            fail(f"trusted M1 validator source pin drifted: {kind}")
        records.append(
            {
                "evidence_kind": kind,
                "path": path,
                "protocol": protocol,
                "source_sha256": expected_digest,
            }
        )
    return records


def validate_paths(
    requirements: JsonObject, repositories: dict[str, Path]
) -> list[JsonObject]:
    tracked = {
        name: set(
            git(
                repo, ["ls-tree", "-r", "--name-only", "HEAD"], f"list {name} tree"
            ).splitlines()
        )
        for name, repo in repositories.items()
    }
    records: list[JsonObject] = []
    for record in requirements["path_obligations"]:
        repository = record["repository"]
        relative = record["path"]
        if relative not in tracked[repository]:
            fail(f"M1 path is absent from exact {repository} tree: {relative}")
        path = repositories[repository] / relative
        try:
            metadata = path.lstat()
        except OSError as error:
            fail(f"M1 path is unavailable: {relative}: {error}")
        if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
            fail(f"M1 path is not a regular nonsymlink file: {relative}")
        records.append(
            {
                "availability": record["availability"],
                "id": record["id"],
                "path": relative,
                "repository": repository,
                "source_identity_id": f"source.{repository}",
            }
        )
    return records


def ensure_external_output(output: Path, repositories: tuple[Path, ...]) -> Path:
    if output.exists() or output.is_symlink():
        fail(f"M1 planning output already exists: {output}")
    try:
        parent = output.parent.resolve(strict=True)
    except OSError as error:
        fail(f"M1 planning output parent is unavailable: {error}")
    candidate = parent / output.name
    for repository in repositories:
        try:
            candidate.relative_to(repository.resolve(strict=True))
        except ValueError:
            continue
        fail("M1 planning output must be external to both source repositories")
    os.mkdir(candidate, 0o700)
    for name in ("source-closures", "transcripts"):
        os.mkdir(candidate / name, 0o700)
    return candidate


def write_new(path: Path, value: bytes) -> None:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    descriptor = os.open(path, flags, 0o600)
    try:
        with os.fdopen(descriptor, "wb", closefd=False) as output:
            output.write(value)
            output.flush()
            os.fsync(output.fileno())
    finally:
        os.close(descriptor)


def sync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def source_closure(
    ferric: Path,
    repository: Path,
    repository_name: str,
    output: Path,
    before: tuple[str, str],
) -> JsonObject:
    relative = f"source-closures/source.{repository_name}.records"
    closure = output / relative
    producer_path = ferric / "proofs/m1/evidence/measure-source-closure.py"
    transcript = run(
        [sys.executable, "-I", str(producer_path), str(repository), str(closure)],
        f"measure exact {repository_name} source closure",
        cwd=ferric,
    )
    os.chmod(closure, 0o600)
    after = repository_identity(repository, repository_name)
    if after != before:
        fail(f"{repository_name} source identity changed during closure measurement")
    write_new(
        output / "transcripts" / f"{repository_name}-source-closure.txt",
        transcript.encode("ascii"),
    )
    raw = closure.read_bytes()
    if not raw or not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail(f"{repository_name} source closure is not canonical")
    return {
        "artifact": {
            "id": f"artifact.source.{repository_name}",
            "kind": "SourceClosure",
            "path": relative,
            "sha256": digest_bytes(raw),
            "size_bytes": len(raw),
        },
        "file_count": len(raw.splitlines()),
        "producer": {
            "command": [
                "python3",
                "-I",
                "proofs/m1/evidence/measure-source-closure.py",
                repository_name.upper() + "_REPO",
                relative,
            ],
            "source_sha256": digest_file(producer_path),
        },
    }


def revalidate_source_closure(
    ferric: Path, repository: Path, expected: Path, repository_name: str
) -> None:
    with tempfile.TemporaryDirectory(
        prefix=f"ferric-m1-planner-{repository_name}-closure-"
    ) as raw:
        candidate = Path(raw) / "closure.records"
        run(
            [
                sys.executable,
                "-I",
                str(ferric / "proofs/m1/evidence/measure-source-closure.py"),
                str(repository),
                str(candidate),
            ],
            f"revalidate exact {repository_name} source closure",
            cwd=ferric,
        )
        if candidate.read_bytes() != expected.read_bytes():
            fail(f"{repository_name} source closure changed during planning")


def global_work_items() -> list[JsonObject]:
    items: list[JsonObject] = []
    for identifier, kind in TCB:
        artifact_id = f"artifact.{identifier}"
        items.append(
            {
                "expected_artifact": {
                    "id": artifact_id,
                    "kind": "TcbReport",
                    "path": f"artifacts/{artifact_id}.tcb-report.json",
                },
                "id": f"work.{identifier}",
                "producer": {
                    "availability": "available",
                    "command": [
                        "python3",
                        "-I",
                        "proofs/m1-qualification/produce-tcb-report.py",
                        "FERRIC_REPO",
                        "FE2O3_REPO",
                        "PLAN_DIR",
                        identifier,
                    ],
                    "role": f"ferric-{kind.lower()}-tcb-reporter",
                },
                "state": "missing",
                "subject": f"tcb:{identifier}",
            }
        )
    artifact_id = "artifact.qualification.m1"
    items.append(
        {
            "expected_artifact": {
                "id": artifact_id,
                "kind": "QualificationReceipt",
                "path": f"artifacts/{artifact_id}.qualification-receipt.json",
            },
            "id": "work.qualification.m1",
            "producer": {
                "availability": "missing",
                "command": None,
                "role": "external-m1-qualification-receipt-producer",
            },
            "state": "blocked-on-all-validated-evidence",
            "subject": "qualification:M1",
        }
    )
    return items


def prepare(ferric_argument: str, fe2o3_argument: str, output_argument: str) -> None:
    ferric = Path(ferric_argument).resolve(strict=True)
    fe2o3 = Path(fe2o3_argument).resolve(strict=True)
    requirements_path = ferric / "proofs/M1_REQUIREMENTS.json"
    requirements = read_canonical_json(requirements_path, "M1 requirements manifest")

    requirements_check = run(
        [
            sys.executable,
            "-I",
            str(ferric / "proofs/check-m1-requirements.py"),
            str(ferric),
        ],
        "M1 requirements preflight",
        cwd=ferric,
    )
    infrastructure_check = run(
        [
            sys.executable,
            "-I",
            str(ferric / "proofs/m1-evidence-index/check-infrastructure.py"),
            str(ferric),
        ],
        "M1 evidence infrastructure preflight",
        cwd=ferric,
    )

    ferric_identity = repository_identity(ferric, "ferric")
    fe2o3_identity = repository_identity(fe2o3, "fe2o3")
    ferric_base = literal_assignment(
        ferric / "proofs/check-m1-evidence-index.py", "FERRIC_REQUIREMENTS_BASE_COMMIT"
    )
    if not isinstance(ferric_base, str):
        fail("checker-owned Ferric M1 base commit is malformed")
    require_ancestor(ferric, ferric_base, "Ferric")
    require_ancestor(fe2o3, requirements["m1_upstream_base_commit"], "fe2o3")
    pins = validate_fe2o3_pins(ferric, fe2o3_identity[0])
    repositories = {"ferric": ferric, "fe2o3": fe2o3}
    path_resolutions = validate_paths(requirements, repositories)
    checked_registries = foundation_registries(ferric)
    slots = allocate_bindings(ferric, requirements)
    validate_allocation(requirements, slots)
    roadmap_count = sum(
        slot["binding"]["obligation_class"] == "Roadmap" for slot in slots
    )
    assurance_count = len(slots) - roadmap_count
    if (roadmap_count, assurance_count, len(slots)) != (168, 186, 354):
        fail(
            "M1 binding allocation count drifted "
            f"(roadmap={roadmap_count}, assurance={assurance_count}, total={len(slots)})"
        )
    allocation_sha256 = digest_bytes(allocation_tsv(slots))
    expected_allocation = (
        "948ad3023df7ad4b1313ed865b54464f63b6bad9406f1510c85e60f9db055bd6"
    )
    if allocation_sha256 != expected_allocation:
        fail(
            "M1 binding allocation identity drifted "
            f"(expected={expected_allocation}, actual={allocation_sha256})"
        )

    output = ensure_external_output(Path(output_argument), (ferric, fe2o3))
    write_new(
        output / "transcripts" / "requirements-preflight.txt",
        requirements_check.encode("ascii"),
    )
    write_new(
        output / "transcripts" / "infrastructure-preflight.txt",
        infrastructure_check.encode("ascii"),
    )
    closures = {
        "ferric": source_closure(ferric, ferric, "ferric", output, ferric_identity),
        "fe2o3": source_closure(ferric, fe2o3, "fe2o3", output, fe2o3_identity),
    }
    source_records = []
    for identifier in SOURCE_IDS:
        repository = identifier.removeprefix("source.")
        commit, tree = ferric_identity if repository == "ferric" else fe2o3_identity
        base = (
            ferric_base
            if repository == "ferric"
            else requirements["m1_upstream_base_commit"]
        )
        closure = closures[repository]
        source_records.append(
            {
                "base_commit": base,
                "commit": commit,
                "id": identifier,
                "repository": repository,
                "source_closure_artifact_id": closure["artifact"]["id"],
                "source_closure_sha256": closure["artifact"]["sha256"],
                "tree": tree,
            }
        )

    validators = validator_registry(ferric)
    if (
        read_canonical_json(requirements_path, "M1 requirements manifest")
        != requirements
    ):
        fail("M1 requirements changed during planning")
    if foundation_registries(ferric) != checked_registries:
        fail("checked M1 foundation registries changed during planning")
    obligations = obligation_slots(requirements, slots)
    if len(obligations) != 50:
        fail("M1 obligation assembly roster is incomplete")
    plan = {
        "authority": AUTHORITY,
        "binding_slots": slots,
        "counts": {
            "assurance_binding_slots": assurance_count,
            "binding_slots": len(slots),
            "obligation_slots": len(obligations),
            "path_resolutions": len(path_resolutions),
            "roadmap_binding_slots": roadmap_count,
            "source_closures": len(closures),
            "trusted_validators": len(validators),
        },
        "fe2o3_pins": pins,
        "finalization": {
            "evidence_index_output": "forbidden-while-work-queue-is-incomplete",
            "qualification_receipt_output": "forbidden-while-work-queue-is-incomplete",
            "required_validator": "proofs/check-m1-evidence-index.py",
        },
        "format": PLAN_FORMAT,
        "nonclaim": NONCLAIM,
        "obligation_slots": obligations,
        "path_resolutions": path_resolutions,
        "planner_sha256": digest_file(Path(__file__).resolve()),
        "requirements": {
            "format": requirements["format"],
            "path": "proofs/M1_REQUIREMENTS.json",
            "sha256": digest_file(requirements_path),
        },
        "allocation_sha256": allocation_sha256,
        "source_closures": [closures["fe2o3"], closures["ferric"]],
        "sources": source_records,
        "target": TARGET,
        "trusted_validators": validators,
    }
    plan_bytes = canonical_bytes(plan)
    plan_sha256 = digest_bytes(plan_bytes)

    binding_work = [
        {
            "expected_artifact": slot["expected_artifact"],
            "id": slot["binding"]["id"].replace("binding.", "work.", 1),
            "producer": slot["producer"],
            "state": "missing",
            "subject": f"binding:{slot['binding']['id']}",
        }
        for slot in slots
    ]
    work = sorted(binding_work + global_work_items(), key=lambda item: item["id"])
    unavailable = sum(item["producer"]["availability"] == "missing" for item in work)
    queue = {
        "authority": AUTHORITY,
        "counts": {
            "available_producer_items": len(work) - unavailable,
            "missing_items": len(work),
            "missing_producer_items": unavailable,
        },
        "format": WORK_FORMAT,
        "items": work,
        "nonclaim": NONCLAIM,
        "plan_path": "plan.json",
        "plan_sha256": plan_sha256,
        "status": "INCOMPLETE",
    }
    write_new(output / "missing-work.json", canonical_bytes(queue))
    repository_identity(ferric, "ferric")
    repository_identity(fe2o3, "fe2o3")
    if (
        read_canonical_json(requirements_path, "M1 requirements manifest")
        != requirements
    ):
        fail("M1 requirements changed before planning publication")
    if foundation_registries(ferric) != checked_registries:
        fail("checked M1 foundation registries changed before planning publication")
    revalidate_source_closure(
        ferric,
        fe2o3,
        output / "source-closures/source.fe2o3.records",
        "fe2o3",
    )
    revalidate_source_closure(
        ferric,
        ferric,
        output / "source-closures/source.ferric.records",
        "ferric",
    )
    repository_identity(ferric, "ferric")
    repository_identity(fe2o3, "fe2o3")
    if any(
        (output / name).exists() for name in ("evidence-index.json", "receipt.json")
    ):
        fail("planning-only M1 command created a forbidden closure output")
    sync_directory(output / "source-closures")
    sync_directory(output / "transcripts")
    write_new(output / "plan.json", plan_bytes)
    sync_directory(output)
    print(
        "PASS: prepared external M1 evidence plan "
        f"(354 bindings, 358 missing work items, plan_sha256={plan_sha256})"
    )


def main() -> None:
    forbidden = {
        "--emit-index",
        "--emit-receipt",
        "assemble",
        "finalize",
        "index",
        "receipt",
    }
    if any(argument in forbidden for argument in sys.argv[1:]):
        fail(
            "this planning-only slice cannot emit an M1 evidence index or receipt; "
            "all external artifacts and trusted-validator gates must exist first"
        )
    if len(sys.argv) != 4:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_REPO NEW_OUTPUT_DIR")
    prepare(sys.argv[1], sys.argv[2], sys.argv[3])


if __name__ == "__main__":
    main()
