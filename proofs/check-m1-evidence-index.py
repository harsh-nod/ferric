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
import tempfile
from collections import defaultdict
from pathlib import Path
from typing import Any, Callable, NoReturn


FORMAT = "ferric.m1-evidence-index.v1"
PRE_RECEIPT_PROTOCOL = "ferric.m1-pre-receipt-gate.v1"
PRE_RECEIPT_GATE_IDS = (
    "evidence-index",
    "hardware",
    "performance",
    "proof",
    "quality",
    "source-closure",
    "validators",
)
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
FOUNDATION_SELECTOR_KEYS = {
    "negative-mutation": "MUTATION",
    "verus-theorem": "THEOREM",
}
FOUNDATION_REGISTRIES = {
    "negative-mutation": (
        "proofs/m1/negative/REQUIRED_FOUNDATIONS",
        "proofs/m1/negative/check-registry.py",
        11,
    ),
    "verus-theorem": (
        "proofs/m1/theorem/REQUIRED_FOUNDATIONS",
        "proofs/m1/theorem/check-registry.py",
        8,
    ),
}
SELECTOR_KEY = re.compile(r"[A-Z][A-Z0-9_]*\Z")
MAX_FOUNDATION_SELECTOR_BYTES = 2_000_000

# Validator paths, protocols, and reviewed source identities are checker-owned,
# so an evidence index cannot select or substitute an executable. A None source
# identity denotes a validator that remains a RequiredFuture obligation.
TRUSTED_VALIDATORS = {
    "artifact-identity": (
        "proofs/m1/evidence/validate-artifact-identity.py",
        "ferric.m1-validator.artifact-identity.v1",
        "9556e8b8f833edd62fa982b2e2f159c8a39ae266fc3d5818c5e8daed159818d9",
    ),
    "canonical-structure-check": (
        "proofs/m1/evidence/validate-canonical-structure.py",
        "ferric.m1-validator.canonical-structure.v1",
        "dc01a45be09344b1427b1cf9d958302b810201cca69c215b2eb859177d9ec2bb",
    ),
    "external-contract": (
        "proofs/m1/evidence/validate-external-contract.py",
        "ferric.m1-validator.external-contract.v1",
        "ab9ddb1b9e8c3b6ee31e54589526a3afbd11d92370a75c2d3f9a1faae75dbdec",
    ),
    "fe2o3-contract": (
        "proofs/m1/evidence/validate-fe2o3-contract.py",
        "ferric.m1-validator.fe2o3-contract.v1",
        "d35d69f6132678a7636505798eefc1baf10a1a7dce5579e477aedd7650f1235a",
    ),
    "hardware-test": (
        "proofs/m1/evidence/validate-hardware-transcript.py",
        "ferric.m1-validator.hardware-transcript.v1",
        "bfc3a952a0ebac4eee479faf7d7306d2a8a3889ffb22ad9ec3422fcd8b1eace0",
    ),
    "independent-validator": (
        "proofs/m1/evidence/validate-independent-validator.py",
        "ferric.m1-validator.independent-validator.v1",
        "bcf7622be4d154eddaa023e7570ec36b28a30106408335b26f7e4c3fc7a940ca",
    ),
    "negative-mutation": (
        "proofs/m1/evidence/validate-negative-mutation.py",
        "ferric.m1-validator.negative-mutation.v1",
        "ff415a6207353f13c374736262c268a42b0c286ff97b76d3e3c64680d3bdae8e",
    ),
    "performance-gate": (
        "proofs/m1/evidence/validate-performance-report.py",
        "ferric.m1-validator.performance-report.v1",
        "f9f804f10dfce1ffc83aceba9bd950a4cf5cc462ae8fb7b4288a2f917b19072a",
    ),
    "qualification-receipt": (
        "proofs/m1/evidence/validate-qualification-receipt.py",
        "ferric.m1-validator.qualification-receipt.v1",
        "450c1283df88c7c36ba7a6b43627a5f4dfce26535bec21b0b9568bc336ecd7e0",
    ),
    "tcb-report": (
        "proofs/m1/evidence/validate-tcb-report.py",
        "ferric.m1-validator.tcb-report.v1",
        "edd85198dda98ee7c9e1bbf3b7d1f815ccddeb13be35ff17879a98bc66acc754",
    ),
    "unsupported-rationale": (
        "proofs/m1/evidence/validate-unsupported-rationale.py",
        "ferric.m1-validator.unsupported-rationale.v1",
        "195833cf0d8a18255aec442f49ea5e7e87c191373d981ab25d4650db3831153d",
    ),
    "verus-theorem": (
        "proofs/m1/evidence/validate-verus-theorem.py",
        "ferric.m1-validator.verus-theorem.v1",
        "cf4dc52143968b1eddf579f8af21033defe42f4739b3b0842248f9e815ddae0b",
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


def stable_source_bytes(path: Path, description: str) -> bytes:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before_path = path.lstat()
        descriptor = os.open(path, flags)
        before = os.fstat(descriptor)
    except OSError as error:
        fail(f"{description} is unavailable: {path}: {error}")
    try:
        if (
            stat.S_ISLNK(before_path.st_mode)
            or not stat.S_ISREG(before_path.st_mode)
            or not stat.S_ISREG(before.st_mode)
            or (before_path.st_dev, before_path.st_ino)
            != (before.st_dev, before.st_ino)
            or before.st_size <= 0
            or before.st_size > 8_000_000
        ):
            fail(f"{description} must be a bounded stable regular file: {path}")
        with os.fdopen(descriptor, "rb", closefd=False) as source:
            raw = source.read(8_000_001)
        after = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    finally:
        os.close(descriptor)
    try:
        after_path = path.lstat()
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")

    def identity(value: os.stat_result) -> tuple[int, ...]:
        return (
            value.st_dev,
            value.st_ino,
            value.st_mode,
            value.st_nlink,
            value.st_size,
            value.st_mtime_ns,
            value.st_ctime_ns,
        )

    if (
        len(raw) != before.st_size
        or len(raw) > 8_000_000
        or identity(before_path) != identity(before)
        or identity(before) != identity(after)
        or identity(after) != identity(after_path)
    ):
        fail(f"{description} changed while it was read: {path}")
    return raw


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


def git_tree_modes(repo: Path) -> dict[str, int]:
    result = subprocess.run(
        ["git", "-C", str(repo), "ls-tree", "-rz", "--full-tree", "HEAD"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        error = result.stderr.decode("utf-8", errors="replace").strip()
        fail(f"cannot enumerate exact Git tree: {repo}: {error}")
    modes: dict[str, int] = {}
    for record in result.stdout.split(b"\0"):
        if not record:
            continue
        header, separator, raw_name = record.partition(b"\t")
        fields = header.split(b" ")
        if not separator or len(fields) != 3:
            fail(f"exact Git tree contains a malformed entry: {repo}")
        try:
            git_mode = fields[0].decode("ascii")
            object_type = fields[1].decode("ascii")
            name = raw_name.decode("utf-8")
        except UnicodeDecodeError:
            fail(f"exact Git tree contains a non-UTF-8 entry: {repo}")
        if any(part in SOURCE_EXCLUDED_DIRECTORIES for part in Path(name).parts) or (
            Path(name).suffix in SOURCE_EXCLUDED_SUFFIXES
        ):
            continue
        if object_type != "blob" or git_mode not in {"100644", "100755"}:
            fail(f"exact Git tree contains a non-regular entry: {name}")
        if name in modes:
            fail(f"exact Git tree contains a duplicate entry: {name}")
        modes[name] = 0o755 if git_mode == "100755" else 0o644
    return modes


def source_closure(repo: Path) -> tuple[bytes, set[str]]:
    tree_modes = git_tree_modes(repo)
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
            metadata = path.stat()
            mode = tree_modes.get(relative_name)
            if mode is None:
                fail(f"M1 source closure is not the exact committed tree: {repo}")
            records.append(
                f"{relative_name}|{mode:o}|{metadata.st_size}|{digest_file(path)}"
            )
            paths.add(relative_name)
    except (OSError, ValueError) as error:
        fail(f"cannot measure M1 source closure for {repo}: {error}")
    if not records:
        fail(f"M1 source closure is empty: {repo}")
    if paths != set(tree_modes):
        fail(f"M1 source closure is not the exact committed tree: {repo}")
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
        if members != set(git_tree_modes(repositories[repository])):
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
    artifact_ids: set[str] = set()
    identities: set[str] = set()
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
        identity = require_sha256(
            record["identity_sha256"], f"TCB identity: {identifier}"
        )
        if identity != artifact["sha256"]:
            fail(f"M1 TCB identity does not bind its report: {identifier}")
        if artifact_id in artifact_ids or identity in identities:
            fail(f"M1 TCB artifact or identity is reused: {identifier}")
        entries[identifier] = record
        artifact_ids.add(artifact_id)
        identities.add(identity)
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


def checked_foundation_registries(
    ferric: Path, ferric_closure_paths: set[str]
) -> dict[str, dict[str, tuple[str, ...]]]:
    checked: dict[str, dict[str, tuple[str, ...]]] = {}
    with tempfile.TemporaryDirectory(prefix="ferric-m1-foundation-registry-") as raw:
        temporary = Path(raw)
        for evidence_kind, (
            registry_relative,
            checker_relative,
            field_count,
        ) in FOUNDATION_REGISTRIES.items():
            for relative, description in (
                (registry_relative, f"{evidence_kind} foundation registry"),
                (checker_relative, f"{evidence_kind} foundation registry checker"),
            ):
                if relative not in ferric_closure_paths:
                    fail(
                        f"{description} is absent from the exact Ferric source closure"
                    )
                regular_file(ferric / relative, description)
            output = temporary / evidence_kind
            try:
                result = subprocess.run(
                    [
                        sys.executable,
                        "-I",
                        str(ferric / checker_relative),
                        str(ferric),
                        str(ferric / registry_relative),
                        str(output),
                    ],
                    check=False,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    timeout=60,
                    cwd=ferric,
                    env={"PATH": os.environ.get("PATH", "")},
                )
            except (OSError, subprocess.TimeoutExpired) as error:
                fail(
                    f"checked {evidence_kind} foundation registry could not run: {error}"
                )
            if result.returncode != 0:
                fail(
                    f"checked {evidence_kind} foundation registry is invalid: "
                    f"{result.stdout.strip()}"
                )
            regular_file(output, f"checked {evidence_kind} foundation rows")
            try:
                raw_rows = output.read_bytes()
                text = raw_rows.decode("ascii")
            except (OSError, UnicodeDecodeError) as error:
                fail(f"cannot read checked {evidence_kind} foundation rows: {error}")
            if not raw_rows.endswith(b"\n") or raw_rows.endswith(b"\n\n"):
                fail(f"checked {evidence_kind} foundation rows are not canonical")
            rows: dict[str, tuple[str, ...]] = {}
            for line in text.splitlines():
                fields = tuple(line.split("|"))
                if len(fields) != field_count or not all(fields):
                    fail(f"checked {evidence_kind} foundation row is malformed")
                name = fields[0]
                if not SAFE_ID.fullmatch(name) or name in rows:
                    fail(
                        f"checked {evidence_kind} foundation selector is invalid: {name!r}"
                    )
                source = safe_relative(fields[5], f"{evidence_kind} foundation source")
                source_relative = source.as_posix()
                if source_relative not in ferric_closure_paths:
                    fail(
                        f"checked {evidence_kind} foundation source is absent from "
                        f"the exact Ferric source closure: {name}"
                    )
                if evidence_kind == "negative-mutation":
                    mutator = safe_relative(fields[6], "negative-mutation mutator")
                    mutator_relative = (
                        Path("proofs/m1/negative/components") / mutator
                    ).as_posix()
                    if mutator_relative not in ferric_closure_paths:
                        fail(
                            "checked negative-mutation mutator is absent from the exact "
                            f"Ferric source closure: {name}"
                        )
                rows[name] = fields
            if not rows or tuple(rows) != tuple(sorted(rows)):
                fail(f"checked {evidence_kind} foundation roster is empty or reordered")
            checked[evidence_kind] = rows
    return checked


def artifact_foundation_selector(artifact_path: Path, evidence_kind: str) -> str:
    selector_key = FOUNDATION_SELECTOR_KEYS[evidence_kind]
    try:
        with artifact_path.open("rb") as stream:
            raw = stream.read(MAX_FOUNDATION_SELECTOR_BYTES + 1)
        text = raw.decode("ascii")
    except (OSError, UnicodeDecodeError) as error:
        fail(f"{evidence_kind} foundation selector artifact is unreadable: {error}")
    lines = text.splitlines()
    canonical = ("\n".join(lines) + "\n").encode("ascii")
    if (
        not raw
        or len(raw) > MAX_FOUNDATION_SELECTOR_BYTES
        or raw != canonical
        or raw.endswith(b"\n\n")
    ):
        fail(f"{evidence_kind} foundation selector artifact is not canonical")
    values: dict[str, str] = {}
    for line in lines:
        if "=" not in line:
            fail(f"{evidence_kind} foundation selector artifact is malformed")
        key, value = line.split("=", 1)
        if not SELECTOR_KEY.fullmatch(key) or not value or key in values:
            fail(f"{evidence_kind} foundation selector artifact is malformed")
        values[key] = value
    opposite_key = "THEOREM" if selector_key == "MUTATION" else "MUTATION"
    if selector_key not in values or opposite_key in values:
        fail(f"{evidence_kind} foundation selector is missing or has the wrong kind")
    selector = values[selector_key]
    if not SAFE_ID.fullmatch(selector):
        fail(f"{evidence_kind} foundation selector is malformed: {selector!r}")
    if artifact_path.name != f"{selector}.result":
        fail(
            f"{evidence_kind} artifact filename does not match its foundation selector"
        )
    return selector


def validate_foundation_reachability(
    ferric: Path,
    ferric_closure_paths: set[str],
    specs: list[dict[str, Any]],
    bindings: dict[str, dict[str, Any]],
    artifact_files: dict[str, Path],
) -> None:
    registries = checked_foundation_registries(ferric, ferric_closure_paths)
    status_by_key = {(spec["class"], spec["id"]): spec["status"] for spec in specs}
    for identifier, binding in bindings.items():
        evidence_kind = binding["evidence_kind"]
        rows = registries.get(evidence_kind)
        if rows is None:
            continue
        if binding["obligation_class"] != "Assurance":
            fail(
                f"{evidence_kind} foundation selector is not Assurance-bound: {identifier}"
            )
        status = status_by_key[("Assurance", binding["obligation_id"])]
        if status == "Unsupported" or (
            evidence_kind == "verus-theorem" and status != "Proved"
        ):
            fail(
                f"{evidence_kind} foundation selector cannot discharge {status}: "
                f"{identifier}"
            )
        if binding["source_identity_id"] != "source.ferric":
            fail(
                f"{evidence_kind} foundation selector substituted a non-Ferric source: "
                f"{identifier}"
            )
        artifact_path = artifact_files[binding["artifact_id"]]
        selector = artifact_foundation_selector(artifact_path, evidence_kind)
        row = rows.get(selector)
        if row is None:
            fail(
                f"{evidence_kind} foundation selector is not in the checked registry: "
                f"{selector}"
            )
        if row[2] != binding["obligation_id"]:
            fail(
                f"{evidence_kind} foundation selector substituted a different property: "
                f"{identifier}"
            )
        if row[3] != binding["path_id"]:
            fail(
                f"{evidence_kind} foundation selector substituted a different path: "
                f"{identifier}"
            )


def validate_bindings(
    value: Any,
    specs: list[dict[str, Any]],
    profiles: dict[str, tuple[str, ...]],
    binding_classes: dict[str, tuple[str, ...]],
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
    observed_triplets: dict[tuple[str, str], set[tuple[str, str, str]]] = defaultdict(
        set
    )
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
        if spec["class"] not in binding_classes.get(kind, ()):
            fail(
                f"M1 evidence kind does not support the obligation class: {identifier}"
            )
        path_id = record["path_id"]
        if path_id not in spec["paths"] or path_id not in resolutions:
            fail(f"M1 evidence binding has the wrong path: {identifier}")
        triplet = (profile, kind, path_id)
        if triplet in observed_triplets[key]:
            fail(
                "duplicate M1 profile-kind-path evidence binding: "
                f"{key[0]}:{key[1]}:{profile}:{kind}:{path_id}"
            )
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
        observed_triplets[key].add(triplet)
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
            if spec["class"] in binding_classes[kind]
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
    *,
    allow_missing_receipt: bool = False,
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
                "rationale_artifact_ids",
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
            if receipt is None and allow_missing_receipt:
                if receipt_id != "artifact.qualification.m1":
                    fail(f"M1 roadmap receipt identity drifted: {spec['id']}")
            elif receipt is None or receipt["kind"] != "QualificationReceipt":
                fail(f"M1 roadmap receipt is unavailable or fake: {spec['id']}")
            receipt_ids.add(receipt_id)
            if receipt is not None:
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
            supplied_rationale_ids = string_list(
                record["rationale_artifact_ids"],
                f"closure {key} rationale artifacts",
            )
            if supplied_rationale_ids != rationale_ids:
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
    validator_raw = stable_source_bytes(
        validator, f"trusted M1 validator for {evidence_kind}"
    )
    if expected_source_digest is None:
        fail(f"trusted M1 validator has no pinned source identity: {relative}")
    if require_sha256(
        expected_source_digest, f"trusted validator source: {evidence_kind}"
    ) != digest_bytes(validator_raw):
        fail(f"trusted M1 validator source identity mismatch: {relative}")
    payload = json.dumps(
        context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    payload_digest = digest_bytes(payload.encode("ascii"))
    artifact_digest = context["artifact"]["sha256"]
    bootstrap = (
        "import os,sys;"
        "p=sys.argv.pop(1);f=int(sys.argv.pop(1));sys.argv[0]=p;"
        "s=os.fdopen(os.dup(f),'rb').read();"
        "g={'__name__':'__main__','__file__':p,'__package__':None};"
        "exec(compile(s,p,'exec'),g)"
    )
    with tempfile.TemporaryFile(mode="w+b") as pinned_source:
        pinned_source.write(validator_raw)
        pinned_source.flush()
        pinned_source.seek(0)
        try:
            result = subprocess.run(
                [
                    sys.executable,
                    "-I",
                    "-c",
                    bootstrap,
                    str(validator),
                    str(pinned_source.fileno()),
                    validator_format,
                ],
                check=False,
                input=payload + "\n",
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                timeout=60,
                cwd=ferric,
                env={"PATH": os.environ.get("PATH", "")},
                pass_fds=(pinned_source.fileno(),),
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
    *,
    evidence_kinds: set[str] | None = None,
    include_tcb: bool = True,
    include_receipt: bool = True,
) -> None:
    common = {
        "format": FORMAT,
        "requirements_sha256": requirements_digest,
        "sources": [sources[identifier] for identifier in SOURCE_IDS],
        "tcb": [tcb[identifier] for identifier in TCB_IDS],
    }
    for identifier, binding in bindings.items():
        if (
            evidence_kinds is not None
            and binding["evidence_kind"] not in evidence_kinds
        ):
            continue
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
    for identifier in TCB_IDS if include_tcb else ():
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
    if not include_receipt:
        return
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
    _pre_receipt_gate: str | None = None,
    _pre_receipt_artifact_root: Path | None = None,
) -> None:
    if _pre_receipt_gate is not None and _pre_receipt_gate not in PRE_RECEIPT_GATE_IDS:
        fail(f"unknown M1 pre-receipt gate: {_pre_receipt_gate}")
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

    if _pre_receipt_artifact_root is not None:
        if _pre_receipt_gate is None:
            fail("M1 artifact-root override is restricted to pre-receipt validation")
        supplied_root = _pre_receipt_artifact_root.absolute()
        try:
            index_root = _pre_receipt_artifact_root.resolve(strict=True)
        except OSError as error:
            fail(f"M1 pre-receipt artifact root is unavailable: {error}")
        if supplied_root != index_root or not index_root.is_dir():
            fail("M1 pre-receipt artifact root is not a canonical directory")
    else:
        index_root = index_path.resolve(strict=True).parent
    artifacts, artifact_files = validate_artifacts(index_root, index["artifacts"])
    if _pre_receipt_gate is not None and "artifact.qualification.m1" in artifacts:
        fail("M1 pre-receipt candidate already contains the qualification receipt")
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
    binding_classes = {
        record["kind"]: tuple(record["classes"])
        for record in requirements["evidence_kind_binding_classes"]
    }
    specs = obligation_specs(requirements)
    bindings, grouped = validate_bindings(
        index["evidence_bindings"],
        specs,
        profiles,
        binding_classes,
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
        allow_missing_receipt=_pre_receipt_gate is not None,
    )
    if used_artifacts != set(artifacts):
        unused = sorted(set(artifacts) - used_artifacts)
        fail(f"M1 evidence index contains unreferenced artifacts: {unused}")
    validate_foundation_reachability(
        ferric,
        closure_paths["source.ferric"],
        specs,
        bindings,
        artifact_files,
    )
    if _pre_receipt_gate is None:
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
    else:
        gate_kinds: dict[str, set[str] | None] = {
            "evidence-index": set(),
            "hardware": {"hardware-test"},
            "performance": {"performance-gate"},
            "proof": {"negative-mutation", "verus-theorem"},
            "quality": set(),
            "source-closure": set(),
            "validators": None,
        }
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
            evidence_kinds=gate_kinds[_pre_receipt_gate],
            include_tcb=_pre_receipt_gate == "validators",
            include_receipt=False,
        )
    counts = (
        f"33 roadmap, 17 assurance, {len(bindings)} independent bindings, "
        f"{len(artifacts)} identity-bound artifacts, sha256={digest_file(index_path)}"
    )
    if _pre_receipt_gate is not None:
        print(
            f"PASS: {PRE_RECEIPT_PROTOCOL} gate={_pre_receipt_gate} "
            f"candidate_sha256={digest_file(index_path)}"
        )
    elif _test_only_validator is None:
        print(f"PASS: closed M1 evidence index ({counts})")
    else:
        print(f"PASS: structurally complete synthetic M1 evidence index ({counts})")


def main() -> None:
    if len(sys.argv) == 7 and sys.argv[1] == PRE_RECEIPT_PROTOCOL:
        validate_evidence_index(
            Path(sys.argv[3]),
            Path(sys.argv[4]),
            Path(sys.argv[6]),
            _pre_receipt_gate=sys.argv[2],
            _pre_receipt_artifact_root=Path(sys.argv[5]),
        )
        return
    if len(sys.argv) != 4:
        fail(
            f"usage: {sys.argv[0]} FERRIC_REPO EVIDENCE_INDEX FE2O3_REPO; or "
            f"{sys.argv[0]} {PRE_RECEIPT_PROTOCOL} GATE FERRIC_REPO "
            "CANDIDATE_INDEX ARTIFACT_ROOT FE2O3_REPO"
        )
    validate_evidence_index(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]))


if __name__ == "__main__":
    main()
