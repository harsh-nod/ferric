#!/usr/bin/env python3
"""Produce one source-authenticated M1 fe2o3-contract declaration."""

from __future__ import annotations

import ast
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
import tempfile
from typing import Any, BinaryIO, Callable, NoReturn


PLAN_FORMAT = "FERRIC-M1-EVIDENCE-PLAN-V1"
WORK_FORMAT = "FERRIC-M1-EVIDENCE-WORK-QUEUE-V1"
PLAN_AUTHORITY = "planning-only-no-evidence"
PLAN_NONCLAIM = (
    "This bundle allocates external M1 evidence work only. It is not an evidence "
    "index, qualification receipt, validation result, or M1 closure claim."
)
REPORT_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-V1"
REPORT_TARGET = "gfx942:xnack-"
REPORT_AUTHORITY = "contract-declaration-structure-only"
REPORT_NONCLAIM = (
    "This report authenticates an exact fe2o3 ContractSetV1 and Contracted "
    "property declaration only. A contract is not implementation or proof and "
    "grants no machine-refinement, load, launch, hardware, performance, or "
    "qualification authority."
)
CONTRACT_BODY_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-BODY-V1"
CONTRACT_SET_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-SET-V1"
CONTRACT_SET_SCHEMA = "fe2o3-proof-contracts::ContractSetV1"
CONTRACT_SET_SOURCE_PATH = "crates/fe2o3-proof-contracts/src/model.rs"
CONTRACT_SET_VALIDATION = "ContractSetV1::validate_closed-structural-only"
PROPERTY_KIND_NAMESPACE = "harsh-nod.ferric.m1.fe2o3-contract-binding.v1"
PROPERTY_KIND_CODE = 1
TCB_REPORT_FORMAT = "FERRIC-M1-TCB-REPORT-V1"
TCB_REPORT_AUTHORITY = "trusted-boundary-declaration-only"
TCB_REPORT_NONCLAIM = (
    "This report authenticates the declared M1 trusted boundary only. It does "
    "not establish component presence, version provenance, compiler or runtime "
    "correctness, hardware behavior, theorem truth, machine refinement, load, "
    "launch, performance, or qualification authority and closes no obligation."
)
FE2O3_CONTRACT_ROSTER_SHA256 = (
    "3a9caeaddd98840035fb55233aa1b3ccf53993313a955f95248a03f831cd45a9"
)
FE2O3_CONTRACT_TSV_SHA256 = (
    "04dee49ed87d5e3659abdf5478617188d45af3a278b4db958048f4598bfcf841"
)
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
ALLOCATION_SHA256 = "948ad3023df7ad4b1313ed865b54464f63b6bad9406f1510c85e60f9db055bd6"
TCB = (
    ("tcb.compiler", "Compiler"),
    ("tcb.hardware", "Hardware"),
    ("tcb.runtime", "Runtime"),
)
SOURCE_IDS = ("source.fe2o3", "source.ferric")
PROFILE_IDS = (
    "admission",
    "authentication",
    "composition",
    "kernel",
    "nonclaim",
    "qualification",
    "runtime",
)
EVIDENCE_KINDS = (
    "artifact-identity",
    "canonical-structure-check",
    "external-contract",
    "fe2o3-contract",
    "hardware-test",
    "independent-validator",
    "negative-mutation",
    "performance-gate",
    "tcb-report",
    "unsupported-rationale",
    "verus-theorem",
)
VALIDATOR_KINDS = (
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
PLAN_KEYS = {
    "allocation_sha256",
    "authority",
    "binding_slots",
    "counts",
    "fe2o3_pins",
    "finalization",
    "format",
    "nonclaim",
    "obligation_slots",
    "path_resolutions",
    "planner_sha256",
    "requirements",
    "source_closures",
    "sources",
    "target",
    "trusted_validators",
}
WORK_KEYS = {
    "authority",
    "counts",
    "format",
    "items",
    "nonclaim",
    "plan_path",
    "plan_sha256",
    "status",
}
SOURCE_KEYS = {
    "base_commit",
    "commit",
    "id",
    "repository",
    "source_closure_artifact_id",
    "source_closure_sha256",
    "tree",
}
TCB_REPORT_KEYS = {
    "authority",
    "component_roster",
    "evidence_kind",
    "format",
    "milestone",
    "nonclaim",
    "obligation_roster",
    "obligation_state",
    "path_roster",
    "profile_roster",
    "requirements_sha256",
    "source_roster",
    "subject_tcb_id",
    "subject_tcb_kind",
    "target",
    "tcb_structure_roster",
    "validator_roster",
}
REPORT_KEYS = {
    "assurance_property_declarations",
    "authority",
    "binding_sha256",
    "bound_source_identity_sha256",
    "contract_body_path",
    "contract_body_sha256",
    "contract_body_size_bytes",
    "contract_set_path",
    "contract_set_schema",
    "contract_set_sha256",
    "contract_set_size_bytes",
    "contract_set_source_path",
    "contract_set_validation",
    "contract_target",
    "evidence_kind",
    "format",
    "nonclaim",
    "obligation_class",
    "obligation_id",
    "obligation_state",
    "path_id",
    "path_resolution_sha256",
    "profile_id",
    "requirements_sha256",
    "source_identity_id",
    "source_roster_sha256",
    "statement_sha256",
    "tcb_identity_sha256s",
    "tcb_roster_sha256",
}
CONTRACT_BODY_KEYS = {
    "assurance_property_declarations",
    "binding_sha256",
    "format",
    "obligation_class",
    "obligation_id",
    "path_id",
    "profile_id",
    "requirements_sha256",
    "source_roster_sha256",
    "statement_sha256",
    "target",
    "tcb_roster_sha256",
}
CONTRACT_SET_KEYS = {
    "correspondences",
    "format",
    "obligations",
    "properties",
    "schema",
    "schema_source_path",
    "trusted_computing_base",
    "validation",
}
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
MAX_JSON_BYTES = 16_000_000
MAX_FILE_BYTES = 64_000_000


JsonObject = dict[str, Any]
HeldFile = tuple[str, BinaryIO, os.stat_result, bytes, str]
HeldDirectoryFiles = tuple[int, list[HeldFile]]
HeldDirectoryComponent = tuple[int, str, int, os.stat_result, str]
AbsoluteDirectoryCustody = tuple[int, list[HeldDirectoryComponent], str]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def file_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def directory_binding(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
    )


def single_component(name: str, description: str) -> str:
    if (
        not isinstance(name, str)
        or not name
        or name in {".", ".."}
        or "/" in name
        or "\0" in name
    ):
        fail(f"{description} must be a single path component")
    return name


def directory_open_flags() -> int:
    required = ("O_NOFOLLOW", "O_DIRECTORY", "O_CLOEXEC")
    if any(not hasattr(os, name) for name in required):
        fail("descriptor-relative custody requires O_NOFOLLOW/O_DIRECTORY/O_CLOEXEC")
    return os.O_RDONLY | os.O_NOFOLLOW | os.O_DIRECTORY | os.O_CLOEXEC


def open_directory_component_at(
    parent_fd: int, name: str, description: str
) -> HeldDirectoryComponent:
    component = single_component(name, description)
    descriptor = -1
    try:
        before = os.stat(component, dir_fd=parent_fd, follow_symlinks=False)
        descriptor = os.open(component, directory_open_flags(), dir_fd=parent_fd)
        opened = os.fstat(descriptor)
    except OSError as error:
        if descriptor >= 0:
            os.close(descriptor)
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(before.st_mode)
        or not stat.S_ISDIR(before.st_mode)
        or directory_binding(before) != directory_binding(opened)
    ):
        os.close(descriptor)
        fail(f"{description} must be a held nonsymlink directory")
    return parent_fd, component, descriptor, opened, description


def revalidate_directory_component(
    held: HeldDirectoryComponent, *, private: bool = False
) -> None:
    parent_fd, name, descriptor, authenticated, description = held
    try:
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        stat.S_ISLNK(named.st_mode)
        or directory_binding(authenticated) != directory_binding(opened)
        or directory_binding(opened) != directory_binding(named)
    ):
        fail(f"{description} was replaced after it was opened")
    if private:
        verify_private_directory(opened, description)


