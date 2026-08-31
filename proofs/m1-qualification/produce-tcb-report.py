#!/usr/bin/env python3
"""Produce one source-authenticated, declaration-only M1 TCB report."""

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
from typing import Any, BinaryIO, NoReturn


PLAN_FORMAT = "FERRIC-M1-EVIDENCE-PLAN-V1"
WORK_FORMAT = "FERRIC-M1-EVIDENCE-WORK-QUEUE-V1"
PLAN_AUTHORITY = "planning-only-no-evidence"
PLAN_NONCLAIM = (
    "This bundle allocates external M1 evidence work only. It is not an evidence "
    "index, qualification receipt, validation result, or M1 closure claim."
)
REPORT_FORMAT = "FERRIC-M1-TCB-REPORT-V1"
REPORT_TARGET = "gfx942:xnack-"
REPORT_AUTHORITY = "trusted-boundary-declaration-only"
REPORT_NONCLAIM = (
    "This report authenticates the declared M1 trusted boundary only. It does "
    "not establish component presence, version provenance, compiler or runtime "
    "correctness, hardware behavior, theorem truth, machine refinement, load, "
    "launch, performance, or qualification authority and closes no obligation."
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
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
MAX_JSON_BYTES = 16_000_000
MAX_FILE_BYTES = 64_000_000


JsonObject = dict[str, Any]


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


def run(arguments: list[str], description: str, *, cwd: Path) -> str:
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
        fail("TCB production requires every M1 obligation to remain Open")
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
    with tempfile.TemporaryDirectory(prefix="ferric-m1-tcb-planner-replay-") as raw:
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
    ferric: Path, fe2o3: Path, plan_fd: int, subject: str
) -> tuple[JsonObject, JsonObject, list[JsonObject], list[JsonObject], bytes, bytes]:
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
            "completion_transition_output": "completed-work.json",
            "evidence_index_output": "requires-authenticated-work-queue-completion-v1",
            "qualification_receipt_output": "requires-authenticated-work-queue-completion-v1",
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
            "available_producer_items": 358,
            "missing_items": 358,
            "missing_producer_items": 0,
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
    if subject not in {identifier for identifier, _ in TCB}:
        fail(f"unknown M1 TCB subject: {subject}")
    if any(
        entry_exists_at(plan_fd, name)
        for name in ("evidence-index.json", "receipt.json")
    ):
        fail("TCB production refuses a plan containing a closure output")
    rederive_candidate_plan(ferric, fe2o3, plan_fd, plan_raw, queue_raw)
    return requirements, plan, sources, report_validators, plan_raw, queue_raw


