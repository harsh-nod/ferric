#!/usr/bin/env python3
"""Validate the single identity-bound M1 qualification receipt."""

from __future__ import annotations

import ast
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
from typing import Any, NoReturn


PROTOCOL = "ferric.m1-validator.qualification-receipt.v1"
OBLIGATION_CLASSES = ()
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REPORT_FORMAT = "FERRIC-M1-QUALIFICATION-RECEIPT-V1"
TRANSCRIPT_FORMAT = "FERRIC-M1-QUALIFICATION-TRANSCRIPT-V1"
QUALIFICATION_PROTOCOL = "ferric.m1.qualification.v1"
TARGET = "gfx942:xnack-"
AUTHORITY = "m1-qualification-receipt-only"
NONCLAIM = (
    "This validator authenticates the exact evidence closure and immutable "
    "qualification transcript supplied by the checker. It does not generate "
    "evidence, execute qualification gates, alter repository requirements, or "
    "turn an Open in-repository obligation into a closure claim."
)
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.:+/-]+\Z")
PRINTABLE_ASCII = re.compile(r"[\x20-\x7e]{1,256}\Z")
UTC_TIMESTAMP = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z\Z")
UUID = re.compile(
    r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-"
    r"[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z"
)
PCI_BDF = re.compile(r"[0-9a-f]{4}:[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]\Z")
PYTHON_VERSION = re.compile(r"3\.[0-9]+\.[0-9]+\Z")
MAX_CONTEXT_BYTES = 32_000_000
MAX_REQUIREMENTS_BYTES = 2_000_000
MAX_REPORT_BYTES = 4_000_000
MAX_TRANSCRIPT_BYTES = 4_000_000
MAX_CHECKER_BYTES = 2_000_000
MAX_VALIDATOR_BYTES = 4_000_000
MAX_ARTIFACT_BYTES = 64_000_000
MAX_TOTAL_ARTIFACT_BYTES = 512_000_000
MAX_ARTIFACTS = 10_000
MAX_BINDINGS = 10_000
SOURCE_EXCLUDED_DIRECTORIES = {".git", ".ruff_cache", "__pycache__", "target"}
SOURCE_EXCLUDED_SUFFIXES = {".pyc", ".receipt"}
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {"source.fe2o3": "fe2o3", "source.ferric": "ferric"}
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
EVIDENCE_KIND_BINDING_CLASSES = (
    ("artifact-identity", ("Assurance", "Roadmap")),
    ("canonical-structure-check", ("Assurance", "Roadmap")),
    ("external-contract", ("Assurance", "Roadmap")),
    ("fe2o3-contract", ("Assurance", "Roadmap")),
    ("hardware-test", ("Assurance", "Roadmap")),
    ("independent-validator", ("Assurance", "Roadmap")),
    ("negative-mutation", ("Assurance",)),
    ("performance-gate", ("Assurance", "Roadmap")),
    ("tcb-report", ()),
    ("unsupported-rationale", ("Assurance",)),
    ("verus-theorem", ("Assurance",)),
)
ARTIFACT_KINDS = set(EVIDENCE_ARTIFACT_KINDS.values()) | {
    "QualificationReceipt",
    "SourceClosure",
}
VALIDATOR_IDS = (
    "artifact-identity",
    "canonical-structure-check",
    "external-contract",
    "fe2o3-contract",
    "hardware-test",
    "independent-validator",
    "negative-mutation",
    "performance-gate",
    "qualification-receipt",
    "tcb-report",
    "unsupported-rationale",
    "verus-theorem",
)
GATE_IDS = (
    "evidence-index",
    "hardware",
    "performance",
    "proof",
    "quality",
    "source-closure",
    "validators",
)
TOOL_IDS = (
    "compiler.cargo",
    "compiler.rustc",
    "compiler.verus",
    "runtime.python",
    "validator.evidence-index",
    "validator.qualification-receipt",
)