def authenticate_absolute_directory(
    path: Path, description: str, *, private: bool = False
) -> AbsoluteDirectoryCustody:
    absolute = path.absolute()
    if not absolute.is_absolute() or not absolute.parts or absolute.parts[0] != "/":
        fail(f"{description} must resolve from the filesystem root")
    components = absolute.parts[1:]
    for component in components:
        single_component(component, description)
    try:
        root_fd = os.open("/", directory_open_flags())
    except OSError as error:
        fail(f"filesystem root is unavailable for {description}: {error}")
    chain: list[HeldDirectoryComponent] = []
    parent_fd = root_fd
    try:
        for ordinal, component in enumerate(components, 1):
            held = open_directory_component_at(
                parent_fd, component, f"{description} component {ordinal}"
            )
            chain.append(held)
            parent_fd = held[2]
        final = os.fstat(chain[-1][2] if chain else root_fd)
        if private:
            verify_private_directory(final, description)
        return root_fd, chain, description
    except BaseException:
        for held in reversed(chain):
            os.close(held[2])
        os.close(root_fd)
        raise


def directory_custody_fd(custody: AbsoluteDirectoryCustody) -> int:
    root_fd, chain, _ = custody
    return chain[-1][2] if chain else root_fd


def revalidate_absolute_directory(
    custody: AbsoluteDirectoryCustody, *, private: bool = False
) -> None:
    root_fd, chain, description = custody
    try:
        root_metadata = os.fstat(root_fd)
    except OSError as error:
        fail(f"cannot revalidate filesystem root for {description}: {error}")
    if not stat.S_ISDIR(root_metadata.st_mode):
        fail(f"filesystem root changed for {description}")
    for ordinal, held in enumerate(chain):
        revalidate_directory_component(
            held, private=private and ordinal == len(chain) - 1
        )


def close_absolute_directory(custody: AbsoluteDirectoryCustody) -> None:
    root_fd, chain, _ = custody
    for held in reversed(chain):
        os.close(held[2])
    os.close(root_fd)


def open_regular(path: Path, description: str) -> tuple[BinaryIO, os.stat_result]:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before = path.lstat()
        descriptor = os.open(path, flags)
        source = os.fdopen(descriptor, "rb")
        opened = os.fstat(source.fileno())
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(before.st_mode)
        or not stat.S_ISREG(before.st_mode)
        or not stat.S_ISREG(opened.st_mode)
        or (before.st_dev, before.st_ino) != (opened.st_dev, opened.st_ino)
    ):
        source.close()
        fail(f"{description} must be a stable regular nonsymlink file")
    return source, opened


def open_regular_at(
    directory_fd: int, name: str, description: str, *, writable: bool = False
) -> tuple[BinaryIO, os.stat_result]:
    flags = (os.O_RDWR if writable else os.O_RDONLY) | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        descriptor = os.open(name, flags, dir_fd=directory_fd)
        source = os.fdopen(descriptor, "r+b" if writable else "rb")
        opened = os.fstat(source.fileno())
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(before.st_mode)
        or not stat.S_ISREG(before.st_mode)
        or not stat.S_ISREG(opened.st_mode)
        or (before.st_dev, before.st_ino) != (opened.st_dev, opened.st_ino)
    ):
        source.close()
        fail(f"{description} must be a stable regular nonsymlink file")
    return source, opened


def read_regular(path: Path, limit: int, description: str) -> bytes:
    source, before = open_regular(path, description)
    try:
        if before.st_size <= 0 or before.st_size > limit:
            fail(f"{description} size is outside the admitted bound")
        raw = source.read(limit + 1)
        after = os.fstat(source.fileno())
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    finally:
        source.close()
    if (
        len(raw) != before.st_size
        or len(raw) > limit
        or file_identity(before) != file_identity(after)
    ):
        fail(f"{description} changed while it was read")
    return raw


def read_regular_at(
    directory_fd: int, name: str, limit: int, description: str
) -> bytes:
    source, before = open_regular_at(directory_fd, name, description)
    try:
        if before.st_size <= 0 or before.st_size > limit:
            fail(f"{description} size is outside the admitted bound")
        raw = source.read(limit + 1)
        after = os.fstat(source.fileno())
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    finally:
        source.close()
    if (
        len(raw) != before.st_size
        or len(raw) > limit
        or file_identity(before) != file_identity(after)
    ):
        fail(f"{description} changed while it was read")
    return raw


def authenticate_held_file_at(
    directory_fd: int, name: str, expected: bytes, description: str
) -> HeldFile:
    source, before = open_regular_at(directory_fd, name, description)
    try:
        if (
            before.st_size != len(expected)
            or stat.S_IMODE(before.st_mode) != 0o600
            or before.st_uid != os.geteuid()
        ):
            fail(f"{description} is not the exact owner-private expected file")
        raw = source.read(len(expected) + 1)
        after = os.fstat(source.fileno())
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if (
            raw != expected
            or file_identity(before) != file_identity(after)
            or file_identity(after) != file_identity(named)
        ):
            fail(f"{description} changed while it was authenticated")
        return name, source, after, expected, description
    except OSError as error:
        source.close()
        fail(f"cannot authenticate {description}: {error}")
    except BaseException:
        source.close()
        raise


def revalidate_held_file(directory_fd: int, held: HeldFile) -> None:
    name, source, authenticated, expected, description = held
    try:
        before = os.fstat(source.fileno())
        source.seek(0)
        raw = source.read(len(expected) + 1)
        after = os.fstat(source.fileno())
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        stat.S_ISLNK(named.st_mode)
        or raw != expected
        or file_identity(authenticated) != file_identity(before)
        or file_identity(before) != file_identity(after)
        or file_identity(after) != file_identity(named)
    ):
        fail(f"{description} changed after authentication")


def digest_file(path: Path) -> str:
    return digest_bytes(read_regular(path, MAX_FILE_BYTES, str(path)))


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def canonical_digest(value: Any) -> str:
    return digest_bytes(
        json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
    )


def exact_keys(value: Any, expected: set[str], description: str) -> JsonObject:
    if not isinstance(value, dict) or set(value) != expected:
        fail(f"{description} fields drifted")
    return value


def read_canonical_json(path: Path, description: str) -> tuple[JsonObject, bytes]:
    def unique(pairs: list[tuple[str, Any]]) -> JsonObject:
        value: JsonObject = {}
        for key, item in pairs:
            if key in value:
                fail(f"{description} contains a duplicate JSON key: {key}")
            value[key] = item
        return value

    try:
        raw = read_regular(path, MAX_JSON_BYTES, description)
        value = json.loads(raw, object_pairs_hook=unique)
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot read {description}: {error}")
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"{description} is not a canonical JSON object")
    return value, raw


def read_canonical_json_at(
    directory_fd: int, name: str, description: str
) -> tuple[JsonObject, bytes]:
    def unique(pairs: list[tuple[str, Any]]) -> JsonObject:
        value: JsonObject = {}
        for key, item in pairs:
            if key in value:
                fail(f"{description} contains a duplicate JSON key: {key}")
            value[key] = item
        return value

    try:
        raw = read_regular_at(directory_fd, name, MAX_JSON_BYTES, description)
        value = json.loads(raw, object_pairs_hook=unique)
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot read {description}: {error}")
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"{description} is not a canonical JSON object")
    return value, raw


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


def safe_relative(value: Any, description: str) -> PurePosixPath:
    if not isinstance(value, str) or not value or len(value) > 4096:
        fail(f"invalid {description}")
    relative = PurePosixPath(value)
    if (
        relative.is_absolute()
        or relative.as_posix() != value
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        fail(f"unsafe {description}")
    return relative


def verify_private_directory(metadata: os.stat_result, description: str) -> None:
    if (
        not stat.S_ISDIR(metadata.st_mode)
        or stat.S_IMODE(metadata.st_mode) != 0o700
        or metadata.st_uid != os.geteuid()
    ):
        fail(f"{description} must be an exact owner-private 0700 directory")


def open_directory(path: Path, description: str) -> int:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before = path.lstat()
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(before.st_mode)
        or not stat.S_ISDIR(opened.st_mode)
        or directory_binding(before) != directory_binding(opened)
    ):
        os.close(descriptor)
        fail(f"{description} must be a held nonsymlink directory")
    return descriptor


def revalidate_directory(path: Path, descriptor: int, description: str) -> None:
    try:
        named = path.lstat()
        held = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if stat.S_ISLNK(named.st_mode) or directory_binding(named) != directory_binding(
        held
    ):
        fail(f"{description} was replaced after it was opened")


def open_private_directory(path: Path, description: str) -> int:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before = path.lstat()
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if stat.S_ISLNK(before.st_mode) or directory_binding(before) != directory_binding(
        opened
    ):
        os.close(descriptor)
        fail(f"{description} must be a held nonsymlink directory")
    verify_private_directory(opened, description)
    return descriptor