def report_for(
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
        "authority": REPORT_AUTHORITY,
        "component_roster": component_roster(ferric, sources),
        "evidence_kind": "tcb-report",
        "format": REPORT_FORMAT,
        "milestone": "M1",
        "nonclaim": REPORT_NONCLAIM,
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


def ensure_artifact_directory(plan_fd: int) -> int:
    try:
        os.mkdir("artifacts", 0o700, dir_fd=plan_fd)
    except FileExistsError:
        pass
    except OSError as error:
        fail(f"cannot create M1 artifact directory: {error}")
    return open_private_directory_at(plan_fd, "artifacts", "M1 artifact directory")


def report_binding(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
    )


def create_new_report_at(artifact_fd: int, name: str, value: bytes) -> int:
    flags = os.O_RDWR | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        descriptor = os.open(name, flags, 0o600, dir_fd=artifact_fd)
    except OSError as error:
        fail(f"cannot create TCB report without replacement: {error}")
    try:
        created = os.fstat(descriptor)
        if (
            not stat.S_ISREG(created.st_mode)
            or stat.S_IMODE(created.st_mode) != 0o600
            or created.st_uid != os.geteuid()
            or created.st_size != 0
        ):
            fail("new M1 TCB report is not an exact owner-private regular file")
        remaining = memoryview(value)
        while remaining:
            written = os.write(descriptor, remaining)
            if written <= 0:
                fail("cannot completely write M1 TCB report")
            remaining = remaining[written:]
        os.fsync(descriptor)
        after_write = os.fstat(descriptor)
        if after_write.st_size != len(value):
            fail("published M1 TCB report has an unexpected size")
        os.lseek(descriptor, 0, os.SEEK_SET)
        chunks = []
        remaining_size = len(value) + 1
        while remaining_size:
            chunk = os.read(descriptor, remaining_size)
            if not chunk:
                break
            chunks.append(chunk)
            remaining_size -= len(chunk)
        after_read = os.fstat(descriptor)
        if b"".join(chunks) != value or file_identity(after_write) != file_identity(
            after_read
        ):
            fail("published M1 TCB report bytes changed")
        named = os.stat(name, dir_fd=artifact_fd, follow_symlinks=False)
        if (
            stat.S_ISLNK(named.st_mode)
            or not stat.S_ISREG(named.st_mode)
            or report_binding(named) != report_binding(after_read)
            or named.st_size != len(value)
        ):
            fail("published M1 TCB report binding changed")
    except OSError as error:
        os.close(descriptor)
        fail(f"cannot publish M1 TCB report: {error}")
    except BaseException:
        os.close(descriptor)
        raise
    return descriptor


def publish_report(plan_path: Path, plan_fd: int, subject: str, value: bytes) -> None:
    revalidate_directory_path(plan_path, plan_fd, "M1 evidence plan directory")
    artifact_fd = ensure_artifact_directory(plan_fd)
    report_fd = -1
    try:
        revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
        )
        artifact_id = f"artifact.{subject}"
        name = f"{artifact_id}.tcb-report.json"
        report_fd = create_new_report_at(artifact_fd, name, value)
        os.fsync(artifact_fd)
        os.fsync(plan_fd)
        revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
        )
        revalidate_directory_path(plan_path, plan_fd, "M1 evidence plan directory")
        named = os.stat(name, dir_fd=artifact_fd, follow_symlinks=False)
        held = os.fstat(report_fd)
        if (
            stat.S_ISLNK(named.st_mode)
            or not stat.S_ISREG(named.st_mode)
            or report_binding(named) != report_binding(held)
            or named.st_size != len(value)
        ):
            fail("published M1 TCB report binding changed after directory sync")
    except OSError as error:
        fail(f"cannot durably publish M1 TCB report: {error}")
    finally:
        if report_fd >= 0:
            os.close(report_fd)
        os.close(artifact_fd)


def produce(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    subject: str,
) -> None:
    ferric = Path(ferric_argument).resolve(strict=True)
    fe2o3 = Path(fe2o3_argument).resolve(strict=True)
    plan_root = Path(plan_argument).absolute()
    try:
        if plan_root.resolve(strict=True) != plan_root:
            fail("M1 evidence plan path contains a symlink")
    except OSError as error:
        fail(f"M1 evidence plan directory is unavailable: {error}")
    plan_fd = open_private_directory(plan_root, "M1 evidence plan directory")
    try:
        revalidate_directory_path(plan_root, plan_fd, "M1 evidence plan directory")
        requirements, _, sources, validators, plan_raw, queue_raw = validate_plan(
            ferric, fe2o3, plan_fd, subject
        )
        report_bytes = canonical_bytes(
            report_for(ferric, requirements, sources, validators, subject)
        )

        revalidate_directory_path(plan_root, plan_fd, "M1 evidence plan directory")
        repeated = validate_plan(ferric, fe2o3, plan_fd, subject)
        if repeated[4] != plan_raw or repeated[5] != queue_raw:
            fail("M1 plan or work queue changed during TCB production")
        if (
            canonical_bytes(
                report_for(ferric, repeated[0], repeated[2], repeated[3], subject)
            )
            != report_bytes
        ):
            fail("M1 TCB report inputs changed during production")

        publish_report(plan_root, plan_fd, subject, report_bytes)
        if any(
            entry_exists_at(plan_fd, name)
            for name in ("evidence-index.json", "receipt.json")
        ):
            fail("TCB producer created a forbidden closure output")
        revalidate_directory_path(plan_root, plan_fd, "M1 evidence plan directory")
    finally:
        os.close(plan_fd)
    print(
        f"PASS: produced M1 TCB report subject={subject} "
        f"sha256={digest_bytes(report_bytes)}"
    )


def main() -> None:
    if len(sys.argv) != 5:
        fail(
            f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_REPO PLAN_DIR "
            "tcb.compiler|tcb.hardware|tcb.runtime"
        )
    produce(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4])


if __name__ == "__main__":
    main()