CONTEXT_KEYS = {
    "artifact",
    "artifact_absolute_path",
    "format",
    "index",
    "repository_absolute_paths",
    "requirements_sha256",
    "sources",
    "subject",
    "tcb",
}
INDEX_KEYS = {
    "artifacts",
    "evidence_bindings",
    "format",
    "obligations",
    "path_resolutions",
    "requirements_sha256",
    "sources",
    "tcb",
}
REPOSITORY_KEYS = {"fe2o3", "ferric"}
ARTIFACT_KEYS = {"id", "kind", "path", "sha256", "size_bytes"}
SOURCE_KEYS = {
    "base_commit",
    "commit",
    "id",
    "repository",
    "source_closure_artifact_id",
    "source_closure_sha256",
    "tree",
}
TCB_KEYS = {"artifact_id", "id", "identity_sha256", "kind"}
PATH_KEYS = {"availability", "id", "path", "repository", "source_identity_id"}
BINDING_KEYS = {
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
}
REQUIREMENTS_KEYS = {
    "assurance_properties",
    "evidence_kind_binding_classes",
    "evidence_kinds",
    "evidence_profiles",
    "format",
    "m0_contracts_commit",
    "m1_upstream_base_commit",
    "m1_upstream_base_tree",
    "milestone",
    "path_obligations",
    "roadmap_requirements",
}
VALIDATOR_KEYS = {
    "availability",
    "evidence_kind",
    "path",
    "protocol",
    "source_sha256",
}
SOURCE_CLOSURE_KEYS = {
    "artifact_id",
    "commit",
    "file_count",
    "id",
    "sha256",
    "tree",
}
RECEIPT_ARTIFACT_KEYS = {"id", "kind", "path"}
REPORT_KEYS = {
    "artifact_count",
    "artifact_roster_sha256",
    "assurance_count",
    "authority",
    "binding_count",
    "binding_roster_sha256",
    "format",
    "gate_ids",
    "index_roster_sha256",
    "milestone",
    "nonclaim",
    "obligation_roster_sha256",
    "path_count",
    "path_roster_sha256",
    "protocol",
    "qualification_id_sha256",
    "receipt_artifact",
    "requirements_roster_sha256",
    "requirements_sha256",
    "result",
    "roadmap_count",
    "source_closure_roster",
    "source_roster",
    "source_roster_sha256",
    "target",
    "tcb_roster",
    "tcb_roster_sha256",
    "transcript_relative_path",
    "transcript_sha256",
    "transcript_size_bytes",
    "validator_count",
    "validator_roster",
    "validator_roster_sha256",
}
TRANSCRIPT_KEYS = {
    "all_required_gates_passed",
    "environment",
    "environment_identity_sha256",
    "finished_at_utc",
    "format",
    "gate_roster_sha256",
    "gates",
    "index_roster_sha256",
    "milestone",
    "no_failed_gates",
    "no_skipped_gates",
    "protocol",
    "qualification_id_sha256",
    "requirements_sha256",
    "result",
    "run_id",
    "source_closure_sha256s",
    "source_roster_sha256",
    "started_at_utc",
    "target",
    "target_identity_sha256",
    "tcb_identity_sha256s",
    "tcb_roster_sha256",
    "tool_roster_sha256",
    "tools",
    "validator_roster_sha256",
}
GATE_KEYS = {
    "artifact_ids",
    "binding_ids",
    "command_sha256",
    "finished_at_utc",
    "id",
    "output_sha256",
    "result",
    "started_at_utc",
}
TOOL_KEYS = {"authority", "id", "identity_sha256", "version"}
ENVIRONMENT_KEYS = {"device", "driver", "firmware", "host", "rocm"}
DEVICE_KEYS = {
    "device_count",
    "device_uuid",
    "marketing_name",
    "pci_bdf",
    "processor",
    "vendor_id",
    "xnack",
}
DRIVER_KEYS = {"module_sha256", "name", "version"}
FIRMWARE_KEYS = {"bundle_sha256", "package_version"}
HOST_KEYS = {"kernel_sha256", "machine", "os_release_sha256"}
ROCM_KEYS = {"installation_sha256", "version"}
TARGET_VALUE = {
    "architecture": "gfx942",
    "device_count": 1,
    "feature": "xnack-",
    "triple": "amdgcn-amd-amdhsa",
}


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def exact_keys(value: Any, expected: set[str], description: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        fail(f"{description} fields drifted")
    return value


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_digest(value: Any) -> str:
    return digest_bytes(
        json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
    )


def require_sha256(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or SHA256.fullmatch(value) is None
        or len(set(value)) == 1
    ):
        fail(f"invalid {description}")
    return value


def require_git_id(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or GIT_ID.fullmatch(value) is None
        or len(set(value)) == 1
    ):
        fail(f"invalid {description}")
    return value


def require_id(value: Any, description: str) -> str:
    if not isinstance(value, str) or SAFE_ID.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_name(value: Any, description: str) -> str:
    if not isinstance(value, str) or SAFE_NAME.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_ascii(value: Any, description: str) -> str:
    if not isinstance(value, str) or PRINTABLE_ASCII.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def string_list(
    value: Any, description: str, *, allow_empty: bool = False
) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        fail(f"{description} must be a string array")
    if not allow_empty and not value:
        fail(f"{description} must not be empty")
    if len(value) != len(set(value)):
        fail(f"{description} contains a duplicate")
    return value


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def metadata_identity(value: os.stat_result) -> tuple[int, ...]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_nlink,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def read_bounded(
    path: Path, limit: int, description: str, *, single_link: bool = True
) -> bytes:
    try:
        before_path = path.lstat()
        if stat.S_ISLNK(before_path.st_mode) or not stat.S_ISREG(before_path.st_mode):
            fail(f"{description} must be a regular non-symlink file")
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or (single_link and before.st_nlink != 1):
            fail(f"{description} must be a single-link regular file")
        if before.st_size <= 0 or before.st_size > limit:
            fail(f"{description} size is invalid")
        with os.fdopen(descriptor, "rb", closefd=False) as source:
            payload = source.read(limit + 1)
        after = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    finally:
        os.close(descriptor)
    try:
        after_path = path.lstat()
    except OSError as error:
        fail(f"cannot restat {description}: {error}")
    if (
        len(payload) != before.st_size
        or len(payload) > limit
        or metadata_identity(before_path) != metadata_identity(before)
        or metadata_identity(before) != metadata_identity(after)
        or metadata_identity(after) != metadata_identity(after_path)
    ):
        fail(f"{description} changed while it was read")
    return payload


def load_canonical_json(
    path: Path, limit: int, description: str, *, compact: bool = False
) -> tuple[dict[str, Any], bytes]:
    raw = read_bounded(path, limit, description)
    try:
        source = raw.decode("ascii")
        value = json.loads(source, object_pairs_hook=unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse {description}: {error}")
    if not isinstance(value, dict):
        fail(f"{description} must be an object")
    if compact:
        canonical = (
            json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
            + "\n"
        )
    else:
        canonical = (
            json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
        )
    if source != canonical:
        fail(f"{description} is not canonical JSON")
    return value, raw


def load_context() -> tuple[dict[str, Any], bytes]:
    payload = sys.stdin.buffer.read(MAX_CONTEXT_BYTES + 1)
    if not payload or len(payload) > MAX_CONTEXT_BYTES:
        fail("qualification-receipt context is empty or oversized")
    try:
        source = payload.decode("ascii")
        value = json.loads(source, object_pairs_hook=unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse qualification-receipt context: {error}")
    exact_keys(value, CONTEXT_KEYS, "qualification-receipt context")
    canonical = (
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    )
    if source != canonical:
        fail("qualification-receipt context is not canonical JSON")
    return value, payload.removesuffix(b"\n")


def safe_relative(value: Any, description: str) -> PurePosixPath:
    if not isinstance(value, str) or not value or "\\" in value or "\x00" in value:
        fail(f"invalid {description}")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        fail(f"unsafe {description}")
    if path.as_posix() != value:
        fail(f"noncanonical {description}")
    return path


def evidence_root(report: Path, relative: PurePosixPath) -> Path:
    if not report.is_absolute():
        fail("qualification receipt absolute path is not absolute")
    root = report
    for _ in relative.parts:
        root = root.parent
    try:
        if (root / Path(*relative.parts)).absolute() != report.absolute():
            fail("qualification receipt path and absolute path disagree")
    except OSError as error:
        fail(f"cannot resolve qualification receipt root: {error}")
    return root


def reject_symlink_components(
    root: Path, relative: PurePosixPath, description: str
) -> Path:
    try:
        root_meta = root.lstat()
    except OSError as error:
        fail(f"evidence root is unavailable: {error}")
    if stat.S_ISLNK(root_meta.st_mode) or not stat.S_ISDIR(root_meta.st_mode):
        fail("evidence root must be a non-symlink directory")
    current = root
    for part in relative.parts:
        current = current / part
        try:
            metadata = current.lstat()
        except OSError as error:
            fail(f"{description} is unavailable: {error}")
        if stat.S_ISLNK(metadata.st_mode):
            fail(f"{description} path contains a symlink")
    return current


def run_git(repo: Path, *arguments: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), *arguments],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=30,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"Git identity query failed for {repo}: {error}")
    if result.returncode != 0:
        fail(f"Git identity query failed for {repo}: {result.stderr.strip()}")
    return result.stdout


def git_identity(repo: Path) -> tuple[str, str]:
    commit = require_git_id(
        run_git(repo, "rev-parse", "HEAD^{commit}").strip(), "commit"
    )
    tree = require_git_id(run_git(repo, "rev-parse", "HEAD^{tree}").strip(), "tree")
    if run_git(repo, "status", "--porcelain=v1", "--untracked-files=all"):
        fail(f"source repository is not the exact clean Git tree: {repo}")
    return commit, tree


def included_source_path(name: str) -> bool:
    path = Path(name)
    return not any(part in SOURCE_EXCLUDED_DIRECTORIES for part in path.parts) and (
        path.suffix not in SOURCE_EXCLUDED_SUFFIXES
    )


def git_tree_modes(repo: Path) -> dict[str, int]:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), "ls-tree", "-rz", "--full-tree", "HEAD"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=30,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"Git tree query failed for {repo}: {error}")
    if result.returncode != 0:
        error = result.stderr.decode("utf-8", errors="replace").strip()
        fail(f"Git tree query failed for {repo}: {error}")
    modes: dict[str, int] = {}
    for record in result.stdout.split(b"\0"):
        if not record:
            continue
        header, separator, raw_name = record.partition(b"\t")
        fields = header.split(b" ")
        if not separator or len(fields) != 3:
            fail(f"Git tree contains a malformed entry: {repo}")
        try:
            git_mode = fields[0].decode("ascii")
            object_type = fields[1].decode("ascii")
            name = raw_name.decode("utf-8")
        except UnicodeDecodeError:
            fail(f"Git tree contains a non-UTF-8 entry: {repo}")
        if not included_source_path(name):
            continue
        if object_type != "blob" or git_mode not in {"100644", "100755"}:
            fail(f"Git tree contains a non-regular entry: {name}")
        if name in modes:
            fail(f"Git tree contains a duplicate entry: {name}")
        modes[name] = 0o755 if git_mode == "100755" else 0o644
    return modes