def open_private_directory_at(parent_fd: int, name: str, description: str) -> int:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        descriptor = os.open(name, flags, dir_fd=parent_fd)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if stat.S_ISLNK(before.st_mode) or directory_binding(before) != directory_binding(
        opened
    ):
        os.close(descriptor)
        fail(f"{description} must be a held nonsymlink directory")
    verify_private_directory(opened, description)
    return descriptor


def revalidate_directory_path(path: Path, descriptor: int, description: str) -> None:
    try:
        named = path.lstat()
        held = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if stat.S_ISLNK(named.st_mode) or directory_binding(named) != directory_binding(
        held
    ):
        fail(f"{description} was replaced after it was opened")
    verify_private_directory(held, description)


def revalidate_child_directory(
    parent_fd: int, name: str, descriptor: int, description: str
) -> None:
    try:
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        held = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if stat.S_ISLNK(named.st_mode) or directory_binding(named) != directory_binding(
        held
    ):
        fail(f"{description} was replaced after it was opened")
    verify_private_directory(held, description)


def literal_assignment(path: Path, name: str) -> Any:
    try:
        source = read_regular(path, MAX_FILE_BYTES, str(path)).decode("ascii")
        tree = ast.parse(source, filename=str(path))
    except (UnicodeError, SyntaxError) as error:
        fail(f"cannot parse {path}: {error}")
    matches = []
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(
            isinstance(target, ast.Name) and target.id == name
            for target in node.targets
        ):
            matches.append(node.value)
        elif (
            isinstance(node, ast.AnnAssign)
            and isinstance(node.target, ast.Name)
            and node.target.id == name
        ):
            matches.append(node.value)
    if len(matches) != 1:
        fail(f"{path} must define exactly one literal {name}")
    try:
        return ast.literal_eval(matches[0])
    except (ValueError, TypeError, SyntaxError) as error:
        fail(f"{path} {name} is not literal data: {error}")


def run(
    arguments: list[str],
    description: str,
    *,
    cwd: Path,
    timeout_seconds: int = 120,
) -> str:
    try:
        result = subprocess.run(
            arguments,
            cwd=cwd,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=timeout_seconds,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"{description} could not run: {error}")
    if result.returncode != 0:
        fail(f"{description} failed (status {result.returncode}):\n{result.stdout}")
    return result.stdout.strip()


def repository_identity(repository: Path, description: str) -> tuple[str, str]:
    status = run(
        [
            "git",
            "-C",
            str(repository),
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
        ],
        f"inspect {description} worktree",
        cwd=repository,
    )
    if status:
        fail(f"{description} repository must be an exact clean worktree")
    commit = run(
        ["git", "-C", str(repository), "rev-parse", "--verify", "HEAD"],
        f"resolve {description} commit",
        cwd=repository,
    )
    tree = run(
        ["git", "-C", str(repository), "rev-parse", "--verify", "HEAD^{tree}"],
        f"resolve {description} tree",
        cwd=repository,
    )
    return (
        require_git_id(commit, f"{description} commit"),
        require_git_id(tree, f"{description} tree"),
    )


