#!/usr/bin/env python3
"""Validate an external, identity-bound M1 closure evidence index."""

from __future__ import annotations

import hashlib
import json
import os
import re
import stat
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Callable, NoReturn


FORMAT = "ferric.m1-evidence-index.v1"
FERRIC_REQUIREMENTS_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
ROADMAP_STATUS = "Closed"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_PATH = re.compile(r"[A-Za-z0-9_./-]+\Z")

TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {
    "source.fe2o3": "fe2o3",
    "source.ferric": "ferric",
}

EVIDENCE_ARTIFACT_KINDS = {
    "artifact-identity": "ArtifactIdentityReport",
    "canonical-structure-check": "CheckerTranscript",
    "external-contract": "ContractDocument",
    "fe2o3-contract": "ContractDocument",
    "hardware-test": "HardwareTranscript",
    "independent-validator": "ValidatorTranscript",
    "negative-mutation": "MutationTranscript",
    "performance-gate": "PerformanceReport",
    "tcb-report": "TcbReport",
    "unsupported-rationale": "UnsupportedRationale",
    "verus-theorem": "TheoremTranscript",
}
ARTIFACT_KINDS = set(EVIDENCE_ARTIFACT_KINDS.values()) | {
    "QualificationReceipt",
    "SourceClosure",
}
SOURCE_EXCLUDED_DIRECTORIES = {".git", ".ruff_cache", "__pycache__", "target"}
SOURCE_EXCLUDED_SUFFIXES = {".pyc", ".receipt"}

# Validator paths, protocols, and reviewed source identities are checker-owned,
# so an evidence index cannot select or substitute an executable. A None source
# identity denotes a validator that remains a RequiredFuture obligation.
TRUSTED_VALIDATORS = {
    "artifact-identity": (
        "proofs/m1/evidence/validate-artifact-identity.py",
        "ferric.m1-validator.artifact-identity.v1",
        "a72f2e33e92ac9064302194a341f146480950ee090b0ff8de037c7fc71919092",
    ),
    "canonical-structure-check": (
        "proofs/m1/evidence/validate-canonical-structure.py",
        "ferric.m1-validator.canonical-structure.v1",
        "f8760f0edd21d996c0f59fa2b5aa16a1ab07ecd67b4352e226214c47ef9fe288",
    ),
    "external-contract": (
        "proofs/m1/evidence/validate-external-contract.py",
        "ferric.m1-validator.external-contract.v1",
        "21ddc8a9f00e90ef2255c27fb562279fcd899814fa5ea06824bc4cd9b250c57e",
    ),
    "fe2o3-contract": (
        "proofs/m1/evidence/validate-fe2o3-contract.py",
        "ferric.m1-validator.fe2o3-contract.v1",
        "8e493c1146c4a8e6b2b9992b42e33aabf63b0a6dcd5af87897324cfa9433e024",
    ),
    "hardware-test": (
        "proofs/m1/evidence/validate-hardware-transcript.py",
        "ferric.m1-validator.hardware-transcript.v1",
        "8a1e06fab53e38f1d48a8c26f132204a169c54ce56cf4bd283695cdc38b6e21f",
    ),
    "independent-validator": (
        "proofs/m1/evidence/validate-independent-validator.py",
        "ferric.m1-validator.independent-validator.v1",
        "d6188b3a1ff8f637b745fe4100fdd234ebb2c59f86badfddb8c59d10d71b1782",
    ),
    "negative-mutation": (
        "proofs/m1/evidence/validate-negative-mutation.py",
        "ferric.m1-validator.negative-mutation.v1",
        "b4ee8e7c362f28506a87a4c7620249950c61a3eb34fbddd963961f45a78092c2",
    ),
    "performance-gate": (
        "proofs/m1/evidence/validate-performance-report.py",
        "ferric.m1-validator.performance-report.v1",
        "dac25a582fcb6786d4aeabbfa31ab0fbd00cf962ee9313074ed732894d9feb65",
    ),
    "qualification-receipt": (
        "proofs/m1/evidence/validate-qualification-receipt.py",
        "ferric.m1-validator.qualification-receipt.v1",
        "266d14ed4cff8dffd7dc0e383f49df835cd0bebef4a1f1682b557a059f518702",
    ),
    "tcb-report": (
        "proofs/m1/evidence/validate-tcb-report.py",
        "ferric.m1-validator.tcb-report.v1",
        "2fe6de0da707b36d46d4e68c1cc3657c14fdf1225b0491acd8baee696f68460f",
    ),
    "unsupported-rationale": (
        "proofs/m1/evidence/validate-unsupported-rationale.py",
        "ferric.m1-validator.unsupported-rationale.v1",
        "32d008741e317446e1fda1f5fd021efa13f0ea91b6da3c4b3d5635aca61d560e",
    ),
    "verus-theorem": (
        "proofs/m1/evidence/validate-verus-theorem.py",
        "ferric.m1-validator.verus-theorem.v1",
        "389fd5beac597c0177ae4a02d57dfecaa314bd4dcca55e0ac09db4d086738d0d",
    ),
}