def source_closure(repo: Path) -> tuple[bytes, set[str]]:
    tree_modes = git_tree_modes(repo)
    records: list[str] = []
    members: set[str] = set()
    try:
        candidates = sorted(
            repo.rglob("*"), key=lambda path: path.relative_to(repo).as_posix()
        )
        for path in candidates:
            relative = path.relative_to(repo)
            name = relative.as_posix()
            if not included_source_path(name):
                continue
            metadata = path.lstat()
            if stat.S_ISLNK(metadata.st_mode):
                fail(f"source closure contains a symlink: {name}")
            if stat.S_ISDIR(metadata.st_mode):
                continue
            if not stat.S_ISREG(metadata.st_mode):
                fail(f"source closure contains a special entry: {name}")
            raw = read_bounded(
                path, MAX_ARTIFACT_BYTES, f"source file {name}", single_link=False
            )
            mode = tree_modes.get(name)
            if mode is None:
                fail(f"source closure is not the exact committed tree: {repo}")
            records.append(
                f"{name}|{mode:o}|{len(raw)}|{digest_bytes(raw)}"
            )
            members.add(name)
    except (OSError, ValueError) as error:
        fail(f"cannot measure source closure for {repo}: {error}")
    if not records or members != set(tree_modes):
        fail(f"source closure is not the exact committed tree: {repo}")
    return ("\n".join(records) + "\n").encode("utf-8"), members


def validate_requirements(value: dict[str, Any]) -> None:
    exact_keys(value, REQUIREMENTS_KEYS, "M1 requirements manifest")
    if value["format"] != "ferric.m1-requirements.v1" or value["milestone"] != "M1":
        fail("M1 requirements format or milestone drifted")
    require_git_id(value["m0_contracts_commit"], "M0 contracts commit")
    require_git_id(value["m1_upstream_base_commit"], "M1 upstream base commit")
    require_git_id(value["m1_upstream_base_tree"], "M1 upstream base tree")
    roadmaps = value["roadmap_requirements"]
    assurances = value["assurance_properties"]
    paths = value["path_obligations"]
    profiles = value["evidence_profiles"]
    applicability = value["evidence_kind_binding_classes"]
    if not isinstance(roadmaps, list) or len(roadmaps) != 33:
        fail("qualification requires exactly 33 roadmap requirements")
    if not isinstance(assurances, list) or len(assurances) != 17:
        fail("qualification requires exactly 17 assurance properties")
    if not isinstance(paths, list) or len(paths) != 39:
        fail("qualification requires exactly 39 path obligations")
    if not isinstance(profiles, list) or len(profiles) != 7:
        fail("qualification requires exactly seven evidence profiles")
    if not isinstance(applicability, list):
        fail("qualification evidence-kind binding-class roster is invalid")
    observed_applicability: list[tuple[str, tuple[str, ...]]] = []
    for record in applicability:
        exact_keys(record, {"classes", "kind"}, "evidence-kind binding classes")
        kind = record["kind"]
        classes = record["classes"]
        if (
            not isinstance(kind, str)
            or not isinstance(classes, list)
            or not all(isinstance(item, str) for item in classes)
        ):
            fail("qualification evidence-kind binding-class roster is malformed")
        observed_applicability.append((kind, tuple(classes)))
    if tuple(observed_applicability) != EVIDENCE_KIND_BINDING_CLASSES:
        fail("qualification evidence-kind binding-class roster drifted")
    if any(record.get("obligation_state") != "Open" for record in roadmaps):
        fail("repository roadmap obligations must remain Open")
    if any(record.get("obligation_state") != "Open" for record in assurances):
        fail("repository assurance obligations must remain Open")
    if any(record.get("obligation_state") != "Open" for record in paths):
        fail("repository path obligations must remain Open")


def validate_artifacts(
    root: Path, value: Any
) -> tuple[dict[str, dict[str, Any]], dict[str, Path]]:
    if not isinstance(value, list) or not value or len(value) > MAX_ARTIFACTS:
        fail("qualification artifact roster is empty or oversized")
    records: dict[str, dict[str, Any]] = {}
    files: dict[str, Path] = {}
    paths: set[str] = set()
    inodes: set[tuple[int, int]] = set()
    total_size = 0
    for record in value:
        exact_keys(record, ARTIFACT_KEYS, "qualification artifact")
        identifier = require_id(record["id"], "artifact id")
        if identifier in records:
            fail(f"duplicate qualification artifact: {identifier}")
        if record["kind"] not in ARTIFACT_KINDS:
            fail(f"unknown qualification artifact kind: {record['kind']!r}")
        size = record["size_bytes"]
        if (
            not isinstance(size, int)
            or isinstance(size, bool)
            or size <= 0
            or size > MAX_ARTIFACT_BYTES
        ):
            fail(f"qualification artifact size is invalid: {identifier}")
        total_size += size
        if total_size > MAX_TOTAL_ARTIFACT_BYTES:
            fail("qualification artifact roster exceeds its total byte bound")
        relative = safe_relative(record["path"], f"artifact path {identifier}")
        if relative.as_posix() in paths:
            fail("qualification artifact path is reused")
        path = reject_symlink_components(root, relative, f"artifact {identifier}")
        raw = read_bounded(path, MAX_ARTIFACT_BYTES, f"artifact {identifier}")
        try:
            metadata = path.lstat()
        except OSError as error:
            fail(f"cannot inspect artifact {identifier}: {error}")
        inode = (metadata.st_dev, metadata.st_ino)
        if inode in inodes:
            fail("qualification artifacts must not be hard-linked")
        inodes.add(inode)
        if size != len(raw) or require_sha256(
            record["sha256"], f"artifact {identifier} SHA-256"
        ) != digest_bytes(raw):
            fail(f"qualification artifact identity mismatch: {identifier}")
        records[identifier] = record
        files[identifier] = path
        paths.add(relative.as_posix())
    if tuple(records) != tuple(sorted(records)):
        fail("qualification artifact roster is not canonical")
    return records, files