def validate_requirements(requirements: JsonObject) -> None:
    expected_keys = {
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
    exact_keys(requirements, expected_keys, "M1 requirements")
    if (
        requirements["format"] != "ferric.m1-requirements.v1"
        or requirements["milestone"] != "M1"
        or tuple(requirements["evidence_kinds"]) != EVIDENCE_KINDS
        or len(requirements["roadmap_requirements"]) != 33
        or len(requirements["assurance_properties"]) != 17
        or len(requirements["path_obligations"]) != 39
    ):
        fail("M1 requirements cardinality, format, or vocabulary drifted")
    profiles = requirements["evidence_profiles"]
    if [record.get("id") for record in profiles] != list(PROFILE_IDS):
        fail("M1 evidence profile roster drifted")
    if any(
        record.get("obligation_state") != "Open"
        for group in (
            requirements["roadmap_requirements"],
            requirements["assurance_properties"],
            requirements["path_obligations"],
        )
        for record in group
    ):
        fail("fe2o3-contract production requires every M1 obligation to remain Open")
    for key in (
        "m0_contracts_commit",
        "m1_upstream_base_commit",
        "m1_upstream_base_tree",
    ):
        require_git_id(requirements[key], f"requirements {key}")


def projected_obligations(requirements: JsonObject) -> list[JsonObject]:
    rows = [
        {
            "class": "Roadmap",
            "id": record["id"],
            "path_ids": record["path_obligations"],
            "profile_ids": record["evidence_profiles"],
            "statement_sha256": digest_bytes(record["title"].encode("utf-8")),
            "status": record["obligation_state"],
        }
        for record in requirements["roadmap_requirements"]
    ]
    rows.extend(
        {
            "class": "Assurance",
            "id": record["name"],
            "path_ids": record["path_obligations"],
            "profile_ids": record["evidence_profiles"],
            "statement_sha256": digest_bytes(record["boundary"].encode("utf-8")),
            "status": record["obligation_state"],
        }
        for record in requirements["assurance_properties"]
    )
    if len(rows) != 50 or len({(row["class"], row["id"]) for row in rows}) != 50:
        fail("projected M1 obligation roster drifted")
    return rows


def projected_paths(requirements: JsonObject) -> list[JsonObject]:
    rows = [
        {
            "availability": record["availability"],
            "id": record["id"],
            "path": record["path"],
            "repository": record["repository"],
            "source_identity_id": f"source.{record['repository']}",
            "status": record["obligation_state"],
        }
        for record in requirements["path_obligations"]
    ]
    if len(rows) != 39 or len({row["id"] for row in rows}) != 39:
        fail("projected M1 path roster drifted")
    for row in rows:
        safe_relative(row["path"], f"path obligation {row['id']}")
        if row["repository"] not in {"fe2o3", "ferric"}:
            fail(f"M1 path repository drifted: {row['id']}")
    return rows


def projected_profiles(requirements: JsonObject) -> list[JsonObject]:
    return [
        {"evidence_kinds": record["kinds"], "id": record["id"]}
        for record in requirements["evidence_profiles"]
    ]


def trusted_validators(ferric: Path) -> tuple[list[JsonObject], list[JsonObject]]:
    checker = ferric / "proofs/check-m1-evidence-index.py"
    registry = literal_assignment(checker, "TRUSTED_VALIDATORS")
    if not isinstance(registry, dict) or tuple(registry) != VALIDATOR_KINDS:
        fail("checker-owned trusted-validator registry drifted")
    plan_rows: list[JsonObject] = []
    report_rows: list[JsonObject] = []
    for evidence_kind, value in registry.items():
        if (
            not isinstance(value, tuple)
            or len(value) != 3
            or not all(isinstance(item, str) for item in value)
        ):
            fail(f"trusted validator record is malformed: {evidence_kind}")
        relative, protocol, expected_sha256 = value
        path = ferric.joinpath(*safe_relative(relative, "validator path").parts)
        actual_sha256 = digest_file(path)
        if actual_sha256 != require_sha256(expected_sha256, "validator source pin"):
            fail(f"trusted validator source pin drifted: {evidence_kind}")
        plan_rows.append(
            {
                "evidence_kind": evidence_kind,
                "path": relative,
                "protocol": protocol,
                "source_sha256": expected_sha256,
            }
        )
        report_rows.append(
            {
                "availability": "ExistingFoundation",
                "evidence_kind": evidence_kind,
                "path": relative,
                "protocol": protocol,
                "source_sha256": expected_sha256,
            }
        )
    return plan_rows, report_rows


def component(
    identifier: str,
    kind: str,
    version: str,
    status: str,
    authority: str,
    identity_payload: Any,
) -> JsonObject:
    return {
        "authority": authority,
        "id": identifier,
        "identity_sha256": canonical_digest(identity_payload),
        "kind": kind,
        "status": status,
        "version": version,
    }


def component_roster(ferric: Path, sources: list[JsonObject]) -> list[JsonObject]:
    by_id = {record["id"]: record for record in sources}
    rust_toolchain = digest_file(ferric / "rust-toolchain.toml")
    try:
        verus_version = (
            read_regular(
                ferric / "proofs/verus/VERUS_VERSION", 4096, "Verus version pin"
            )
            .decode("ascii")
            .removesuffix("\n")
        )
    except UnicodeError as error:
        fail(f"cannot read Verus version pin: {error}")
    if not verus_version or "\n" in verus_version:
        fail("Verus version pin is not one canonical line")
    verus_closure = digest_file(ferric / "proofs/verus/VERUS_CLOSURE_MANIFEST")
    rows = [
        component(
            "compiler.amdgpu-linker",
            "Compiler",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["compiler.amdgpu-linker", "qualification-bound-external", REPORT_TARGET],
        ),
        component(
            "compiler.llvm-amdgpu",
            "Compiler",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["compiler.llvm-amdgpu", "qualification-bound-external", REPORT_TARGET],
        ),
        component(
            "compiler.rust",
            "Compiler",
            "1.97.1",
            "Pinned",
            "source-configuration-only",
            ["compiler.rust", "1.97.1", rust_toolchain],
        ),
        component(
            "compiler.verus",
            "Compiler",
            verus_version,
            "Pinned",
            "proof-tool-source-closure",
            ["compiler.verus", verus_version, verus_closure],
        ),
        component(
            "hardware.gfx942",
            "Hardware",
            REPORT_TARGET,
            "Contracted",
            "single-device-target-only",
            ["hardware.gfx942", REPORT_TARGET, "one-physical-device"],
        ),
        component(
            "runtime.amdgpu-firmware",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.amdgpu-firmware", "qualification-bound-external", REPORT_TARGET],
        ),
        component(
            "runtime.amdgpu-kernel-driver",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            [
                "runtime.amdgpu-kernel-driver",
                "qualification-bound-external",
                REPORT_TARGET,
            ],
        ),
        component(
            "runtime.fe2o3",
            "Runtime",
            by_id["source.fe2o3"]["commit"],
            "SourceBound",
            "exact-source-identity",
            by_id["source.fe2o3"],
        ),
        component(
            "runtime.ferric",
            "Runtime",
            by_id["source.ferric"]["commit"],
            "SourceBound",
            "exact-source-identity",
            by_id["source.ferric"],
        ),
        component(
            "runtime.hsa",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.hsa", "qualification-bound-external", REPORT_TARGET],
        ),
        component(
            "runtime.posix-host",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "os-filesystem-process-supervision",
            ["runtime.posix-host", "qualification-bound-external"],
        ),
        component(
            "runtime.python",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "validator-interpreter-and-stdlib",
            ["runtime.python", "qualification-bound-external"],
        ),
    ]
    if [row["id"] for row in rows] != sorted(row["id"] for row in rows):
        fail("internal TCB component roster is not canonical")
    if len({row["identity_sha256"] for row in rows}) != len(rows):
        fail("internal TCB component identities are not unique")
    return rows


def validate_sources(
    ferric: Path,
    fe2o3: Path,
    plan_fd: int,
    plan: JsonObject,
    requirements: JsonObject,
) -> list[JsonObject]:
    sources = plan["sources"]
    if not isinstance(sources, list) or len(sources) != 2:
        fail("M1 plan source roster is incomplete")
    repositories = {"fe2o3": fe2o3, "ferric": ferric}
    bases = {
        "source.fe2o3": requirements["m1_upstream_base_commit"],
        "source.ferric": FERRIC_BASE_COMMIT,
    }
    closures = plan["source_closures"]
    if not isinstance(closures, list) or len(closures) != 2:
        fail("M1 plan source-closure roster is incomplete")
    closure_dir_fd = open_private_directory_at(
        plan_fd, "source-closures", "M1 source-closure directory"
    )
    by_artifact: dict[str, JsonObject] = {}
    for closure in closures:
        exact_keys(closure, {"artifact", "file_count", "producer"}, "source closure")
        artifact = exact_keys(
            closure["artifact"],
            {"id", "kind", "path", "sha256", "size_bytes"},
            "source-closure artifact",
        )
        by_artifact[artifact["id"]] = closure
    result: list[JsonObject] = []
    for expected_id, record in zip(SOURCE_IDS, sources, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source {expected_id}")
        repository_name = expected_id.removeprefix("source.")
        if (
            record["id"] != expected_id
            or record["repository"] != repository_name
            or record["base_commit"] != bases[expected_id]
        ):
            fail(f"source identity order, repository, or base drifted: {expected_id}")
        actual = repository_identity(repositories[repository_name], repository_name)
        if (record["commit"], record["tree"]) != actual:
            fail(f"source commit or tree drifted: {expected_id}")
        closure_id = record["source_closure_artifact_id"]
        expected_closure_id = f"artifact.{expected_id}"
        if closure_id != expected_closure_id or closure_id not in by_artifact:
            fail(f"source closure artifact drifted: {expected_id}")
        closure = by_artifact[closure_id]
        artifact = closure["artifact"]
        relative = safe_relative(artifact["path"], "source closure path")
        expected_relative = f"source-closures/{expected_id}.records"
        if (
            artifact["kind"] != "SourceClosure"
            or relative.as_posix() != expected_relative
            or record["source_closure_sha256"] != artifact["sha256"]
        ):
            fail(f"source closure declaration drifted: {expected_id}")
        if len(relative.parts) != 2 or relative.parts[0] != "source-closures":
            fail(f"source closure path escaped its directory: {expected_id}")
        raw = read_regular_at(
            closure_dir_fd,
            relative.parts[1],
            MAX_FILE_BYTES,
            f"source closure {expected_id}",
        )
        if (
            not raw
            or digest_bytes(raw) != require_sha256(artifact["sha256"], "source closure")
            or len(raw) != artifact["size_bytes"]
            or len(raw.splitlines()) != closure["file_count"]
        ):
            fail(f"source closure bytes drifted: {expected_id}")
        producer = exact_keys(
            closure["producer"], {"command", "source_sha256"}, "source-closure producer"
        )
        measure = ferric / "proofs/m1/evidence/measure-source-closure.py"
        if producer["command"] != [
            "python3",
            "-I",
            "proofs/m1/evidence/measure-source-closure.py",
            repository_name.upper() + "_REPO",
            expected_relative,
        ] or producer["source_sha256"] != digest_file(measure):
            fail(f"source closure producer drifted: {expected_id}")
        result.append(record)
    if set(by_artifact) != {f"artifact.{identifier}" for identifier in SOURCE_IDS}:
        fail("M1 plan contains an unknown source closure")
    revalidate_child_directory(
        plan_fd, "source-closures", closure_dir_fd, "M1 source-closure directory"
    )
    os.close(closure_dir_fd)
    return result


def authenticate_source_closures(plan_fd: int, plan: JsonObject) -> HeldDirectoryFiles:
    directory_fd = open_private_directory_at(
        plan_fd, "source-closures", "M1 source-closure directory"
    )
    artifacts = {
        closure["artifact"]["id"]: closure["artifact"]
        for closure in plan["source_closures"]
    }
    held: list[HeldFile] = []
    try:
        for source_id in SOURCE_IDS:
            artifact = artifacts[f"artifact.{source_id}"]
            name = f"{source_id}.records"
            raw = read_regular_at(
                directory_fd, name, MAX_FILE_BYTES, f"source closure {source_id}"
            )
            if (
                artifact["path"] != f"source-closures/{name}"
                or len(raw) != artifact["size_bytes"]
                or digest_bytes(raw) != artifact["sha256"]
            ):
                fail(f"source closure identity drifted: {source_id}")
            held.append(
                authenticate_held_file_at(
                    directory_fd, name, raw, f"source closure {source_id}"
                )
            )
        revalidate_child_directory(
            plan_fd,
            "source-closures",
            directory_fd,
            "M1 source-closure directory",
        )
    except BaseException:
        for _, source, _, _, _ in held:
            source.close()
        os.close(directory_fd)
        raise
    return directory_fd, held


def revalidate_source_closures(plan_fd: int, custody: HeldDirectoryFiles) -> None:
    directory_fd, held = custody
    revalidate_child_directory(
        plan_fd,
        "source-closures",
        directory_fd,
        "M1 source-closure directory",
    )
    for file_custody in held:
        revalidate_held_file(directory_fd, file_custody)


def close_source_closures(custody: HeldDirectoryFiles) -> None:
    directory_fd, held = custody
    for _, source, _, _, _ in held:
        source.close()
    os.close(directory_fd)


def source_identity_map(sources: list[JsonObject]) -> dict[str, tuple[str, str]]:
    return {
        record["repository"]: (record["commit"], record["tree"]) for record in sources
    }


def revalidate_repository_identities(
    repositories: dict[str, tuple[Path, AbsoluteDirectoryCustody]],
    expected: dict[str, tuple[str, str]],
) -> None:
    if set(repositories) != set(expected):
        fail("authenticated source repository roster drifted")
    for name in sorted(repositories):
        path, custody = repositories[name]
        if repository_identity(path, name) != expected[name]:
            fail(f"authenticated source commit or tree changed: {name}")
        revalidate_absolute_directory(custody)


def entry_exists_at(directory_fd: int, name: str) -> bool:
    try:
        os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except FileNotFoundError:
        return False
    except OSError as error:
        fail(f"cannot inspect M1 plan entry {name}: {error}")
    return True


def rederive_candidate_plan(
    ferric: Path,
    fe2o3: Path,
    plan_fd: int,
    candidate_plan: bytes,
    candidate_queue: bytes,
) -> None:
    with tempfile.TemporaryDirectory(
        prefix="ferric-m1-fe2o3-contract-planner-replay-"
    ) as raw:
        reproduced = Path(raw) / "plan"
        run(
            [
                sys.executable,
                "-I",
                str(ferric / "proofs/m1-qualification/planner.py"),
                str(ferric),
                str(fe2o3),
                str(reproduced),
            ],
            "rederive complete M1 evidence plan",
            cwd=ferric,
            timeout_seconds=300,
        )
        reproduced_plan = read_regular(
            reproduced / "plan.json", MAX_JSON_BYTES, "rederived M1 evidence plan"
        )
        reproduced_queue = read_regular(
            reproduced / "missing-work.json",
            MAX_JSON_BYTES,
            "rederived M1 evidence work queue",
        )
        if reproduced_plan != candidate_plan or reproduced_queue != candidate_queue:
            fail(
                "M1 candidate plan or complete work queue differs from exact rederivation"
            )

        closure_fd = open_private_directory_at(
            plan_fd, "source-closures", "candidate source-closure directory"
        )
        try:
            for source_id in SOURCE_IDS:
                name = f"{source_id}.records"
                candidate = read_regular_at(
                    closure_fd, name, MAX_FILE_BYTES, f"candidate {source_id} closure"
                )
                expected = read_regular(
                    reproduced / "source-closures" / name,
                    MAX_FILE_BYTES,
                    f"rederived {source_id} closure",
                )
                if candidate != expected:
                    fail(
                        f"M1 candidate source closure differs from rederivation: {source_id}"
                    )
            revalidate_child_directory(
                plan_fd,
                "source-closures",
                closure_fd,
                "candidate source-closure directory",
            )
        finally:
            os.close(closure_fd)


def validate_plan(
    ferric: Path, fe2o3: Path, plan_fd: int, *, replay: bool = True
) -> tuple[
    JsonObject,
    JsonObject,
    JsonObject,
    list[JsonObject],
    list[JsonObject],
    bytes,
    bytes,
]:
    plan, plan_raw = read_canonical_json_at(plan_fd, "plan.json", "M1 evidence plan")
    queue, queue_raw = read_canonical_json_at(
        plan_fd, "missing-work.json", "M1 evidence work queue"
    )
    exact_keys(plan, PLAN_KEYS, "M1 evidence plan")
    exact_keys(queue, WORK_KEYS, "M1 evidence work queue")
    if (
        plan["format"] != PLAN_FORMAT
        or plan["authority"] != PLAN_AUTHORITY
        or plan["nonclaim"] != PLAN_NONCLAIM
        or plan["target"] != REPORT_TARGET
        or plan["allocation_sha256"] != ALLOCATION_SHA256
        or plan["counts"]
        != {
            "assurance_binding_slots": 186,
            "binding_slots": 354,
            "obligation_slots": 50,
            "path_resolutions": 39,
            "roadmap_binding_slots": 168,
            "source_closures": 2,
            "trusted_validators": 12,
        }
        or plan["finalization"]
        != {
            "evidence_index_output": "forbidden-while-work-queue-is-incomplete",
            "qualification_receipt_output": "forbidden-while-work-queue-is-incomplete",
            "required_validator": "proofs/check-m1-evidence-index.py",
        }
    ):
        fail("M1 evidence plan format, counts, target, or nonclaim drifted")
    if (
        queue["format"] != WORK_FORMAT
        or queue["authority"] != PLAN_AUTHORITY
        or queue["nonclaim"] != PLAN_NONCLAIM
        or queue["status"] != "INCOMPLETE"
        or queue["plan_path"] != "plan.json"
        or queue["plan_sha256"] != digest_bytes(plan_raw)
        or queue["counts"]
        != {
            "available_producer_items": 357,
            "missing_items": 358,
            "missing_producer_items": 1,
        }
    ):
        fail("M1 work queue identity, counts, or incomplete status drifted")
    if plan["planner_sha256"] != digest_file(
        ferric / "proofs/m1-qualification/planner.py"
    ):
        fail("M1 planner source identity drifted")

    requirements, requirements_raw = read_canonical_json(
        ferric / "proofs/M1_REQUIREMENTS.json", "M1 requirements"
    )
    validate_requirements(requirements)
    run(
        [
            sys.executable,
            "-I",
            str(ferric / "proofs/check-m1-requirements.py"),
            str(ferric),
        ],
        "revalidate M1 requirements policy",
        cwd=ferric,
    )
    if plan["requirements"] != {
        "format": requirements["format"],
        "path": "proofs/M1_REQUIREMENTS.json",
        "sha256": digest_bytes(requirements_raw),
    }:
        fail("M1 plan requirements identity drifted")
    expected_resolutions = [
        {key: value for key, value in row.items() if key != "status"}
        for row in projected_paths(requirements)
    ]
    if plan["path_resolutions"] != expected_resolutions:
        fail("M1 plan path-resolution roster drifted")
    expected_validators, report_validators = trusted_validators(ferric)
    if plan["trusted_validators"] != expected_validators:
        fail("M1 plan trusted-validator roster drifted")
    sources = validate_sources(ferric, fe2o3, plan_fd, plan, requirements)

    items = queue["items"]
    if (
        not isinstance(items, list)
        or len(items) != 358
        or any(not isinstance(item, dict) for item in items)
        or [item.get("id") for item in items]
        != sorted(item.get("id") for item in items)
    ):
        fail("M1 work queue item roster drifted")
    tcb_items = [item for item in items if item.get("id", "").startswith("work.tcb.")]
    expected_tcb_items = []
    producer_path = "proofs/m1-qualification/produce-tcb-report.py"
    for identifier, kind in TCB:
        artifact_id = f"artifact.{identifier}"
        expected_tcb_items.append(
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
                        producer_path,
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
    if tcb_items != expected_tcb_items:
        fail("M1 TCB work-item producer contract drifted")
    if any(
        entry_exists_at(plan_fd, name)
        for name in ("evidence-index.json", "receipt.json")
    ):
        fail("fe2o3-contract production refuses a plan containing a closure output")
    if replay:
        rederive_candidate_plan(ferric, fe2o3, plan_fd, plan_raw, queue_raw)
    return (
        requirements,
        plan,
        queue,
        sources,
        report_validators,
        plan_raw,
        queue_raw,
    )


def tcb_report_for(
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
    subject: str,
) -> JsonObject:
    kinds = dict(TCB)
    structure = [
        {"artifact_id": f"artifact.{identifier}", "id": identifier, "kind": kind}
        for identifier, kind in TCB
    ]
    return {
        "authority": TCB_REPORT_AUTHORITY,
        "component_roster": component_roster(ferric, sources),
        "evidence_kind": "tcb-report",
        "format": TCB_REPORT_FORMAT,
        "milestone": "M1",
        "nonclaim": TCB_REPORT_NONCLAIM,
        "obligation_roster": projected_obligations(requirements),
        "obligation_state": "Open",
        "path_roster": projected_paths(requirements),
        "profile_roster": projected_profiles(requirements),
        "requirements_sha256": digest_file(ferric / "proofs/M1_REQUIREMENTS.json"),
        "source_roster": sources,
        "subject_tcb_id": subject,
        "subject_tcb_kind": kinds[subject],
        "target": REPORT_TARGET,
        "tcb_structure_roster": structure,
        "validator_roster": validators,
    }


def authenticate_tcb_reports(
    artifact_fd: int,
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
) -> tuple[list[JsonObject], list[tuple[str, str, BinaryIO, os.stat_result, bytes]]]:
    roster = []
    held: list[tuple[str, str, BinaryIO, os.stat_result, bytes]] = []
    try:
        for subject, kind in TCB:
            artifact_id = f"artifact.{subject}"
            name = f"{artifact_id}.tcb-report.json"
            source, before = open_regular_at(
                artifact_fd, name, f"M1 TCB report {subject}"
            )
            if (
                before.st_size <= 0
                or before.st_size > MAX_JSON_BYTES
                or stat.S_IMODE(before.st_mode) != 0o600
                or before.st_uid != os.geteuid()
            ):
                source.close()
                fail(f"M1 TCB report {subject} is not an exact owner-private 0600 file")
            raw = source.read(MAX_JSON_BYTES + 1)
            after = os.fstat(source.fileno())
            named = os.stat(name, dir_fd=artifact_fd, follow_symlinks=False)
            if (
                len(raw) != before.st_size
                or file_identity(before) != file_identity(after)
                or file_identity(after) != file_identity(named)
            ):
                source.close()
                fail(f"M1 TCB report changed while it was read: {subject}")
            expected = canonical_bytes(
                exact_keys(
                    tcb_report_for(ferric, requirements, sources, validators, subject),
                    TCB_REPORT_KEYS,
                    f"expected M1 TCB report {subject}",
                )
            )
            if raw != expected:
                source.close()
                fail(
                    f"M1 TCB report is not the exact authenticated projection: {subject}"
                )
            held.append((subject, name, source, after, raw))
            roster.append(
                {
                    "artifact_id": artifact_id,
                    "id": subject,
                    "identity_sha256": digest_bytes(raw),
                    "kind": kind,
                }
            )
    except BaseException:
        for _, _, source, _, _ in held:
            source.close()
        raise
    if len({row["identity_sha256"] for row in roster}) != 3:
        for _, _, source, _, _ in held:
            source.close()
        fail("M1 TCB outer report identities are not unique")
    return roster, held


def revalidate_tcb_reports(
    artifact_fd: int,
    held: list[tuple[str, str, BinaryIO, os.stat_result, bytes]],
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
) -> None:
    for subject, name, source, authenticated, raw in held:
        try:
            source.seek(0)
            current_raw = source.read(len(raw) + 1)
            current = os.fstat(source.fileno())
            named = os.stat(name, dir_fd=artifact_fd, follow_symlinks=False)
        except OSError as error:
            fail(f"cannot revalidate M1 TCB report {subject}: {error}")
        expected = canonical_bytes(
            exact_keys(
                tcb_report_for(ferric, requirements, sources, validators, subject),
                TCB_REPORT_KEYS,
                f"expected M1 TCB report {subject}",
            )
        )
        if (
            stat.S_ISLNK(named.st_mode)
            or current_raw != raw
            or raw != expected
            or file_identity(authenticated) != file_identity(current)
            or file_identity(current) != file_identity(named)
        ):
            fail(f"M1 TCB report changed after authentication: {subject}")


def select_fe2o3_contract_binding(
    plan: JsonObject, queue: JsonObject, binding_id: str
) -> tuple[JsonObject, JsonObject]:
    if not isinstance(binding_id, str) or not binding_id.startswith("binding."):
        fail(f"unknown M1 fe2o3-contract binding: {binding_id}")
    slots = [
        slot
        for slot in plan["binding_slots"]
        if slot.get("binding", {}).get("evidence_kind") == "fe2o3-contract"
    ]
    if len(slots) != 52:
        fail("M1 fe2o3-contract binding roster is incomplete")
    ids = [slot["binding"]["id"] for slot in slots]
    if ids != sorted(ids) or digest_bytes(("\n".join(ids) + "\n").encode("ascii")) != (
        FE2O3_CONTRACT_ROSTER_SHA256
    ):
        fail("M1 fe2o3-contract binding ID roster drifted")

    queue_by_id = {item["id"]: item for item in queue["items"]}
    rows = []
    for slot in slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        artifact = {
            "id": artifact_id,
            "kind": "ContractDocument",
            "path": f"artifacts/{artifact_id}.fe2o3-contract.json",
        }
        producer = {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-fe2o3-contract.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                binding["id"],
            ],
            "role": "ferric-m1-fe2o3-contract-reporter",
        }
        work_id = binding["id"].replace("binding.", "work.", 1)
        work = {
            "expected_artifact": artifact,
            "id": work_id,
            "producer": producer,
            "state": "missing",
            "subject": f"binding:{binding['id']}",
        }
        if (
            binding["obligation_class"] not in {"Assurance", "Roadmap"}
            or binding["profile_id"] not in {"composition", "kernel", "runtime"}
            or binding["source_identity_id"] not in SOURCE_IDS
            or binding["tcb_ids"] != [identifier for identifier, _ in TCB]
            or slot["expected_artifact"] != artifact
            or slot["producer"] != producer
            or slot["state"] != "missing"
            or slot["foundation_selectors"] != []
            or queue_by_id.get(work_id) != work
        ):
            fail(f"M1 fe2o3-contract producer contract drifted: {binding['id']}")
        rows.append(
            "|".join(
                [
                    binding["id"],
                    binding["obligation_class"],
                    binding["obligation_id"],
                    binding["profile_id"],
                    binding["path_id"],
                    binding["source_identity_id"],
                    artifact_id,
                    artifact["path"],
                ]
            )
            + "\n"
        )
    if digest_bytes("".join(rows).encode("ascii")) != FE2O3_CONTRACT_TSV_SHA256:
        fail("M1 fe2o3-contract allocation topology drifted")
    matches = [slot for slot in slots if slot["binding"]["id"] == binding_id]
    if len(matches) != 1:
        fail(f"unknown M1 fe2o3-contract binding: {binding_id}")
    slot = matches[0]
    binding = slot["binding"]
    resolutions = [
        row for row in plan["path_resolutions"] if row["id"] == binding["path_id"]
    ]
    if len(resolutions) != 1:
        fail("selected M1 fe2o3-contract path resolution is missing")
    resolution = resolutions[0]
    if (
        resolution["id"] != binding["path_id"]
        or resolution["source_identity_id"] != binding["source_identity_id"]
        or binding["source_identity_id"] != f"source.{resolution['repository']}"
        or resolution["availability"] not in {"ExistingFoundation", "RequiredFuture"}
    ):
        fail("selected M1 fe2o3-contract path resolution drifted")
    return slot, resolution


def requirement_spec(
    requirements: JsonObject, obligation_class: str, obligation_id: str
) -> tuple[JsonObject, str, list[str]]:
    if obligation_class == "Roadmap":
        matches = [
            row
            for row in requirements["roadmap_requirements"]
            if row["id"] == obligation_id
        ]
        if len(matches) != 1:
            fail("selected fe2o3-contract binding names an unknown roadmap obligation")
        spec = matches[0]
        return spec, spec["title"], spec["assurance_properties"]
    if obligation_class == "Assurance":
        matches = [
            row
            for row in requirements["assurance_properties"]
            if row["name"] == obligation_id
        ]
        if len(matches) != 1:
            fail("selected fe2o3-contract binding names an unknown assurance property")
        spec = matches[0]
        return spec, spec["boundary"], [obligation_id]
    fail("selected fe2o3-contract obligation class drifted")


def domain_digest(domain: str, parts: list[bytes]) -> str:
    hasher = hashlib.sha256()
    encoded_domain = domain.encode("ascii")
    hasher.update(len(encoded_domain).to_bytes(8, "big"))
    hasher.update(encoded_domain)
    for part in parts:
        hasher.update(len(part).to_bytes(8, "big"))
        hasher.update(part)
    return hasher.hexdigest()


def assurance_declarations(
    requirements: JsonObject, assurance_property_ids: list[str]
) -> list[JsonObject]:
    by_name = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    declarations = []
    for identifier in assurance_property_ids:
        record = by_name.get(identifier)
        if record is None:
            fail("fe2o3-contract assurance declaration is unknown")
        declarations.append(
            {
                "boundary_sha256": digest_bytes(record["boundary"].encode("utf-8")),
                "fe2o3_kind": record["fe2o3_kind"],
                "name": identifier,
                "obligation_state": "Open",
                "required_status_at_closure": record["required_status_at_closure"],
            }
        )
    return declarations


def contract_body(
    binding: JsonObject,
    declarations: list[JsonObject],
    requirements_sha256: str,
    source_roster_sha256: str,
    tcb_roster_sha256: str,
) -> JsonObject:
    return exact_keys(
        {
            "assurance_property_declarations": declarations,
            "binding_sha256": binding["binding_sha256"],
            "format": CONTRACT_BODY_FORMAT,
            "obligation_class": binding["obligation_class"],
            "obligation_id": binding["obligation_id"],
            "path_id": binding["path_id"],
            "profile_id": binding["profile_id"],
            "requirements_sha256": requirements_sha256,
            "source_roster_sha256": source_roster_sha256,
            "statement_sha256": binding["statement_sha256"],
            "target": REPORT_TARGET,
            "tcb_roster_sha256": tcb_roster_sha256,
        },
        CONTRACT_BODY_KEYS,
        "M1 fe2o3 contract body",
    )


def contract_set(binding: JsonObject, body_sha256: str) -> JsonObject:
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
        identity_parts + [bytes.fromhex(body_sha256)],
    )
    obligation_identity = domain_digest(
        "ferric.m1.fe2o3-contract.obligation-identity.v1", identity_parts
    )
    return exact_keys(
        {
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
                            "bytes_sha256": body_sha256,
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
        },
        CONTRACT_SET_KEYS,
        "M1 fe2o3 ContractSet declaration",
    )


def fe2o3_contract_documents(
    requirements_sha256: str,
    requirements: JsonObject,
    sources: list[JsonObject],
    tcb: list[JsonObject],
    slot: JsonObject,
    resolution: JsonObject,
) -> tuple[JsonObject, JsonObject, JsonObject]:
    binding = slot["binding"]
    spec, statement, assurance_property_ids = requirement_spec(
        requirements, binding["obligation_class"], binding["obligation_id"]
    )
    if (
        spec["obligation_state"] != "Open"
        or binding["profile_id"] not in spec["evidence_profiles"]
        or binding["path_id"] not in spec["path_obligations"]
        or binding["statement_sha256"] != digest_bytes(statement.encode("utf-8"))
        or resolution["id"] != binding["path_id"]
        or resolution["source_identity_id"] != binding["source_identity_id"]
    ):
        fail("selected fe2o3-contract obligation or path drifted")
    profiles = {row["id"]: row["kinds"] for row in requirements["evidence_profiles"]}
    if profiles.get(binding["profile_id"], []).count("fe2o3-contract") != 1:
        fail("selected profile does not admit exactly one fe2o3-contract kind")
    source_by_id = {row["id"]: row for row in sources}
    bound_source = source_by_id.get(binding["source_identity_id"])
    if bound_source is None:
        fail("selected fe2o3-contract source identity is missing")
    declarations = assurance_declarations(requirements, assurance_property_ids)
    source_roster_sha256 = canonical_digest(sources)
    tcb_roster_sha256 = canonical_digest(tcb)
    body = contract_body(
        binding,
        declarations,
        requirements_sha256,
        source_roster_sha256,
        tcb_roster_sha256,
    )
    body_bytes = canonical_bytes(body)
    contract_set_value = contract_set(binding, digest_bytes(body_bytes))
    contract_set_bytes = canonical_bytes(contract_set_value)
    artifact_id = binding["artifact_id"]
    report = exact_keys(
        {
            "assurance_property_declarations": declarations,
            "authority": REPORT_AUTHORITY,
            "binding_sha256": binding["binding_sha256"],
            "bound_source_identity_sha256": canonical_digest(bound_source),
            "contract_body_path": (f"contracts/{artifact_id}.fe2o3-contract-body.json"),
            "contract_body_sha256": digest_bytes(body_bytes),
            "contract_body_size_bytes": len(body_bytes),
            "contract_set_path": (
                f"contract-sets/{artifact_id}.fe2o3-contract-set.json"
            ),
            "contract_set_schema": CONTRACT_SET_SCHEMA,
            "contract_set_sha256": digest_bytes(contract_set_bytes),
            "contract_set_size_bytes": len(contract_set_bytes),
            "contract_set_source_path": CONTRACT_SET_SOURCE_PATH,
            "contract_set_validation": CONTRACT_SET_VALIDATION,
            "contract_target": REPORT_TARGET,
            "evidence_kind": "fe2o3-contract",
            "format": REPORT_FORMAT,
            "nonclaim": REPORT_NONCLAIM,
            "obligation_class": binding["obligation_class"],
            "obligation_id": binding["obligation_id"],
            "obligation_state": "Open",
            "path_id": binding["path_id"],
            "path_resolution_sha256": canonical_digest(resolution),
            "profile_id": binding["profile_id"],
            "requirements_sha256": requirements_sha256,
            "source_identity_id": binding["source_identity_id"],
            "source_roster_sha256": source_roster_sha256,
            "statement_sha256": binding["statement_sha256"],
            "tcb_identity_sha256s": {row["id"]: row["identity_sha256"] for row in tcb},
            "tcb_roster_sha256": tcb_roster_sha256,
        },
        REPORT_KEYS,
        "M1 fe2o3-contract report",
    )
    return body, contract_set_value, report


def ensure_artifact_directory(plan_fd: int) -> int:
    return open_private_directory_at(plan_fd, "artifacts", "M1 artifact directory")


def ensure_private_child_directory(
    plan_fd: int, name: str, description: str
) -> tuple[int, bool]:
    created = False
    try:
        os.mkdir(name, 0o700, dir_fd=plan_fd)
        created = True
        os.fsync(plan_fd)
    except FileExistsError:
        pass
    except OSError as error:
        fail(f"cannot create {description}: {error}")
    try:
        descriptor = open_private_directory_at(plan_fd, name, description)
    except BaseException:
        raise
    return descriptor, created


def published_binding(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
    )


def rollback_exact_file(
    directory_fd: int, name: str, descriptor: int, description: str
) -> str | None:
    try:
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except FileNotFoundError:
        return None
    except OSError as error:
        return f"cannot inspect failed {description} publication: {error}"
    try:
        held = os.fstat(descriptor)
    except OSError as error:
        return f"cannot identify failed {description} publication: {error}"
    if (
        stat.S_ISLNK(named.st_mode)
        or not stat.S_ISREG(named.st_mode)
        or published_binding(named) != published_binding(held)
    ):
        return f"cannot remove replaced failed {description} publication"
    try:
        os.unlink(name, dir_fd=directory_fd)
        os.fsync(directory_fd)
    except OSError as error:
        return f"cannot remove failed {description} publication: {error}"
    return None


def rollback_publications(
    published: list[tuple[int, str, int, bytes, str, os.stat_result]],
) -> list[str]:
    failures = []
    for directory_fd, name, descriptor, _, description, _ in reversed(published):
        failure = rollback_exact_file(directory_fd, name, descriptor, description)
        if failure is not None:
            failures.append(failure)
    return failures


def create_new_file_at(
    directory_fd: int, name: str, value: bytes, description: str
) -> int:
    flags = os.O_RDWR | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        descriptor = os.open(name, flags, 0o600, dir_fd=directory_fd)
    except OSError as error:
        fail(f"cannot create {description} without replacement: {error}")
    try:
        created = os.fstat(descriptor)
        if (
            not stat.S_ISREG(created.st_mode)
            or stat.S_IMODE(created.st_mode) != 0o600
            or created.st_uid != os.geteuid()
            or created.st_size != 0
        ):
            fail(f"new {description} is not an exact owner-private regular file")
        remaining = memoryview(value)
        while remaining:
            written = os.write(descriptor, remaining)
            if written <= 0:
                fail(f"cannot completely write {description}")
            remaining = remaining[written:]
        os.fsync(descriptor)
        after_write = os.fstat(descriptor)
        if after_write.st_size != len(value):
            fail(f"published {description} has an unexpected size")
        os.lseek(descriptor, 0, os.SEEK_SET)
        raw = os.read(descriptor, len(value) + 1)
        after_read = os.fstat(descriptor)
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if (
            raw != value
            or file_identity(after_write) != file_identity(after_read)
            or stat.S_ISLNK(named.st_mode)
            or not stat.S_ISREG(named.st_mode)
            or published_binding(named) != published_binding(after_read)
        ):
            fail(f"published {description} bytes or binding changed")
    except OSError as error:
        rollback_failure = rollback_exact_file(
            directory_fd, name, descriptor, description
        )
        os.close(descriptor)
        if rollback_failure is not None:
            fail(f"cannot publish {description}: {error}; {rollback_failure}")
        fail(f"cannot publish {description}: {error}")
    except BaseException:
        rollback_failure = rollback_exact_file(
            directory_fd, name, descriptor, description
        )
        os.close(descriptor)
        if rollback_failure is not None:
            fail(f"{description} publication rollback failed: {rollback_failure}")
        raise
    return descriptor


def verify_published_file(
    directory_fd: int,
    name: str,
    descriptor: int,
    expected: bytes,
    authenticated: os.stat_result,
    description: str,
) -> None:
    try:
        before = os.fstat(descriptor)
        os.lseek(descriptor, 0, os.SEEK_SET)
        raw = os.read(descriptor, len(expected) + 1)
        after = os.fstat(descriptor)
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        stat.S_ISLNK(named.st_mode)
        or not stat.S_ISREG(named.st_mode)
        or raw != expected
        or file_identity(authenticated) != file_identity(before)
        or file_identity(before) != file_identity(after)
        or file_identity(after) != file_identity(named)
    ):
        fail(f"published {description} bytes or binding changed after directory sync")