TestValidator = Callable[[str, dict[str, Any]], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def regular_file(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {path}: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"{description} must be a regular file: {path}")


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    hasher = hashlib.sha256()
    try:
        with path.open("rb") as source:
            for block in iter(lambda: source.read(1024 * 1024), b""):
                hasher.update(block)
    except OSError as error:
        fail(f"cannot hash {path}: {error}")
    return hasher.hexdigest()


def canonical_digest(value: dict[str, Any]) -> str:
    encoded = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    return digest_bytes(encoded)


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def load_canonical_json(path: Path, description: str) -> dict[str, Any]:
    regular_file(path, description)
    try:
        source = path.read_text(encoding="utf-8")
        value = json.loads(source, object_pairs_hook=unique_object)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse {description}: {error}")
    if not isinstance(value, dict):
        fail(f"{description} must be an object")
    canonical = json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    if source != canonical:
        fail(f"{description} is not canonical JSON")
    return value


def exact_keys(value: dict[str, Any], expected: set[str], description: str) -> None:
    if set(value) != expected:
        fail(
            f"{description} has unexpected keys "
            f"(missing={sorted(expected - set(value))}, extra={sorted(set(value) - expected)})"
        )


def string_list(
    value: Any, description: str, *, allow_empty: bool = False
) -> tuple[str, ...]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        fail(f"{description} must be a string array")
    if not allow_empty and not value:
        fail(f"{description} must not be empty")
    if len(value) != len(set(value)):
        fail(f"{description} contains a duplicate reference")
    return tuple(value)


def safe_relative(value: str, description: str) -> Path:
    path = Path(value)
    if not SAFE_PATH.fullmatch(value) or path.is_absolute() or ".." in path.parts:
        fail(f"unsafe {description}: {value!r}")
    return path


def require_sha256(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or not SHA256.fullmatch(value)
        or len(set(value)) == 1
    ):
        fail(f"invalid {description}")
    return value


def require_git_id(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or not GIT_ID.fullmatch(value)
        or len(set(value)) == 1
    ):
        fail(f"invalid {description}")
    return value


def validate_requirements(repo: Path) -> None:
    checker = repo / "proofs/check-m1-requirements.py"
    regular_file(checker, "M1 requirements checker")
    result = subprocess.run(
        [sys.executable, "-I", str(checker), str(repo)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"M1 requirements are not valid:\n{result.stdout}")


def git_identity(repo: Path) -> tuple[str, str]:
    values: list[str] = []
    for revision in ("HEAD^{commit}", "HEAD^{tree}"):
        result = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", revision],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env={"PATH": os.environ.get("PATH", "")},
        )
        if result.returncode != 0:
            fail(
                f"source repository has no exact Git identity: {repo}: {result.stderr.strip()}"
            )
        values.append(require_git_id(result.stdout.strip(), f"Git identity for {repo}"))
    status_result = subprocess.run(
        ["git", "-C", str(repo), "status", "--porcelain", "--untracked-files=all"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if status_result.returncode != 0 or status_result.stdout:
        fail(f"source repository is not the exact clean Git tree: {repo}")
    return values[0], values[1]


def git_tree_paths(repo: Path) -> set[str]:
    result = subprocess.run(
        ["git", "-C", str(repo), "ls-tree", "-r", "--name-only", "HEAD"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"cannot enumerate exact Git tree: {repo}: {result.stderr.strip()}")
    return {
        value
        for value in result.stdout.splitlines()
        if not any(part in SOURCE_EXCLUDED_DIRECTORIES for part in Path(value).parts)
        and Path(value).suffix not in SOURCE_EXCLUDED_SUFFIXES
    }


def source_closure(repo: Path) -> tuple[bytes, set[str]]:
    records: list[str] = []
    paths: set[str] = set()
    try:
        candidates = sorted(
            repo.rglob("*"), key=lambda path: path.relative_to(repo).as_posix()
        )
        for path in candidates:
            relative = path.relative_to(repo)
            if any(part in SOURCE_EXCLUDED_DIRECTORIES for part in relative.parts):
                continue
            if path.is_symlink():
                fail(f"M1 source closure contains a symlink: {path}")
            if path.is_dir():
                continue
            if not path.is_file():
                fail(f"M1 source closure contains a special entry: {path}")
            if path.suffix in SOURCE_EXCLUDED_SUFFIXES:
                fail(f"M1 source closure contains a generated input: {path}")
            relative_name = relative.as_posix()
            mode = stat.S_IMODE(path.stat().st_mode)
            records.append(
                f"{relative_name}|{mode:o}|{path.stat().st_size}|{digest_file(path)}"
            )
            paths.add(relative_name)
    except (OSError, ValueError) as error:
        fail(f"cannot measure M1 source closure for {repo}: {error}")
    if not records:
        fail(f"M1 source closure is empty: {repo}")
    return ("\n".join(records) + "\n").encode("utf-8"), paths


def external_artifact_path(root: Path, relative: str, description: str) -> Path:
    suffix = safe_relative(relative, description)
    candidate = root / suffix
    resolved_root = root.resolve(strict=True)
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(resolved_root)
    except (OSError, ValueError) as error:
        fail(f"{description} escapes or is unavailable: {relative}: {error}")
    current = candidate
    while current != root:
        try:
            if stat.S_ISLNK(current.lstat().st_mode):
                fail(f"{description} contains a symlink: {relative}")
        except OSError as error:
            fail(f"{description} is unavailable: {relative}: {error}")
        current = current.parent
    regular_file(candidate, description)
    return candidate


def validate_artifacts(
    index_root: Path, value: Any
) -> tuple[dict[str, dict[str, Any]], dict[str, Path]]:
    if not isinstance(value, list) or not value:
        fail("M1 artifact roster must be a nonempty array")
    artifacts: dict[str, dict[str, Any]] = {}
    files: dict[str, Path] = {}
    used_paths: set[str] = set()
    for record in value:
        if not isinstance(record, dict):
            fail("M1 artifact record must be an object")
        exact_keys(
            record, {"id", "kind", "path", "sha256", "size_bytes"}, "M1 artifact"
        )
        identifier = record["id"]
        if not isinstance(identifier, str) or not SAFE_ID.fullmatch(identifier):
            fail(f"unsafe M1 artifact id: {identifier!r}")
        if identifier in artifacts:
            fail(f"duplicate M1 artifact id: {identifier}")
        if record["kind"] not in ARTIFACT_KINDS:
            fail(f"unknown M1 artifact kind: {record['kind']!r}")
        if not isinstance(record["size_bytes"], int) or record["size_bytes"] <= 0:
            fail(f"invalid M1 artifact size: {identifier}")
        expected_digest = require_sha256(
            record["sha256"], f"artifact SHA-256: {identifier}"
        )
        if record["path"] in used_paths:
            fail(f"M1 artifact path is reused: {record['path']}")
        path = external_artifact_path(
            index_root, record["path"], f"M1 artifact {identifier}"
        )
        if (
            path.stat().st_size != record["size_bytes"]
            or digest_file(path) != expected_digest
        ):
            fail(f"M1 artifact identity mismatch: {identifier}")
        artifacts[identifier] = record
        files[identifier] = path
        used_paths.add(record["path"])
    if tuple(artifacts) != tuple(sorted(artifacts)):
        fail("M1 artifact roster is not canonically ordered")
    return artifacts, files


def validate_sources(
    value: Any,
    requirements: dict[str, Any],
    repositories: dict[str, Path],
    artifacts: dict[str, dict[str, Any]],
    artifact_files: dict[str, Path],
    used_artifacts: set[str],
) -> tuple[dict[str, dict[str, Any]], dict[str, set[str]]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail(f"M1 source roster must contain exactly {len(SOURCE_IDS)} records")
    sources: dict[str, dict[str, Any]] = {}
    closure_paths: dict[str, set[str]] = {}
    expected_bases = {
        "source.fe2o3": requirements["m1_upstream_base_commit"],
        "source.ferric": FERRIC_REQUIREMENTS_BASE_COMMIT,
    }
    for record in value:
        if not isinstance(record, dict):
            fail("M1 source identity must be an object")
        exact_keys(
            record,
            {
                "base_commit",
                "commit",
                "id",
                "repository",
                "source_closure_artifact_id",
                "source_closure_sha256",
                "tree",
            },
            "M1 source identity",
        )
        identifier = record["id"]
        if identifier in sources:
            fail(f"duplicate M1 source identity: {identifier}")
        if identifier not in SOURCE_REPOSITORIES:
            fail(f"unknown M1 source identity: {identifier!r}")
        repository = SOURCE_REPOSITORIES[identifier]
        if (
            record["repository"] != repository
            or record["base_commit"] != expected_bases[identifier]
        ):
            fail(f"M1 source authority boundary drifted: {identifier}")
        commit = require_git_id(record["commit"], f"source commit: {identifier}")
        tree = require_git_id(record["tree"], f"source tree: {identifier}")
        actual_commit, actual_tree = git_identity(repositories[repository])
        if commit != actual_commit or tree != actual_tree:
            fail(f"M1 source commit or tree mismatch: {identifier}")
        artifact_id = record["source_closure_artifact_id"]
        artifact = artifacts.get(artifact_id)
        if artifact is None or artifact["kind"] != "SourceClosure":
            fail(f"M1 source closure artifact is unavailable or mistyped: {identifier}")
        closure, members = source_closure(repositories[repository])
        if members != git_tree_paths(repositories[repository]):
            fail(f"M1 source closure is not the exact committed tree: {identifier}")
        closure_digest = digest_bytes(closure)
        supplied_digest = require_sha256(
            record["source_closure_sha256"], f"source closure SHA-256: {identifier}"
        )
        if (
            artifact_files[artifact_id].read_bytes() != closure
            or artifact["sha256"] != closure_digest
            or supplied_digest != closure_digest
        ):
            fail(f"M1 source closure mismatch: {identifier}")
        sources[identifier] = record
        closure_paths[identifier] = members
        used_artifacts.add(artifact_id)
    if tuple(sources) != SOURCE_IDS:
        fail("M1 source identity roster is not canonically ordered")
    return sources, closure_paths


def validate_tcb(
    value: Any,
    artifacts: dict[str, dict[str, Any]],
    used_artifacts: set[str],
) -> dict[str, dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail(f"M1 TCB roster must contain exactly {len(TCB_IDS)} records")
    entries: dict[str, dict[str, Any]] = {}
    for record in value:
        if not isinstance(record, dict):
            fail("M1 TCB entry must be an object")
        exact_keys(
            record, {"artifact_id", "id", "identity_sha256", "kind"}, "M1 TCB entry"
        )
        identifier = record["id"]
        if identifier in entries:
            fail(f"duplicate M1 TCB entry: {identifier}")
        if TCB_KINDS.get(identifier) != record["kind"]:
            fail(f"M1 TCB kind or identity drifted: {identifier}")
        artifact_id = record["artifact_id"]
        artifact = artifacts.get(artifact_id)
        if artifact is None or artifact["kind"] != "TcbReport":
            fail(f"M1 TCB artifact is unavailable or mistyped: {identifier}")
        if (
            require_sha256(record["identity_sha256"], f"TCB identity: {identifier}")
            != artifact["sha256"]
        ):
            fail(f"M1 TCB identity does not bind its report: {identifier}")
        entries[identifier] = record
        used_artifacts.add(artifact_id)
    if tuple(entries) != TCB_IDS:
        fail("M1 TCB roster is not canonically ordered")
    return entries


def validate_path_resolutions(
    value: Any,
    requirements: dict[str, Any],
    sources: dict[str, dict[str, Any]],
    closure_paths: dict[str, set[str]],
    repositories: dict[str, Path],
) -> dict[str, dict[str, Any]]:
    expected_paths = requirements["path_obligations"]
    if not isinstance(value, list) or len(value) != len(expected_paths):
        fail(
            f"M1 path resolution roster must contain exactly {len(expected_paths)} records"
        )
    resolutions: dict[str, dict[str, Any]] = {}
    for record, expected in zip(value, expected_paths, strict=True):
        if not isinstance(record, dict):
            fail("M1 path resolution must be an object")
        exact_keys(
            record,
            {"availability", "id", "path", "repository", "source_identity_id"},
            "M1 path resolution",
        )
        identifier = record["id"]
        if identifier in resolutions:
            fail(f"duplicate M1 path resolution: {identifier}")
        source_id = record["source_identity_id"]
        expected_source = f"source.{expected['repository']}"
        if (
            identifier != expected["id"]
            or record["availability"] != expected["availability"]
            or record["repository"] != expected["repository"]
            or record["path"] != expected["path"]
            or source_id != expected_source
            or source_id not in sources
        ):
            fail(f"M1 path resolution drifted: {expected['id']}")
        if record["path"] not in closure_paths[source_id]:
            fail(f"M1 path is absent from its exact source closure: {identifier}")
        path = repositories[record["repository"]] / safe_relative(
            record["path"], f"M1 resolved path {identifier}"
        )
        regular_file(path, f"M1 resolved path {identifier}")
        resolutions[identifier] = record
    return resolutions


def obligation_specs(requirements: dict[str, Any]) -> list[dict[str, Any]]:
    specs: list[dict[str, Any]] = []
    for record in requirements["roadmap_requirements"]:
        specs.append(
            {
                "assurance_dependencies": tuple(record["assurance_properties"]),
                "class": "Roadmap",
                "id": record["id"],
                "paths": tuple(record["path_obligations"]),
                "profiles": tuple(record["evidence_profiles"]),
                "statement": record["title"],
                "status": ROADMAP_STATUS,
            }
        )
    for record in requirements["assurance_properties"]:
        specs.append(
            {
                "class": "Assurance",
                "id": record["name"],
                "paths": tuple(record["path_obligations"]),
                "profiles": tuple(record["evidence_profiles"]),
                "statement": record["boundary"],
                "status": record["required_status_at_closure"],
            }
        )
    return specs


def validate_bindings(
    value: Any,
    specs: list[dict[str, Any]],
    profiles: dict[str, tuple[str, ...]],
    resolutions: dict[str, dict[str, Any]],
    artifacts: dict[str, dict[str, Any]],
    used_artifacts: set[str],
) -> tuple[dict[str, dict[str, Any]], dict[tuple[str, str], list[str]]]:
    if not isinstance(value, list) or not value:
        fail("M1 evidence binding roster must be a nonempty array")
    spec_by_key = {(spec["class"], spec["id"]): spec for spec in specs}
    bindings: dict[str, dict[str, Any]] = {}
    grouped: dict[tuple[str, str], list[str]] = defaultdict(list)
    observed_pairs: dict[tuple[str, str], set[tuple[str, str]]] = defaultdict(set)
    observed_paths: dict[tuple[str, str], set[str]] = defaultdict(set)
    binding_artifacts: set[str] = set()
    for record in value:
        if not isinstance(record, dict):
            fail("M1 evidence binding must be an object")
        exact_keys(
            record,
            {
                "artifact_id",
                "binding_sha256",
                "evidence_kind",
                "id",
                "obligation_class",
                "obligation_id",
                "path_id",
                "profile_id",
                "source_identity_id",
                "statement_sha256",
                "tcb_ids",
            },
            "M1 evidence binding",
        )
        identifier = record["id"]
        if not isinstance(identifier, str) or not SAFE_ID.fullmatch(identifier):
            fail(f"unsafe M1 evidence binding id: {identifier!r}")
        if identifier in bindings:
            fail(f"duplicate M1 evidence binding id: {identifier}")
        key = (record["obligation_class"], record["obligation_id"])
        spec = spec_by_key.get(key)
        if spec is None:
            fail(f"M1 evidence binding names an unknown obligation: {identifier}")
        profile = record["profile_id"]
        kind = record["evidence_kind"]
        if profile not in spec["profiles"] or kind not in profiles.get(profile, ()):
            fail(f"M1 evidence binding has the wrong profile or kind: {identifier}")
        if (profile, kind) in observed_pairs[key]:
            fail(
                f"duplicate M1 profile-kind evidence binding: {key[0]}:{key[1]}:{profile}:{kind}"
            )
        path_id = record["path_id"]
        if path_id not in spec["paths"] or path_id not in resolutions:
            fail(f"M1 evidence binding has the wrong path: {identifier}")
        source_id = record["source_identity_id"]
        if source_id != resolutions[path_id]["source_identity_id"]:
            fail(f"M1 evidence binding has the wrong source identity: {identifier}")
        statement_digest = digest_bytes(spec["statement"].encode("utf-8"))
        if record["statement_sha256"] != statement_digest:
            fail(f"M1 evidence binding has the wrong statement identity: {identifier}")
        if string_list(record["tcb_ids"], f"binding {identifier} TCB") != TCB_IDS:
            fail(f"M1 evidence binding does not name the complete TCB: {identifier}")
        artifact_id = record["artifact_id"]
        artifact = artifacts.get(artifact_id)
        if artifact is None or artifact["kind"] != EVIDENCE_ARTIFACT_KINDS[kind]:
            fail(
                f"M1 evidence binding artifact is unavailable or cannot satisfy its kind: {identifier}"
            )
        if artifact_id in binding_artifacts:
            fail(
                f"M1 evidence artifact is reused across incompatible bindings: {artifact_id}"
            )
        payload = {
            name: item for name, item in record.items() if name != "binding_sha256"
        }
        if record["binding_sha256"] != canonical_digest(payload):
            fail(f"M1 evidence binding identity mismatch: {identifier}")
        bindings[identifier] = record
        grouped[key].append(identifier)
        observed_pairs[key].add((profile, kind))
        observed_paths[key].add(path_id)
        binding_artifacts.add(artifact_id)
        used_artifacts.add(artifact_id)
    if tuple(bindings) != tuple(sorted(bindings)):
        fail("M1 evidence binding roster is not canonically ordered")
    for key, spec in spec_by_key.items():
        expected_pairs = {
            (profile, kind)
            for profile in spec["profiles"]
            for kind in profiles[profile]
        }
        if observed_pairs[key] != expected_pairs:
            fail(f"M1 evidence profile-kind closure is incomplete: {key[0]}:{key[1]}")
        if observed_paths[key] != set(spec["paths"]):
            fail(f"M1 evidence path coverage is incomplete: {key[0]}:{key[1]}")
        grouped[key].sort()
    return bindings, grouped


def binding_artifacts_for_kind(
    binding_ids: list[str], bindings: dict[str, dict[str, Any]], kind: str
) -> tuple[str, ...]:
    return tuple(
        sorted(
            bindings[identifier]["artifact_id"]
            for identifier in binding_ids
            if bindings[identifier]["evidence_kind"] == kind
        )
    )


def validate_obligations(
    value: Any,
    specs: list[dict[str, Any]],
    bindings: dict[str, dict[str, Any]],
    grouped: dict[tuple[str, str], list[str]],
    artifacts: dict[str, dict[str, Any]],
    used_artifacts: set[str],
) -> str:
    if not isinstance(value, list) or len(value) != len(specs):
        fail(f"M1 closure roster must contain exactly {len(specs)} obligation records")
    seen: set[tuple[str, str]] = set()
    referenced_bindings: set[str] = set()
    receipt_ids: set[str] = set()
    for record, spec in zip(value, specs, strict=True):
        if not isinstance(record, dict):
            fail("M1 closure obligation must be an object")
        common = {
            "closure_status",
            "evidence_binding_ids",
            "id",
            "obligation_class",
            "path_resolution_ids",
            "statement_sha256",
            "tcb_ids",
        }
        status = spec["status"]
        if spec["class"] == "Roadmap":
            expected_keys = common | {"assurance_dependencies", "receipt_artifact_id"}
        elif status == "Proved":
            expected_keys = common | {"mutation_artifact_ids", "proof_artifact_ids"}
        elif status == "Validated":
            expected_keys = common | {"validator_artifact_ids", "validator_tcb_ids"}
        elif status == "Unsupported":
            expected_keys = common | {
                "nonclaim_tcb_ids",
                "rationale",
                "rationale_artifact_id",
            }
        else:
            fail(f"unknown required M1 closure status: {status}")
        exact_keys(
            record, expected_keys, f"M1 closure obligation {spec['class']}:{spec['id']}"
        )
        key = (record["obligation_class"], record["id"])
        expected_key = (spec["class"], spec["id"])
        if key in seen:
            fail(f"duplicate M1 closure obligation: {key[0]}:{key[1]}")
        if key != expected_key:
            fail(
                f"M1 closure obligation roster drifted: expected {expected_key[0]}:{expected_key[1]}"
            )
        if record["closure_status"] != status:
            fail(f"M1 closure status weakened or promoted: {key[0]}:{key[1]}")
        statement_digest = digest_bytes(spec["statement"].encode("utf-8"))
        if record["statement_sha256"] != statement_digest:
            fail(f"M1 closure statement identity mismatch: {key[0]}:{key[1]}")
        if string_list(record["tcb_ids"], f"closure {key} TCB") != TCB_IDS:
            fail(f"M1 closure does not name the complete TCB: {key[0]}:{key[1]}")
        if tuple(record["path_resolution_ids"]) != spec["paths"]:
            fail(f"M1 closure path roster drifted: {key[0]}:{key[1]}")
        evidence_ids = string_list(
            record["evidence_binding_ids"], f"closure {key} evidence bindings"
        )
        if evidence_ids != tuple(grouped[key]):
            fail(f"M1 closure evidence binding roster drifted: {key[0]}:{key[1]}")
        if referenced_bindings & set(evidence_ids):
            fail(
                f"M1 evidence binding is reused by incompatible obligations: {key[0]}:{key[1]}"
            )
        referenced_bindings.update(evidence_ids)

        if spec["class"] == "Roadmap":
            if (
                tuple(record["assurance_dependencies"])
                != spec["assurance_dependencies"]
            ):
                fail(f"M1 roadmap assurance dependencies drifted: {spec['id']}")
            receipt_id = record["receipt_artifact_id"]
            receipt = artifacts.get(receipt_id)
            if receipt is None or receipt["kind"] != "QualificationReceipt":
                fail(f"M1 roadmap receipt is unavailable or fake: {spec['id']}")
            receipt_ids.add(receipt_id)
            used_artifacts.add(receipt_id)
        elif status == "Proved":
            proof_ids = binding_artifacts_for_kind(
                grouped[key], bindings, "verus-theorem"
            )
            mutation_ids = binding_artifacts_for_kind(
                grouped[key], bindings, "negative-mutation"
            )
            if not proof_ids or tuple(record["proof_artifact_ids"]) != proof_ids:
                fail(f"M1 Proved closure has no exact theorem artifacts: {spec['id']}")
            if (
                not mutation_ids
                or tuple(record["mutation_artifact_ids"]) != mutation_ids
            ):
                fail(f"M1 Proved closure has no exact mutation artifacts: {spec['id']}")
        elif status == "Validated":
            validator_ids = binding_artifacts_for_kind(
                grouped[key], bindings, "independent-validator"
            )
            if (
                not validator_ids
                or tuple(record["validator_artifact_ids"]) != validator_ids
            ):
                fail(
                    f"M1 Validated closure has no independent validator artifacts: {spec['id']}"
                )
            if tuple(record["validator_tcb_ids"]) != TCB_IDS:
                fail(
                    f"M1 Validated closure does not expose its validator TCB: {spec['id']}"
                )
        else:
            rationale_ids = binding_artifacts_for_kind(
                grouped[key], bindings, "unsupported-rationale"
            )
            if record["rationale"] != spec["statement"]:
                fail(f"M1 Unsupported closure rationale drifted: {spec['id']}")
            if (
                len(rationale_ids) != 1
                or record["rationale_artifact_id"] != rationale_ids[0]
            ):
                fail(
                    f"M1 Unsupported closure has no exact rationale artifact: {spec['id']}"
                )
            if tuple(record["nonclaim_tcb_ids"]) != TCB_IDS:
                fail(
                    f"M1 Unsupported closure does not expose its nonclaim TCB: {spec['id']}"
                )
        seen.add(key)
    if referenced_bindings != set(bindings):
        fail("not every M1 evidence binding resolves to exactly one closure obligation")
    if len(receipt_ids) != 1:
        fail("M1 roadmap closure must use one canonical qualification receipt")
    return next(iter(receipt_ids))


def invoke_trusted_validator(
    ferric: Path,
    ferric_closure_paths: set[str],
    evidence_kind: str,
    context: dict[str, Any],
    test_only_validator: TestValidator | None,
) -> None:
    if test_only_validator is not None:
        test_only_validator(evidence_kind, context)
        return
    validator_spec = TRUSTED_VALIDATORS.get(evidence_kind)
    if validator_spec is None:
        fail(f"no trusted M1 validator is registered for {evidence_kind}")
    relative, validator_format, expected_source_digest = validator_spec
    if relative not in ferric_closure_paths:
        fail(
            f"trusted M1 validator is absent from the exact Ferric source closure: {relative}"
        )
    validator = ferric / relative
    regular_file(validator, f"trusted M1 validator for {evidence_kind}")
    if expected_source_digest is None:
        fail(f"trusted M1 validator has no pinned source identity: {relative}")
    if require_sha256(
        expected_source_digest, f"trusted validator source: {evidence_kind}"
    ) != digest_file(validator):
        fail(f"trusted M1 validator source identity mismatch: {relative}")
    payload = json.dumps(
        context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    payload_digest = digest_bytes(payload.encode("ascii"))
    artifact_digest = context["artifact"]["sha256"]
    try:
        result = subprocess.run(
            [sys.executable, "-I", str(validator), validator_format],
            check=False,
            input=payload + "\n",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=60,
            cwd=ferric,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"trusted M1 validator could not run for {evidence_kind}: {error}")
    expected = (
        f"PASS: {validator_format} artifact_sha256={artifact_digest} "
        f"context_sha256={payload_digest}\n"
    )
    if result.returncode != 0 or result.stdout != expected:
        fail(
            f"trusted M1 validator rejected {evidence_kind}: "
            f"exit={result.returncode}, output={result.stdout!r}"
        )


def validate_with_trusted_producers(
    ferric: Path,
    fe2o3: Path,
    requirements_digest: str,
    index: dict[str, Any],
    sources: dict[str, dict[str, Any]],
    closure_paths: dict[str, set[str]],
    resolutions: dict[str, dict[str, Any]],
    artifacts: dict[str, dict[str, Any]],
    artifact_files: dict[str, Path],
    tcb: dict[str, dict[str, Any]],
    bindings: dict[str, dict[str, Any]],
    receipt_artifact_id: str,
    test_only_validator: TestValidator | None,
) -> None:
    common = {
        "format": FORMAT,
        "requirements_sha256": requirements_digest,
        "sources": [sources[identifier] for identifier in SOURCE_IDS],
        "tcb": [tcb[identifier] for identifier in TCB_IDS],
    }
    for identifier, binding in bindings.items():
        artifact_id = binding["artifact_id"]
        path_id = binding["path_id"]
        context = {
            **common,
            "artifact": artifacts[artifact_id],
            "artifact_absolute_path": str(artifact_files[artifact_id]),
            "binding": binding,
            "path_resolution": resolutions[path_id],
            "subject": f"binding:{identifier}",
        }
        invoke_trusted_validator(
            ferric,
            closure_paths["source.ferric"],
            binding["evidence_kind"],
            context,
            test_only_validator,
        )
    for identifier in TCB_IDS:
        record = tcb[identifier]
        artifact_id = record["artifact_id"]
        context = {
            **common,
            "artifact": artifacts[artifact_id],
            "artifact_absolute_path": str(artifact_files[artifact_id]),
            "subject": f"tcb:{identifier}",
            "tcb_record": record,
        }
        invoke_trusted_validator(
            ferric,
            closure_paths["source.ferric"],
            "tcb-report",
            context,
            test_only_validator,
        )
    context = {
        **common,
        "artifact": artifacts[receipt_artifact_id],
        "artifact_absolute_path": str(artifact_files[receipt_artifact_id]),
        "index": index,
        "repository_absolute_paths": {
            "fe2o3": str(fe2o3),
            "ferric": str(ferric),
        },
        "subject": "qualification:M1",
    }
    invoke_trusted_validator(
        ferric,
        closure_paths["source.ferric"],
        "qualification-receipt",
        context,
        test_only_validator,
    )


def validate_evidence_index(
    ferric: Path,
    index_path: Path,
    fe2o3: Path,
    *,
    _test_only_validator: TestValidator | None = None,
) -> None:
    ferric = ferric.resolve(strict=True)
    fe2o3 = fe2o3.resolve(strict=True)
    validate_requirements(ferric)
    requirements_path = ferric / "proofs/M1_REQUIREMENTS.json"
    requirements = load_canonical_json(requirements_path, "M1 requirements manifest")
    index = load_canonical_json(index_path, "M1 evidence index")
    exact_keys(
        index,
        {
            "artifacts",
            "evidence_bindings",
            "format",
            "obligations",
            "path_resolutions",
            "requirements_sha256",
            "sources",
            "tcb",
        },
        "M1 evidence index",
    )
    if index["format"] != FORMAT:
        fail("M1 evidence index format drifted")
    requirements_digest = digest_file(requirements_path)
    if index["requirements_sha256"] != requirements_digest:
        fail("M1 evidence index does not bind the exact requirements manifest")

    index_root = index_path.resolve(strict=True).parent
    artifacts, artifact_files = validate_artifacts(index_root, index["artifacts"])
    used_artifacts: set[str] = set()
    repositories = {"ferric": ferric, "fe2o3": fe2o3}
    sources, closure_paths = validate_sources(
        index["sources"],
        requirements,
        repositories,
        artifacts,
        artifact_files,
        used_artifacts,
    )
    tcb = validate_tcb(index["tcb"], artifacts, used_artifacts)
    resolutions = validate_path_resolutions(
        index["path_resolutions"],
        requirements,
        sources,
        closure_paths,
        repositories,
    )
    profiles = {
        record["id"]: tuple(record["kinds"])
        for record in requirements["evidence_profiles"]
    }
    specs = obligation_specs(requirements)
    bindings, grouped = validate_bindings(
        index["evidence_bindings"],
        specs,
        profiles,
        resolutions,
        artifacts,
        used_artifacts,
    )
    receipt_artifact_id = validate_obligations(
        index["obligations"],
        specs,
        bindings,
        grouped,
        artifacts,
        used_artifacts,
    )
    if used_artifacts != set(artifacts):
        unused = sorted(set(artifacts) - used_artifacts)
        fail(f"M1 evidence index contains unreferenced artifacts: {unused}")
    validate_with_trusted_producers(
        ferric,
        fe2o3,
        requirements_digest,
        index,
        sources,
        closure_paths,
        resolutions,
        artifacts,
        artifact_files,
        tcb,
        bindings,
        receipt_artifact_id,
        _test_only_validator,
    )
    counts = (
        f"33 roadmap, 17 assurance, {len(bindings)} independent bindings, "
        f"{len(artifacts)} identity-bound artifacts, sha256={digest_file(index_path)}"
    )
    if _test_only_validator is None:
        print(f"PASS: closed M1 evidence index ({counts})")
    else:
        print(f"PASS: structurally complete synthetic M1 evidence index ({counts})")


def main() -> None:
    if len(sys.argv) != 4:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO EVIDENCE_INDEX FE2O3_REPO")
    validate_evidence_index(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))


if __name__ == "__main__":
    main()