def validate_sources(
    value: Any,
    requirements: dict[str, Any],
    repositories: dict[str, Path],
    artifacts: dict[str, dict[str, Any]],
    files: dict[str, Path],
    used: set[str],
) -> tuple[list[dict[str, Any]], dict[str, set[str]], list[dict[str, Any]]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail("qualification source roster is incomplete")
    expected_bases = {
        "source.fe2o3": requirements["m1_upstream_base_commit"],
        "source.ferric": FERRIC_BASE_COMMIT,
    }
    paths_by_source: dict[str, set[str]] = {}
    closure_roster: list[dict[str, Any]] = []
    seen_closures: set[str] = set()
    for record, expected_id in zip(value, SOURCE_IDS, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source {expected_id}")
        repository_name = SOURCE_REPOSITORIES[expected_id]
        if (
            record["id"] != expected_id
            or record["repository"] != repository_name
            or record["base_commit"] != expected_bases[expected_id]
        ):
            fail("qualification source order or authority drifted")
        commit = require_git_id(record["commit"], f"{expected_id} commit")
        tree = require_git_id(record["tree"], f"{expected_id} tree")
        actual_commit, actual_tree = git_identity(repositories[repository_name])
        if commit != actual_commit or tree != actual_tree:
            fail(f"qualification source replay or identity drifted: {expected_id}")
        artifact_id = require_id(
            record["source_closure_artifact_id"], f"{expected_id} closure artifact"
        )
        artifact = artifacts.get(artifact_id)
        if artifact is None or artifact["kind"] != "SourceClosure":
            fail(f"qualification source closure is unavailable: {expected_id}")
        closure, members = source_closure(repositories[repository_name])
        closure_sha256 = digest_bytes(closure)
        if (
            read_bounded(
                files[artifact_id],
                MAX_ARTIFACT_BYTES,
                f"source closure artifact {expected_id}",
            )
            != closure
            or artifact["sha256"] != closure_sha256
            or require_sha256(
                record["source_closure_sha256"], f"{expected_id} source closure"
            )
            != closure_sha256
            or closure_sha256 in seen_closures
        ):
            fail(f"qualification source closure drifted: {expected_id}")
        seen_closures.add(closure_sha256)
        used.add(artifact_id)
        paths_by_source[expected_id] = members
        closure_roster.append(
            {
                "artifact_id": artifact_id,
                "commit": commit,
                "file_count": len(members),
                "id": expected_id,
                "sha256": closure_sha256,
                "tree": tree,
            }
        )
    return value, paths_by_source, closure_roster


def validate_tcb(
    value: Any, artifacts: dict[str, dict[str, Any]], used: set[str]
) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("qualification TCB roster is incomplete")
    identities: set[str] = set()
    artifact_ids: set[str] = set()
    for record, expected_id in zip(value, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, f"TCB {expected_id}")
        artifact_id = require_id(record["artifact_id"], f"{expected_id} artifact")
        identity = require_sha256(record["identity_sha256"], f"{expected_id} identity")
        artifact = artifacts.get(artifact_id)
        if (
            record["id"] != expected_id
            or record["kind"] != TCB_KINDS[expected_id]
            or artifact is None
            or artifact["kind"] != "TcbReport"
            or artifact["sha256"] != identity
            or artifact_id in artifact_ids
            or identity in identities
        ):
            fail("qualification TCB identity, order, or kind drifted")
        artifact_ids.add(artifact_id)
        identities.add(identity)
        used.add(artifact_id)
    return value


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
                "status": "Closed",
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


def validate_paths(
    value: Any,
    requirements: dict[str, Any],
    paths_by_source: dict[str, set[str]],
) -> dict[str, dict[str, Any]]:
    expected = requirements["path_obligations"]
    if not isinstance(value, list) or len(value) != len(expected):
        fail("qualification path roster is incomplete")
    result: dict[str, dict[str, Any]] = {}
    for record, specification in zip(value, expected, strict=True):
        exact_keys(record, PATH_KEYS, "qualification path resolution")
        source_id = f"source.{specification['repository']}"
        if (
            record["id"] != specification["id"]
            or record["availability"] != specification["availability"]
            or record["path"] != specification["path"]
            or record["repository"] != specification["repository"]
            or record["source_identity_id"] != source_id
            or record["path"] not in paths_by_source[source_id]
            or record["id"] in result
        ):
            fail(f"qualification path resolution drifted: {specification['id']}")
        result[record["id"]] = record
    return result


def validate_bindings(
    value: Any,
    specs: list[dict[str, Any]],
    requirements: dict[str, Any],
    paths: dict[str, dict[str, Any]],
    artifacts: dict[str, dict[str, Any]],
    used: set[str],
) -> tuple[dict[str, dict[str, Any]], dict[tuple[str, str], list[str]]]:
    if not isinstance(value, list) or not value or len(value) > MAX_BINDINGS:
        fail("qualification binding roster is empty or oversized")
    profiles = {
        record["id"]: tuple(record["kinds"])
        for record in requirements["evidence_profiles"]
    }
    binding_classes = {kind: classes for kind, classes in EVIDENCE_KIND_BINDING_CLASSES}
    spec_by_key = {(record["class"], record["id"]): record for record in specs}
    result: dict[str, dict[str, Any]] = {}
    grouped: dict[tuple[str, str], list[str]] = {key: [] for key in spec_by_key}
    pairs: dict[tuple[str, str], set[tuple[str, str]]] = {
        key: set() for key in spec_by_key
    }
    triplets: dict[tuple[str, str], set[tuple[str, str, str]]] = {
        key: set() for key in spec_by_key
    }
    covered_paths: dict[tuple[str, str], set[str]] = {key: set() for key in spec_by_key}
    bound_artifacts: set[str] = set()
    for record in value:
        exact_keys(record, BINDING_KEYS, "qualification evidence binding")
        identifier = require_id(record["id"], "binding id")
        key = (record["obligation_class"], record["obligation_id"])
        spec = spec_by_key.get(key)
        profile = record["profile_id"]
        kind = record["evidence_kind"]
        path_id = record["path_id"]
        artifact_id = require_id(
            record["artifact_id"], f"binding {identifier} artifact"
        )
        if identifier in result or spec is None:
            fail(f"duplicate or unknown qualification binding: {identifier}")
        if profile not in spec["profiles"] or kind not in profiles.get(profile, ()):
            fail(f"qualification binding profile or kind drifted: {identifier}")
        if spec["class"] not in binding_classes.get(kind, ()):
            fail(
                f"qualification evidence kind does not support the obligation class: {identifier}"
            )
        if path_id not in spec["paths"] or path_id not in paths:
            fail(f"qualification binding path drifted: {identifier}")
        triplet = (profile, kind, path_id)
        if triplet in triplets[key]:
            fail(f"duplicate qualification profile-kind-path binding: {identifier}")
        if record["source_identity_id"] != paths[path_id]["source_identity_id"]:
            fail(f"qualification binding source drifted: {identifier}")
        if record["statement_sha256"] != digest_bytes(
            spec["statement"].encode("utf-8")
        ):
            fail(f"qualification binding statement drifted: {identifier}")
        if string_list(record["tcb_ids"], f"binding {identifier} TCB") != list(TCB_IDS):
            fail(f"qualification binding TCB is incomplete: {identifier}")
        artifact = artifacts.get(artifact_id)
        if (
            artifact is None
            or artifact["kind"] != EVIDENCE_ARTIFACT_KINDS.get(kind)
            or artifact_id in bound_artifacts
        ):
            fail(
                f"qualification binding artifact is absent, reused, or mistyped: {identifier}"
            )
        payload = {
            name: item for name, item in record.items() if name != "binding_sha256"
        }
        if record["binding_sha256"] != canonical_digest(payload):
            fail(f"qualification binding identity mismatch: {identifier}")
        result[identifier] = record
        grouped[key].append(identifier)
        pairs[key].add((profile, kind))
        triplets[key].add(triplet)
        covered_paths[key].add(path_id)
        bound_artifacts.add(artifact_id)
        used.add(artifact_id)
    if tuple(result) != tuple(sorted(result)):
        fail("qualification binding roster is not canonical")
    for key, spec in spec_by_key.items():
        expected_pairs = {
            (profile, kind)
            for profile in spec["profiles"]
            for kind in profiles[profile]
            if spec["class"] in binding_classes[kind]
        }
        if pairs[key] != expected_pairs or covered_paths[key] != set(spec["paths"]):
            fail(f"qualification evidence closure is incomplete: {key[0]}:{key[1]}")
        grouped[key].sort()
    return result, grouped


def bound_artifacts(
    binding_ids: list[str], bindings: dict[str, dict[str, Any]], kind: str
) -> list[str]:
    return sorted(
        bindings[identifier]["artifact_id"]
        for identifier in binding_ids
        if bindings[identifier]["evidence_kind"] == kind
    )


def validate_obligations(
    value: Any,
    specs: list[dict[str, Any]],
    bindings: dict[str, dict[str, Any]],
    grouped: dict[tuple[str, str], list[str]],
    artifacts: dict[str, dict[str, Any]],
    used: set[str],
) -> str:
    if not isinstance(value, list) or len(value) != len(specs):
        fail(
            "qualification obligation roster must contain 33 roadmap and 17 assurance rows"
        )
    receipts: set[str] = set()
    referenced: set[str] = set()
    for record, spec in zip(value, specs, strict=True):
        common = {
            "closure_status",
            "evidence_binding_ids",
            "id",
            "obligation_class",
            "path_resolution_ids",
            "statement_sha256",
            "tcb_ids",
        }
        if spec["class"] == "Roadmap":
            expected_keys = common | {"assurance_dependencies", "receipt_artifact_id"}
        elif spec["status"] == "Proved":
            expected_keys = common | {"mutation_artifact_ids", "proof_artifact_ids"}
        elif spec["status"] == "Validated":
            expected_keys = common | {"validator_artifact_ids", "validator_tcb_ids"}
        else:
            expected_keys = common | {
                "nonclaim_tcb_ids",
                "rationale",
                "rationale_artifact_ids",
            }
        exact_keys(record, expected_keys, "qualification closure obligation")
        key = (record["obligation_class"], record["id"])
        expected_key = (spec["class"], spec["id"])
        if key != expected_key:
            fail(
                f"qualification obligation is missing, duplicated, or reordered: {expected_key}"
            )
        if record["closure_status"] != spec["status"]:
            fail(f"qualification status was weakened or promoted: {key[0]}:{key[1]}")
        if record["statement_sha256"] != digest_bytes(
            spec["statement"].encode("utf-8")
        ):
            fail(f"qualification statement drifted: {key[0]}:{key[1]}")
        if (
            string_list(record["tcb_ids"], f"closure {key} TCB") != list(TCB_IDS)
            or tuple(string_list(record["path_resolution_ids"], f"closure {key} paths"))
            != spec["paths"]
        ):
            fail(f"qualification path or TCB roster drifted: {key[0]}:{key[1]}")
        binding_ids = string_list(
            record["evidence_binding_ids"], f"closure {key} bindings"
        )
        if binding_ids != grouped[key] or referenced & set(binding_ids):
            fail(
                f"qualification binding roster is incomplete or reused: {key[0]}:{key[1]}"
            )
        referenced.update(binding_ids)
        if spec["class"] == "Roadmap":
            if (
                tuple(record["assurance_dependencies"])
                != spec["assurance_dependencies"]
            ):
                fail(f"qualification roadmap dependencies drifted: {spec['id']}")
            receipt_id = record["receipt_artifact_id"]
            receipt = artifacts.get(receipt_id)
            if receipt is None or receipt["kind"] != "QualificationReceipt":
                fail(f"qualification receipt is unavailable: {spec['id']}")
            receipts.add(receipt_id)
            used.add(receipt_id)
        elif spec["status"] == "Proved":
            proofs = bound_artifacts(binding_ids, bindings, "verus-theorem")
            mutations = bound_artifacts(binding_ids, bindings, "negative-mutation")
            if not proofs or record["proof_artifact_ids"] != proofs:
                fail(f"qualification Proved theorem roster is incomplete: {spec['id']}")
            if not mutations or record["mutation_artifact_ids"] != mutations:
                fail(
                    f"qualification Proved mutation roster is incomplete: {spec['id']}"
                )
        elif spec["status"] == "Validated":
            validators = bound_artifacts(binding_ids, bindings, "independent-validator")
            if not validators or record["validator_artifact_ids"] != validators:
                fail(f"qualification Validated evidence is incomplete: {spec['id']}")
            if record["validator_tcb_ids"] != list(TCB_IDS):
                fail(f"qualification validator TCB is incomplete: {spec['id']}")
        else:
            rationales = bound_artifacts(binding_ids, bindings, "unsupported-rationale")
            if (
                record["rationale"] != spec["statement"]
                or not rationales
                or string_list(
                    record["rationale_artifact_ids"],
                    f"Unsupported {spec['id']} rationale artifacts",
                )
                != rationales
                or record["nonclaim_tcb_ids"] != list(TCB_IDS)
            ):
                fail(f"qualification Unsupported boundary drifted: {spec['id']}")
    if referenced != set(bindings):
        fail("qualification does not bind every evidence record exactly once")
    if len(receipts) != 1:
        fail("qualification must use one canonical receipt")
    return next(iter(receipts))


def checker_registry(repo: Path) -> list[dict[str, Any]]:
    checker = repo / "proofs/check-m1-evidence-index.py"
    raw = read_bounded(checker, MAX_CHECKER_BYTES, "M1 evidence-index checker")
    try:
        tree = ast.parse(raw.decode("ascii"), filename=str(checker))
    except (UnicodeDecodeError, SyntaxError) as error:
        fail(f"cannot parse trusted-validator registry: {error}")
    assignments = [
        node.value
        for node in tree.body
        if isinstance(node, (ast.Assign, ast.AnnAssign))
        and (
            isinstance(node, ast.Assign)
            and any(
                isinstance(target, ast.Name) and target.id == "TRUSTED_VALIDATORS"
                for target in node.targets
            )
            or isinstance(node, ast.AnnAssign)
            and isinstance(node.target, ast.Name)
            and node.target.id == "TRUSTED_VALIDATORS"
        )
    ]
    if len(assignments) != 1:
        fail("checker must contain one literal trusted-validator registry")
    try:
        value = ast.literal_eval(assignments[0])
    except (ValueError, TypeError, SyntaxError) as error:
        fail(f"trusted-validator registry is not literal data: {error}")
    if not isinstance(value, dict) or tuple(value) != VALIDATOR_IDS:
        fail("trusted-validator registry is incomplete, reordered, or substituted")
    result: list[dict[str, Any]] = []
    paths: set[str] = set()
    protocols: set[str] = set()
    identities: set[str] = set()
    for evidence_kind, entry in value.items():
        if (
            not isinstance(entry, tuple)
            or len(entry) != 3
            or not isinstance(entry[0], str)
            or not isinstance(entry[1], str)
            or entry[2] is None
        ):
            fail(f"required validator is not source-pinned: {evidence_kind}")
        relative_name, protocol, source_sha256 = entry
        relative = safe_relative(relative_name, f"{evidence_kind} validator path")
        source_sha256 = require_sha256(
            source_sha256, f"{evidence_kind} validator source"
        )
        path = repo.joinpath(*relative.parts)
        raw = read_bounded(path, MAX_VALIDATOR_BYTES, f"{evidence_kind} validator")
        if digest_bytes(raw) != source_sha256:
            fail(f"trusted-validator source identity drifted: {evidence_kind}")
        if (
            relative_name in paths
            or protocol in protocols
            or source_sha256 in identities
        ):
            fail("trusted-validator path, protocol, or source identity is duplicated")
        paths.add(relative_name)
        protocols.add(protocol)
        identities.add(source_sha256)
        result.append(
            {
                "availability": "ExistingFoundation",
                "evidence_kind": evidence_kind,
                "path": relative_name,
                "protocol": protocol,
                "source_sha256": source_sha256,
            }
        )
    return result


def validate_tools(
    value: Any, repo: Path, validators: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TOOL_IDS):
        fail("qualification tool roster is incomplete")
    validator_by_id = {record["evidence_kind"]: record for record in validators}
    verus_version_raw = read_bounded(
        repo / "proofs/verus/VERUS_VERSION", 4096, "Verus version pin"
    )
    try:
        verus_version = verus_version_raw.decode("ascii").removesuffix("\n")
    except UnicodeDecodeError as error:
        fail(f"Verus version pin is not ASCII: {error}")
    expected = {
        "compiler.cargo": ("qualification-measured-binary", "1.97.1", None),
        "compiler.rustc": ("qualification-measured-binary", "1.97.1", None),
        "compiler.verus": (
            "pinned-proof-tool-closure",
            verus_version,
            digest_bytes(
                read_bounded(
                    repo / "proofs/verus/VERUS_CLOSURE_MANIFEST",
                    MAX_VALIDATOR_BYTES,
                    "Verus closure manifest",
                )
            ),
        ),
        "runtime.python": ("qualification-measured-binary", None, None),
        "validator.evidence-index": (
            "checker-owned-source",
            INDEX_FORMAT,
            digest_bytes(
                read_bounded(
                    repo / "proofs/check-m1-evidence-index.py",
                    MAX_CHECKER_BYTES,
                    "M1 evidence-index checker",
                )
            ),
        ),
        "validator.qualification-receipt": (
            "checker-owned-source",
            PROTOCOL,
            validator_by_id["qualification-receipt"]["source_sha256"],
        ),
    }
    identities: set[str] = set()
    for record, expected_id in zip(value, TOOL_IDS, strict=True):
        exact_keys(record, TOOL_KEYS, f"qualification tool {expected_id}")
        authority, version, identity = expected[expected_id]
        supplied_identity = require_sha256(
            record["identity_sha256"], f"{expected_id} identity"
        )
        if (
            record["id"] != expected_id
            or record["authority"] != authority
            or (version is not None and record["version"] != version)
            or (identity is not None and supplied_identity != identity)
            or supplied_identity in identities
        ):
            fail(f"qualification tool identity drifted: {expected_id}")
        require_ascii(record["version"], f"{expected_id} version")
        if (
            expected_id == "runtime.python"
            and PYTHON_VERSION.fullmatch(record["version"]) is None
        ):
            fail("qualification Python version is not exact")
        identities.add(supplied_identity)
    return value


def validate_environment(value: Any) -> dict[str, Any]:
    environment = exact_keys(value, ENVIRONMENT_KEYS, "qualification environment")
    device = exact_keys(environment["device"], DEVICE_KEYS, "qualification device")
    driver = exact_keys(environment["driver"], DRIVER_KEYS, "qualification driver")
    firmware = exact_keys(
        environment["firmware"], FIRMWARE_KEYS, "qualification firmware"
    )
    host = exact_keys(environment["host"], HOST_KEYS, "qualification host")
    rocm = exact_keys(environment["rocm"], ROCM_KEYS, "qualification ROCm")
    if (
        device["device_count"] != 1
        or device["marketing_name"] != "AMD Instinct MI300X"
        or device["processor"] != "gfx942"
        or device["vendor_id"] != "1002"
        or device["xnack"] != "disabled"
        or not isinstance(device["device_uuid"], str)
        or UUID.fullmatch(device["device_uuid"]) is None
        or not isinstance(device["pci_bdf"], str)
        or PCI_BDF.fullmatch(device["pci_bdf"]) is None
        or driver["name"] != "amdgpu"
        or host["machine"] != "x86_64"
    ):
        fail("qualification target device or host identity drifted")
    for record, fields in (
        (driver, ("module_sha256",)),
        (firmware, ("bundle_sha256",)),
        (host, ("kernel_sha256", "os_release_sha256")),
        (rocm, ("installation_sha256",)),
    ):
        for field in fields:
            require_sha256(record[field], f"environment {field}")
    for record, fields in (
        (driver, ("version",)),
        (firmware, ("package_version",)),
        (rocm, ("version",)),
    ):
        for field in fields:
            require_ascii(record[field], f"environment {field}")
    return environment


def parse_time(value: Any, description: str) -> datetime:
    if not isinstance(value, str) or UTC_TIMESTAMP.fullmatch(value) is None:
        fail(f"invalid {description}")
    try:
        parsed = datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(
            tzinfo=timezone.utc
        )
    except ValueError as error:
        fail(f"invalid {description}: {error}")
    return parsed


def expected_gate_rosters(
    index: dict[str, Any], receipt_id: str
) -> dict[str, tuple[list[str], list[str]]]:
    bindings = index["evidence_bindings"]
    by_kind = {
        kind: [record for record in bindings if record["evidence_kind"] == kind]
        for kind in EVIDENCE_ARTIFACT_KINDS
    }
    source_ids = sorted(
        record["source_closure_artifact_id"] for record in index["sources"]
    )
    nonreceipt_artifacts = sorted(
        record["id"] for record in index["artifacts"] if record["id"] != receipt_id
    )
    all_bindings = sorted(record["id"] for record in bindings)
    validator_artifacts = sorted(
        {record["artifact_id"] for record in bindings}
        | {record["artifact_id"] for record in index["tcb"]}
    )

    def select(*kinds: str) -> tuple[list[str], list[str]]:
        selected = [record for kind in kinds for record in by_kind[kind]]
        return (
            sorted(record["artifact_id"] for record in selected),
            sorted(record["id"] for record in selected),
        )

    return {
        "evidence-index": (nonreceipt_artifacts, all_bindings),
        "hardware": select("hardware-test"),
        "performance": select("performance-gate"),
        "proof": select("negative-mutation", "verus-theorem"),
        "quality": (source_ids, []),
        "source-closure": (source_ids, []),
        "validators": (validator_artifacts, all_bindings),
    }


def validate_gates(
    value: Any,
    index: dict[str, Any],
    receipt_id: str,
    run_start: datetime,
    run_end: datetime,
) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(GATE_IDS):
        fail("qualification gate roster is incomplete")
    expected = expected_gate_rosters(index, receipt_id)
    commands: set[str] = set()
    outputs: set[str] = set()
    for record, expected_id in zip(value, GATE_IDS, strict=True):
        exact_keys(record, GATE_KEYS, f"qualification gate {expected_id}")
        command = require_sha256(record["command_sha256"], f"{expected_id} command")
        output = require_sha256(record["output_sha256"], f"{expected_id} output")
        started = parse_time(record["started_at_utc"], f"{expected_id} start")
        finished = parse_time(record["finished_at_utc"], f"{expected_id} finish")
        artifacts, bindings = expected[expected_id]
        if (
            record["id"] != expected_id
            or record["result"] != "pass"
            or record["artifact_ids"] != artifacts
            or record["binding_ids"] != bindings
            or not (run_start <= started < finished <= run_end)
            or command in commands
            or output in outputs
        ):
            fail(
                f"qualification gate failed, skipped, replayed, or incomplete: {expected_id}"
            )
        commands.add(command)
        outputs.add(output)
    return value


def qualification_identity(transcript: dict[str, Any]) -> str:
    return canonical_digest(
        {
            "environment_identity_sha256": transcript["environment_identity_sha256"],
            "gate_roster_sha256": transcript["gate_roster_sha256"],
            "index_roster_sha256": transcript["index_roster_sha256"],
            "requirements_sha256": transcript["requirements_sha256"],
            "run_id": transcript["run_id"],
            "source_closure_sha256s": transcript["source_closure_sha256s"],
            "source_roster_sha256": transcript["source_roster_sha256"],
            "target_identity_sha256": transcript["target_identity_sha256"],
            "tcb_roster_sha256": transcript["tcb_roster_sha256"],
            "tool_roster_sha256": transcript["tool_roster_sha256"],
            "validator_roster_sha256": transcript["validator_roster_sha256"],
        }
    )


def revalidate_stable_inputs(
    repositories: dict[str, Path],
    sources: list[dict[str, Any]],
    artifacts: dict[str, dict[str, Any]],
    artifact_files: dict[str, Path],
) -> None:
    source_by_repository = {record["repository"]: record for record in sources}
    for repository_name, repository in repositories.items():
        commit, tree = git_identity(repository)
        expected = source_by_repository[repository_name]
        if commit != expected["commit"] or tree != expected["tree"]:
            fail(f"qualification source changed during validation: {repository_name}")
    for identifier, record in artifacts.items():
        raw = read_bounded(
            artifact_files[identifier],
            MAX_ARTIFACT_BYTES,
            f"final artifact identity {identifier}",
        )
        if len(raw) != record["size_bytes"] or digest_bytes(raw) != record["sha256"]:
            fail(f"qualification artifact changed during validation: {identifier}")


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT or context["subject"] != "qualification:M1":
        fail("qualification-receipt context format or subject drifted")
    validator_path = Path(__file__).absolute()
    if validator_path.resolve(strict=True) != validator_path:
        fail("qualification validator source path contains a symlink")
    repo = validator_path.parents[3]
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements, requirements_raw = load_canonical_json(
        requirements_path,
        MAX_REQUIREMENTS_BYTES,
        "M1 requirements manifest",
    )
    validate_requirements(requirements)
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("qualification requirements identity drifted")

    repositories_value = exact_keys(
        context["repository_absolute_paths"],
        REPOSITORY_KEYS,
        "qualification repository paths",
    )
    repositories: dict[str, Path] = {}
    for name in sorted(REPOSITORY_KEYS):
        raw = repositories_value[name]
        if not isinstance(raw, str) or not Path(raw).is_absolute():
            fail(f"qualification repository path is invalid: {name}")
        try:
            path = Path(raw).resolve(strict=True)
        except OSError as error:
            fail(f"qualification repository is unavailable: {name}: {error}")
        if path != Path(raw).absolute():
            fail(f"qualification repository path contains a symlink: {name}")
        repositories[name] = path
    if repositories["ferric"] != repo:
        fail("qualification Ferric repository does not own the validator")

    index = exact_keys(context["index"], INDEX_KEYS, "qualification evidence index")
    if (
        index["format"] != INDEX_FORMAT
        or index["requirements_sha256"] != requirements_sha256
        or context["sources"] != index["sources"]
        or context["tcb"] != index["tcb"]
    ):
        fail("qualification evidence-index identity drifted")

    artifact_context = exact_keys(
        context["artifact"], ARTIFACT_KEYS, "receipt artifact"
    )
    receipt_id = require_id(artifact_context["id"], "receipt artifact id")
    report_relative = safe_relative(artifact_context["path"], "receipt report path")
    expected_report_path = f"artifacts/{receipt_id}.qualification-receipt.json"
    if (
        artifact_context["kind"] != "QualificationReceipt"
        or report_relative.as_posix() != expected_report_path
        or not isinstance(context["artifact_absolute_path"], str)
    ):
        fail("qualification receipt artifact kind or path drifted")
    report_path = Path(context["artifact_absolute_path"])
    root = evidence_root(report_path, report_relative)
    reject_symlink_components(root, report_relative, "qualification receipt")

    artifacts, artifact_files = validate_artifacts(root, index["artifacts"])
    if artifacts.get(receipt_id) != artifact_context:
        fail("qualification receipt context is substituted or replayed")
    used: set[str] = set()
    sources, paths_by_source, closure_roster = validate_sources(
        index["sources"], requirements, repositories, artifacts, artifact_files, used
    )
    tcb = validate_tcb(index["tcb"], artifacts, used)
    paths = validate_paths(index["path_resolutions"], requirements, paths_by_source)
    specs = obligation_specs(requirements)
    bindings, grouped = validate_bindings(
        index["evidence_bindings"],
        requirements=requirements,
        specs=specs,
        paths=paths,
        artifacts=artifacts,
        used=used,
    )
    canonical_receipt_id = validate_obligations(
        index["obligations"], specs, bindings, grouped, artifacts, used
    )
    if canonical_receipt_id != receipt_id:
        fail("qualification receipt subject does not own every roadmap closure")
    if used != set(artifacts):
        fail("qualification artifact roster is partial or contains self-reports")

    validators = checker_registry(repo)
    report, report_raw = load_canonical_json(
        report_path, MAX_REPORT_BYTES, "qualification receipt"
    )
    report = exact_keys(report, REPORT_KEYS, "qualification receipt")
    if (
        len(report_raw) != artifact_context["size_bytes"]
        or digest_bytes(report_raw) != artifact_context["sha256"]
    ):
        fail("qualification receipt bytes drifted from their artifact identity")

    transcript_relative = safe_relative(
        report["transcript_relative_path"], "qualification transcript path"
    )
    expected_transcript_path = f"qualification-transcripts/{receipt_id}.json"
    if transcript_relative.as_posix() != expected_transcript_path:
        fail("qualification transcript path is noncanonical")
    transcript_path = reject_symlink_components(
        root, transcript_relative, "qualification transcript"
    )
    transcript, transcript_raw = load_canonical_json(
        transcript_path, MAX_TRANSCRIPT_BYTES, "qualification transcript"
    )
    transcript = exact_keys(transcript, TRANSCRIPT_KEYS, "qualification transcript")
    transcript_sha256 = require_sha256(
        report["transcript_sha256"], "qualification transcript SHA-256"
    )
    if (
        not isinstance(report["transcript_size_bytes"], int)
        or isinstance(report["transcript_size_bytes"], bool)
        or report["transcript_size_bytes"] != len(transcript_raw)
        or transcript_sha256 != digest_bytes(transcript_raw)
    ):
        fail("qualification transcript identity drifted")

    nonself_artifacts = [
        record for record in index["artifacts"] if record["id"] != receipt_id
    ]
    index_projection = {**index, "artifacts": nonself_artifacts}
    index_roster_sha256 = canonical_digest(index_projection)
    source_roster_sha256 = canonical_digest(sources)
    tcb_roster_sha256 = canonical_digest(tcb)
    validator_roster_sha256 = canonical_digest(validators)
    binding_roster_sha256 = canonical_digest(index["evidence_bindings"])
    path_roster_sha256 = canonical_digest(index["path_resolutions"])
    obligation_roster_sha256 = canonical_digest(index["obligations"])
    artifact_roster_sha256 = canonical_digest(nonself_artifacts)
    source_closure_sha256s = {
        record["id"]: record["source_closure_sha256"] for record in sources
    }
    tcb_identity_sha256s = {record["id"]: record["identity_sha256"] for record in tcb}

    exact_keys(transcript["target"], set(TARGET_VALUE), "qualification target")
    if transcript["target"] != TARGET_VALUE:
        fail("qualification target identity drifted")
    environment = validate_environment(transcript["environment"])
    tools = validate_tools(transcript["tools"], repo, validators)
    run_id = transcript["run_id"]
    if not isinstance(run_id, str) or UUID.fullmatch(run_id) is None:
        fail("qualification run identity is invalid")
    run_start = parse_time(transcript["started_at_utc"], "qualification start")
    run_end = parse_time(transcript["finished_at_utc"], "qualification finish")
    if run_start >= run_end:
        fail("qualification transcript has no positive run interval")
    gates = validate_gates(transcript["gates"], index, receipt_id, run_start, run_end)

    if (
        transcript["format"] != TRANSCRIPT_FORMAT
        or transcript["protocol"] != QUALIFICATION_PROTOCOL
        or transcript["milestone"] != "M1"
        or transcript["result"] != "pass"
        or transcript["all_required_gates_passed"] is not True
        or transcript["no_failed_gates"] is not True
        or transcript["no_skipped_gates"] is not True
        or transcript["requirements_sha256"] != requirements_sha256
        or transcript["index_roster_sha256"] != index_roster_sha256
        or transcript["source_roster_sha256"] != source_roster_sha256
        or transcript["source_closure_sha256s"] != source_closure_sha256s
        or transcript["tcb_roster_sha256"] != tcb_roster_sha256
        or transcript["tcb_identity_sha256s"] != tcb_identity_sha256s
        or transcript["validator_roster_sha256"] != validator_roster_sha256
        or transcript["target_identity_sha256"] != canonical_digest(TARGET_VALUE)
        or transcript["environment_identity_sha256"] != canonical_digest(environment)
        or transcript["tool_roster_sha256"] != canonical_digest(tools)
        or transcript["gate_roster_sha256"] != canonical_digest(gates)
        or transcript["qualification_id_sha256"] != qualification_identity(transcript)
    ):
        fail(
            "qualification transcript is partial, replayed, self-reported, or inconsistent"
        )

    for record in closure_roster:
        exact_keys(record, SOURCE_CLOSURE_KEYS, "source-closure roster")
    for record in validators:
        exact_keys(record, VALIDATOR_KEYS, "validator roster")
    receipt_descriptor = {
        "id": receipt_id,
        "kind": "QualificationReceipt",
        "path": expected_report_path,
    }
    if (
        report["format"] != REPORT_FORMAT
        or report["protocol"] != PROTOCOL
        or report["authority"] != AUTHORITY
        or report["nonclaim"] != NONCLAIM
        or report["milestone"] != "M1"
        or report["result"] != "pass"
        or report["target"] != TARGET
        or report["receipt_artifact"] != receipt_descriptor
        or report["requirements_sha256"] != requirements_sha256
        or report["requirements_roster_sha256"] != canonical_digest(requirements)
        or report["index_roster_sha256"] != index_roster_sha256
        or report["artifact_roster_sha256"] != artifact_roster_sha256
        or report["binding_roster_sha256"] != binding_roster_sha256
        or report["obligation_roster_sha256"] != obligation_roster_sha256
        or report["path_roster_sha256"] != path_roster_sha256
        or report["source_roster"] != sources
        or report["source_roster_sha256"] != source_roster_sha256
        or report["source_closure_roster"] != closure_roster
        or report["tcb_roster"] != tcb
        or report["tcb_roster_sha256"] != tcb_roster_sha256
        or report["validator_roster"] != validators
        or report["validator_roster_sha256"] != validator_roster_sha256
        or report["qualification_id_sha256"] != transcript["qualification_id_sha256"]
        or report["roadmap_count"] != 33
        or report["assurance_count"] != 17
        or report["path_count"] != 39
        or report["artifact_count"] != len(index["artifacts"])
        or report["binding_count"] != len(bindings)
        or report["validator_count"] != len(validators)
        or report["gate_ids"] != list(GATE_IDS)
    ):
        fail("qualification receipt content, roster, status, or identity drifted")
    revalidate_stable_inputs(repositories, sources, artifacts, artifact_files)


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("qualification-receipt validator protocol mismatch")
    context, context_payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(context_payload)}"
    )


if __name__ == "__main__":
    main()