def publish_fe2o3_contract(
    plan_custody: AbsoluteDirectoryCustody,
    plan_fd: int,
    artifact_fd: int,
    body_fd: int,
    set_fd: int,
    artifact_id: str,
    body: bytes,
    contract_set_value: bytes,
    report: bytes,
    custody_check: Callable[[], None],
) -> None:
    report_name = f"{artifact_id}.fe2o3-contract.json"
    body_name = f"{artifact_id}.fe2o3-contract-body.json"
    set_name = f"{artifact_id}.fe2o3-contract-set.json"
    published: list[tuple[int, str, int, bytes, str, os.stat_result]] = []
    try:
        revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
        )
        revalidate_child_directory(
            plan_fd, "contracts", body_fd, "M1 contract body directory"
        )
        revalidate_child_directory(
            plan_fd, "contract-sets", set_fd, "M1 ContractSet directory"
        )
        revalidate_absolute_directory(plan_custody, private=True)
        if (
            entry_exists_at(artifact_fd, report_name)
            or entry_exists_at(body_fd, body_name)
            or entry_exists_at(set_fd, set_name)
        ):
            fail("fe2o3-contract publication refuses a preexisting output")
        custody_check()
        body_file_fd = create_new_file_at(
            body_fd, body_name, body, "M1 fe2o3 contract body"
        )
        published.append(
            (
                body_fd,
                body_name,
                body_file_fd,
                body,
                "M1 fe2o3 contract body",
                os.fstat(body_file_fd),
            )
        )
        custody_check()
        set_file_fd = create_new_file_at(
            set_fd,
            set_name,
            contract_set_value,
            "M1 fe2o3 ContractSet declaration",
        )
        published.append(
            (
                set_fd,
                set_name,
                set_file_fd,
                contract_set_value,
                "M1 fe2o3 ContractSet declaration",
                os.fstat(set_file_fd),
            )
        )
        custody_check()
        report_file_fd = create_new_file_at(
            artifact_fd, report_name, report, "M1 fe2o3-contract report"
        )
        published.append(
            (
                artifact_fd,
                report_name,
                report_file_fd,
                report,
                "M1 fe2o3-contract report",
                os.fstat(report_file_fd),
            )
        )
        os.fsync(body_fd)
        os.fsync(set_fd)
        os.fsync(artifact_fd)
        os.fsync(plan_fd)
        revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
        )
        revalidate_child_directory(
            plan_fd, "contracts", body_fd, "M1 contract body directory"
        )
        revalidate_child_directory(
            plan_fd, "contract-sets", set_fd, "M1 ContractSet directory"
        )
        revalidate_absolute_directory(plan_custody, private=True)
        custody_check()
        for (
            directory_fd,
            name,
            descriptor,
            expected,
            description,
            identity,
        ) in published:
            verify_published_file(
                directory_fd, name, descriptor, expected, identity, description
            )
    except OSError as error:
        failures = rollback_publications(published)
        if failures:
            fail(
                f"cannot durably publish M1 fe2o3 contract: {error}; "
                f"rollback failures: {' | '.join(failures)}"
            )
        fail(f"cannot durably publish M1 fe2o3 contract: {error}")
    except BaseException:
        failures = rollback_publications(published)
        if failures:
            fail(
                "M1 fe2o3 contract publication rollback failures: "
                + " | ".join(failures)
            )
        raise
    finally:
        for _, _, descriptor, _, _, _ in published:
            try:
                os.close(descriptor)
            except OSError:
                pass


def produce(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    binding_id: str,
) -> None:
    ferric = Path(ferric_argument).absolute()
    fe2o3 = Path(fe2o3_argument).absolute()
    plan_root = Path(plan_argument).absolute()
    ferric_custody: AbsoluteDirectoryCustody | None = None
    fe2o3_custody: AbsoluteDirectoryCustody | None = None
    plan_custody: AbsoluteDirectoryCustody | None = None
    plan_fd = -1
    artifact_fd = -1
    body_fd = -1
    set_fd = -1
    tcb_files: list[tuple[str, str, BinaryIO, os.stat_result, bytes]] = []
    plan_files: list[HeldFile] = []
    closure_custody: HeldDirectoryFiles | None = None
    report_bytes = b""
    body_bytes = b""
    set_bytes = b""
    try:
        ferric_custody = authenticate_absolute_directory(
            ferric, "Ferric source repository"
        )
        fe2o3_custody = authenticate_absolute_directory(
            fe2o3, "fe2o3 source repository"
        )
        plan_custody = authenticate_absolute_directory(
            plan_root, "M1 evidence plan directory", private=True
        )
        plan_fd = directory_custody_fd(plan_custody)
        artifact_fd = ensure_artifact_directory(plan_fd)
        revalidate_absolute_directory(plan_custody, private=True)
        requirements, plan, queue, sources, validators, plan_raw, queue_raw = (
            validate_plan(ferric, fe2o3, plan_fd)
        )
        plan_files = [
            authenticate_held_file_at(
                plan_fd, "plan.json", plan_raw, "M1 evidence plan"
            ),
            authenticate_held_file_at(
                plan_fd,
                "missing-work.json",
                queue_raw,
                "M1 evidence work queue",
            ),
        ]
        closure_custody = authenticate_source_closures(plan_fd, plan)
        slot, resolution = select_fe2o3_contract_binding(plan, queue, binding_id)
        tcb, tcb_files = authenticate_tcb_reports(
            artifact_fd, ferric, requirements, sources, validators
        )
        documents = fe2o3_contract_documents(
            plan["requirements"]["sha256"],
            requirements,
            sources,
            tcb,
            slot,
            resolution,
        )
        body_bytes, set_bytes, report_bytes = tuple(
            canonical_bytes(value) for value in documents
        )

        repeated = validate_plan(ferric, fe2o3, plan_fd, replay=False)
        if repeated[5] != plan_raw or repeated[6] != queue_raw:
            fail("M1 plan or work queue changed during fe2o3-contract production")
        repeated_slot, repeated_resolution = select_fe2o3_contract_binding(
            repeated[1], repeated[2], binding_id
        )
        repeated_documents = fe2o3_contract_documents(
            repeated[1]["requirements"]["sha256"],
            repeated[0],
            repeated[3],
            tcb,
            repeated_slot,
            repeated_resolution,
        )
        if tuple(canonical_bytes(value) for value in repeated_documents) != (
            body_bytes,
            set_bytes,
            report_bytes,
        ):
            fail("M1 fe2o3-contract inputs changed during production")
        repositories = {
            "fe2o3": (fe2o3, fe2o3_custody),
            "ferric": (ferric, ferric_custody),
        }
        expected_repository_identities = source_identity_map(repeated[3])

        def revalidate_completion_inputs() -> None:
            for held in plan_files:
                revalidate_held_file(plan_fd, held)
            revalidate_source_closures(plan_fd, closure_custody)
            revalidate_tcb_reports(
                artifact_fd,
                tcb_files,
                ferric,
                repeated[0],
                repeated[3],
                repeated[4],
            )
            revalidate_repository_identities(
                repositories, expected_repository_identities
            )
            if any(
                entry_exists_at(plan_fd, name)
                for name in ("evidence-index.json", "receipt.json")
            ):
                fail("fe2o3-contract producer created a forbidden closure output")
            revalidate_child_directory(
                plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
            )
            revalidate_absolute_directory(plan_custody, private=True)

        revalidate_completion_inputs()
        body_fd, _ = ensure_private_child_directory(
            plan_fd, "contracts", "M1 contract body directory"
        )
        set_fd, _ = ensure_private_child_directory(
            plan_fd, "contract-sets", "M1 ContractSet directory"
        )
        publish_fe2o3_contract(
            plan_custody,
            plan_fd,
            artifact_fd,
            body_fd,
            set_fd,
            slot["binding"]["artifact_id"],
            body_bytes,
            set_bytes,
            report_bytes,
            revalidate_completion_inputs,
        )
    finally:
        if closure_custody is not None:
            close_source_closures(closure_custody)
        for _, source, _, _, _ in plan_files:
            source.close()
        for _, _, source, _, _ in tcb_files:
            source.close()
        if set_fd >= 0:
            os.close(set_fd)
        if body_fd >= 0:
            os.close(body_fd)
        if artifact_fd >= 0:
            os.close(artifact_fd)
        if plan_custody is not None:
            close_absolute_directory(plan_custody)
        if fe2o3_custody is not None:
            close_absolute_directory(fe2o3_custody)
        if ferric_custody is not None:
            close_absolute_directory(ferric_custody)
    print(
        f"PASS: produced M1 fe2o3 contract binding={binding_id} "
        f"report_sha256={digest_bytes(report_bytes)}"
    )


def main() -> None:
    if len(sys.argv) != 5:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_REPO PLAN_DIR binding.NNNNN")
    produce(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4])


if __name__ == "__main__":
    main()
