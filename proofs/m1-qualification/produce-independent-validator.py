#!/usr/bin/env python3
"""Export M1 independent-review requests and ingest one external response."""

from __future__ import annotations

import copy
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
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REQUEST_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATION-REQUEST-V1"
HANDOFF_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATION-HANDOFF-V1"
RESPONSE_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATION-RESPONSE-V1"
REPORT_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-REPORT-V1"
ROSTER_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-ROSTER-V1"
TRANSCRIPT_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-TRANSCRIPT-V1"
VALIDATOR_PROTOCOL = "ferric.external-independent-validation.v1"
TARGET = "gfx942:xnack-"
AUTHORITY = "independent-validation-observation-only"
TCB_REPORT_AUTHORITY = "trusted-boundary-declaration-only"
TCB_REPORT_NONCLAIM = (
    "This report authenticates the declared M1 trusted boundary only. It does "
    "not establish component presence, version provenance, compiler or runtime "
    "correctness, hardware behavior, theorem truth, machine refinement, load, "
    "launch, performance, or qualification authority and closes no obligation."
)
INDEPENDENCE_ATTESTATION = (
    "The named checker organization, repository, source closure, and executable "
    "are independent of the Ferric and fe2o3 subject source closures."
)
NONCLAIM = (
    "This report authenticates an independent checker identity and its exact "
    "case observations only. Observations are not a theorem, machine refinement, "
    "load, launch, hardware, performance, or qualification authority."
)
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
INDEPENDENT_ROSTER_SHA256 = (
    "1f462479589b1e4bf3e1138997109d297c25279c89fd9f2d5fd6ec53192f0305"
)
INDEPENDENT_TOPOLOGY_SHA256 = (
    "e9ae479793ac9844bd7fc6ec64680f94273e1312f09f88bb6d608d357b38fa3c"
)
PRODUCER_PATH = "proofs/m1-qualification/produce-independent-validator.py"
PRODUCER_ROLE = "ferric-m1-independent-review-intake"
TCB = (
    ("tcb.compiler", "Compiler"),
    ("tcb.hardware", "Hardware"),
    ("tcb.runtime", "Runtime"),
)
SOURCE_IDS = ("source.fe2o3", "source.ferric")
CASE_MATRIX = (
    ("canonical-subject", "PASS"),
    ("boundary-conforming-subject", "PASS"),
    ("obligation-substitution", "EXPECTED_FAIL"),
    ("property-substitution", "EXPECTED_FAIL"),
    ("path-substitution", "EXPECTED_FAIL"),
    ("profile-substitution", "EXPECTED_FAIL"),
    ("source-closure-substitution", "EXPECTED_FAIL"),
    ("target-substitution", "EXPECTED_FAIL"),
    ("tcb-substitution", "EXPECTED_FAIL"),
    ("malformed-status", "EXPECTED_FAIL"),
)
CASE_COUNTS = {"expected_fail": 8, "pass": 2, "total": 10}
SUBJECT_ORGANIZATIONS = {"fe2o3", "ferric", "harsh-nod"}
SUBJECT_REPOSITORIES = {"fe2o3", "ferric"}
TRUSTED_CHECKER_PATH = "proofs/m1/evidence/validate-independent-validator.py"
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+\Z")
UTC_TIME = re.compile(
    r"20[0-9]{2}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12][0-9]|3[01])"
    r"T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z\Z"
)
MAX_JSON_BYTES = 2_000_000
MAX_MATERIAL_BYTES = 128_000_000
MAX_OUTPUT_BYTES = 4_000_000

CHECKER_KEYS = {
    "commit",
    "executable_path",
    "executable_sha256",
    "id",
    "input_schema_sha256",
    "organization",
    "output_schema_sha256",
    "protocol",
    "repository",
    "source_closure_sha256",
    "tree",
    "version",
}
RESPONSE_KEYS = {
    "binding_sha256",
    "checker",
    "completed_at_utc",
    "format",
    "independence_attestation",
    "request_sha256",
    "results",
    "started_at_utc",
}
RESPONSE_RESULT_KEYS = {
    "exit_code",
    "expected_status",
    "id",
    "input_sha256",
    "observed_status",
    "output_path",
    "output_sha256",
    "output_size_bytes",
}

JsonObject = dict[str, Any]
FileCustody = tuple[str, BinaryIO, os.stat_result, bytes]
ChildDirectoryCustody = tuple[int, str, int, os.stat_result, str]
AbsoluteDirectoryCustody = tuple[list[ChildDirectoryCustody], list[int], Path]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


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


def reject_duplicate_key(pairs: list[tuple[str, Any]]) -> JsonObject:
    value: JsonObject = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def exact_keys(value: Any, keys: set[str], description: str) -> JsonObject:
    if not isinstance(value, dict) or set(value) != keys:
        fail(f"{description} fields drifted")
    return value


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
        or any(
            part in {"", ".", ".."} or SAFE_SEGMENT.fullmatch(part) is None
            for part in relative.parts
        )
    ):
        fail(f"unsafe {description}")
    return relative


def file_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def stable_binding(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
        metadata.st_nlink,
    )


def directory_binding(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
    )


def canonical_root(argument: str, description: str, *, private: bool) -> Path:
    candidate = Path(argument).absolute()
    try:
        resolved = candidate.resolve(strict=True)
        metadata = candidate.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if resolved != candidate or not stat.S_ISDIR(metadata.st_mode):
        fail(f"{description} must be a canonical non-symlink directory")
    if private and (
        stat.S_IMODE(metadata.st_mode) != 0o700 or metadata.st_uid != os.geteuid()
    ):
        fail(f"{description} must be an owner-private 0700 directory")
    return candidate


def open_directory(
    path: Path, description: str, *, private: bool
) -> tuple[int, os.stat_result]:
    flags = os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before = path.lstat()
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot open {description}: {error}")
    if (
        not stat.S_ISDIR(before.st_mode)
        or not stat.S_ISDIR(opened.st_mode)
        or directory_binding(before) != directory_binding(opened)
        or (
            private
            and (stat.S_IMODE(opened.st_mode) != 0o700 or opened.st_uid != os.geteuid())
        )
    ):
        os.close(descriptor)
        fail(f"{description} directory identity or ownership drifted")
    return descriptor, opened


def revalidate_directory(
    path: Path, descriptor: int, authenticated: os.stat_result, description: str
) -> None:
    try:
        named = path.lstat()
        held = os.fstat(descriptor)
        resolved = path.resolve(strict=True)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        resolved != path
        or not stat.S_ISDIR(named.st_mode)
        or directory_binding(named) != directory_binding(authenticated)
        or directory_binding(held) != directory_binding(authenticated)
    ):
        fail(f"{description} directory was rebound")


def open_directory_at(
    parent_fd: int, name: str, description: str
) -> tuple[int, os.stat_result]:
    if SAFE_SEGMENT.fullmatch(name) is None:
        fail(f"unsafe {description} name")
    flags = os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        descriptor = os.open(name, flags, dir_fd=parent_fd)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot open {description}: {error}")
    if (
        not stat.S_ISDIR(named.st_mode)
        or directory_binding(named) != directory_binding(opened)
        or stat.S_IMODE(opened.st_mode) != 0o700
        or opened.st_uid != os.geteuid()
    ):
        os.close(descriptor)
        fail(f"{description} must be an exact owner-private 0700 directory")
    return descriptor, opened


def revalidate_directory_at(
    parent_fd: int,
    name: str,
    descriptor: int,
    authenticated: os.stat_result,
    description: str,
) -> None:
    try:
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        held = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        not stat.S_ISDIR(named.st_mode)
        or directory_binding(named) != directory_binding(authenticated)
        or directory_binding(held) != directory_binding(authenticated)
    ):
        fail(f"{description} directory was rebound")


def open_absolute_directory(
    path: Path, description: str, *, private: bool
) -> AbsoluteDirectoryCustody:
    if not path.is_absolute() or path.resolve(strict=True) != path:
        fail(f"{description} must be a canonical absolute directory")
    flags = os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        root_fd = os.open("/", flags)
    except OSError as error:
        fail(f"cannot open filesystem root for {description}: {error}")
    descriptors = [root_fd]
    custody: list[ChildDirectoryCustody] = []
    parent_fd = root_fd
    try:
        for ordinal, name in enumerate(path.parts[1:]):
            named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
            descriptor = os.open(name, flags, dir_fd=parent_fd)
            opened = os.fstat(descriptor)
            if (
                not stat.S_ISDIR(named.st_mode)
                or directory_binding(named) != directory_binding(opened)
                or (
                    private
                    and ordinal == len(path.parts[1:]) - 1
                    and (
                        stat.S_IMODE(opened.st_mode) != 0o700
                        or opened.st_uid != os.geteuid()
                    )
                )
            ):
                os.close(descriptor)
                fail(f"{description} absolute directory chain drifted")
            descriptors.append(descriptor)
            custody.append(
                (
                    parent_fd,
                    name,
                    descriptor,
                    opened,
                    f"{description} component {name}",
                )
            )
            parent_fd = descriptor
    except OSError as error:
        for descriptor in reversed(descriptors):
            os.close(descriptor)
        fail(f"cannot authenticate {description} absolute directory chain: {error}")
    except BaseException:
        for descriptor in reversed(descriptors):
            try:
                os.close(descriptor)
            except OSError:
                pass
        raise
    return custody, descriptors, path


def revalidate_absolute_directory(custody: AbsoluteDirectoryCustody) -> None:
    components, _, path = custody
    try:
        if path.resolve(strict=True) != path:
            fail("absolute directory path became noncanonical")
    except OSError as error:
        fail(f"cannot revalidate absolute directory path: {error}")
    revalidate_child_custodies(components)


def close_absolute_directory(custody: AbsoluteDirectoryCustody) -> None:
    _, descriptors, _ = custody
    for descriptor in reversed(descriptors):
        try:
            os.close(descriptor)
        except OSError:
            pass


def open_regular_at(
    directory_fd: int,
    name: str,
    limit: int,
    description: str,
    *,
    mode: int = 0o600,
    canonical_json: bool = False,
) -> FileCustody:
    if SAFE_SEGMENT.fullmatch(name) is None:
        fail(f"unsafe {description} name")
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        descriptor = os.open(name, flags, dir_fd=directory_fd)
        source = os.fdopen(descriptor, "rb")
        before = os.fstat(source.fileno())
        raw = source.read(limit + 1)
        after = os.fstat(source.fileno())
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    if (
        not stat.S_ISREG(named.st_mode)
        or stable_binding(named) != stable_binding(before)
        or file_identity(before) != file_identity(after)
        or before.st_nlink != 1
        or before.st_uid != os.geteuid()
        or stat.S_IMODE(before.st_mode) != mode
        or before.st_size <= 0
        or before.st_size > limit
        or len(raw) != before.st_size
    ):
        source.close()
        fail(f"{description} is not an exact stable owner-private file")
    if canonical_json:
        parse_canonical(raw, description)
    return name, source, after, raw


def revalidate_file_at(
    directory_fd: int, custody: FileCustody, description: str
) -> None:
    name, source, authenticated, raw = custody
    try:
        source.seek(0)
        current_raw = source.read(len(raw) + 1)
        current = os.fstat(source.fileno())
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        current_raw != raw
        or file_identity(current) != file_identity(authenticated)
        or stable_binding(named) != stable_binding(authenticated)
    ):
        fail(f"{description} changed after authentication")


def parse_canonical(raw: bytes, description: str) -> JsonObject:
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail(f"{description} must have one trailing newline")
    try:
        value = json.loads(raw, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"{description} is not canonical ASCII JSON: {error}")
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"{description} is not canonical JSON")
    return value


def read_regular(path: Path, limit: int, description: str) -> bytes:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before_named = path.lstat()
        descriptor = os.open(path, flags)
        with os.fdopen(descriptor, "rb") as source:
            before = os.fstat(source.fileno())
            raw = source.read(limit + 1)
            after = os.fstat(source.fileno())
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    if (
        not stat.S_ISREG(before_named.st_mode)
        or (before_named.st_dev, before_named.st_ino) != (before.st_dev, before.st_ino)
        or file_identity(before) != file_identity(after)
        or before.st_size <= 0
        or before.st_size > limit
        or len(raw) != before.st_size
    ):
        fail(f"{description} changed while it was read")
    return raw


def read_canonical(path: Path, description: str) -> tuple[JsonObject, bytes]:
    raw = read_regular(path, MAX_JSON_BYTES, description)
    return parse_canonical(raw, description), raw


def run(arguments: list[str], description: str, *, cwd: Path) -> str:
    result = subprocess.run(
        arguments,
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=300,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"{description} failed (status {result.returncode}):\n{result.stdout}")
    return result.stdout.strip()


def repository_identity(repository: Path, description: str) -> tuple[str, str]:
    if run(
        [
            "git",
            "-C",
            str(repository),
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
        ],
        f"inspect {description} source repository",
        cwd=repository,
    ):
        fail(f"{description} source repository is not clean")
    commit = run(
        ["git", "-C", str(repository), "rev-parse", "HEAD^{commit}"],
        f"identify {description} commit",
        cwd=repository,
    )
    tree = run(
        ["git", "-C", str(repository), "rev-parse", "HEAD^{tree}"],
        f"identify {description} tree",
        cwd=repository,
    )
    return require_git_id(commit, f"{description} commit"), require_git_id(
        tree, f"{description} tree"
    )


def requirement_spec(
    requirements: JsonObject, binding: JsonObject
) -> tuple[JsonObject, list[str]]:
    obligation_class = binding["obligation_class"]
    obligation_id = binding["obligation_id"]
    if obligation_class == "Roadmap":
        matches = [
            row
            for row in requirements["roadmap_requirements"]
            if row["id"] == obligation_id
        ]
        if len(matches) != 1:
            fail("independent-validator binding names an unknown roadmap obligation")
        return matches[0], matches[0]["assurance_properties"]
    if obligation_class == "Assurance":
        matches = [
            row
            for row in requirements["assurance_properties"]
            if row["name"] == obligation_id
        ]
        if len(matches) != 1:
            fail("independent-validator binding names an unknown assurance property")
        return matches[0], [obligation_id]
    fail("independent-validator obligation class drifted")


def property_bindings(
    requirements: JsonObject, identifiers: list[str]
) -> list[JsonObject]:
    by_name = {row["name"]: row for row in requirements["assurance_properties"]}
    result = []
    for identifier in identifiers:
        row = by_name.get(identifier)
        if row is None:
            fail("independent-validator assurance property is unknown")
        result.append(
            {
                "boundary_sha256": digest_bytes(row["boundary"].encode("utf-8")),
                "fe2o3_kind": row["fe2o3_kind"],
                "name": identifier,
                "obligation_state": "Open",
                "required_status_at_closure": row["required_status_at_closure"],
            }
        )
    return result


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
    return rows


def projected_profiles(requirements: JsonObject) -> list[JsonObject]:
    return [
        {"evidence_kinds": record["kinds"], "id": record["id"]}
        for record in requirements["evidence_profiles"]
    ]


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
    rust_toolchain = digest_bytes(
        read_regular(
            ferric / "rust-toolchain.toml", MAX_JSON_BYTES, "Rust toolchain pin"
        )
    )
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
    verus_closure = digest_bytes(
        read_regular(
            ferric / "proofs/verus/VERUS_CLOSURE_MANIFEST",
            MAX_MATERIAL_BYTES,
            "Verus closure manifest",
        )
    )
    rows = [
        component(
            "compiler.amdgpu-linker",
            "Compiler",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["compiler.amdgpu-linker", "qualification-bound-external", TARGET],
        ),
        component(
            "compiler.llvm-amdgpu",
            "Compiler",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["compiler.llvm-amdgpu", "qualification-bound-external", TARGET],
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
            TARGET,
            "Contracted",
            "single-device-target-only",
            ["hardware.gfx942", TARGET, "one-physical-device"],
        ),
        component(
            "runtime.amdgpu-firmware",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.amdgpu-firmware", "qualification-bound-external", TARGET],
        ),
        component(
            "runtime.amdgpu-kernel-driver",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.amdgpu-kernel-driver", "qualification-bound-external", TARGET],
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
            ["runtime.hsa", "qualification-bound-external", TARGET],
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


def report_validators(plan: JsonObject) -> list[JsonObject]:
    validators = plan.get("trusted_validators")
    if not isinstance(validators, list) or len(validators) != 12:
        fail("M1 trusted-validator roster drifted")
    return [
        {
            "availability": "ExistingFoundation",
            "evidence_kind": row["evidence_kind"],
            "path": row["path"],
            "protocol": row["protocol"],
            "source_sha256": row["source_sha256"],
        }
        for row in validators
    ]


def expected_tcb_report(
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
    identifier: str,
    kind: str,
) -> JsonObject:
    return {
        "authority": TCB_REPORT_AUTHORITY,
        "component_roster": component_roster(ferric, sources),
        "evidence_kind": "tcb-report",
        "format": "FERRIC-M1-TCB-REPORT-V1",
        "milestone": "M1",
        "nonclaim": TCB_REPORT_NONCLAIM,
        "obligation_roster": projected_obligations(requirements),
        "obligation_state": "Open",
        "path_roster": projected_paths(requirements),
        "profile_roster": projected_profiles(requirements),
        "requirements_sha256": digest_bytes(
            read_regular(
                ferric / "proofs/M1_REQUIREMENTS.json",
                MAX_JSON_BYTES,
                "M1 requirements manifest",
            )
        ),
        "source_roster": sources,
        "subject_tcb_id": identifier,
        "subject_tcb_kind": kind,
        "target": TARGET,
        "tcb_structure_roster": [
            {"artifact_id": f"artifact.{subject}", "id": subject, "kind": subject_kind}
            for subject, subject_kind in TCB
        ],
        "validator_roster": validators,
    }


def load_tcb(
    plan_root: Path,
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
) -> list[JsonObject]:
    result = []
    for identifier, kind in TCB:
        artifact_id = f"artifact.{identifier}"
        path = plan_root / "artifacts" / f"{artifact_id}.tcb-report.json"
        raw = read_regular(path, MAX_JSON_BYTES, f"M1 TCB report {identifier}")
        report = parse_canonical(raw, f"M1 TCB report {identifier}")
        expected = expected_tcb_report(
            ferric, requirements, sources, validators, identifier, kind
        )
        if report != expected or raw != canonical_bytes(expected):
            fail(
                f"M1 TCB report is not the exact authenticated projection: {identifier}"
            )
        result.append(
            {
                "artifact_id": artifact_id,
                "id": identifier,
                "identity_sha256": digest_bytes(raw),
                "kind": kind,
            }
        )
    if len({row["identity_sha256"] for row in result}) != 3:
        fail("M1 TCB report identities are not unique")
    return result


def hold_plan_inputs(
    plan_root: Path,
    plan_raw: bytes,
    queue_raw: bytes,
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
) -> tuple[
    int,
    os.stat_result,
    int,
    list[ChildDirectoryCustody],
    list[tuple[int, FileCustody, str]],
]:
    plan_fd, plan_identity = open_directory(plan_root, "M1 plan", private=True)
    directories: list[ChildDirectoryCustody] = []
    held: list[tuple[int, FileCustody, str]] = []
    descriptors = [plan_fd]
    try:
        for name, expected, description in (
            ("plan.json", plan_raw, "M1 evidence plan"),
            ("missing-work.json", queue_raw, "M1 evidence work queue"),
        ):
            custody = open_regular_at(
                plan_fd,
                name,
                MAX_JSON_BYTES,
                description,
                canonical_json=True,
            )
            held.append((plan_fd, custody, description))
            if custody[3] != expected:
                fail(f"{description} changed before custody acquisition")
        closure_fd, closure_identity = open_directory_at(
            plan_fd, "source-closures", "M1 source-closure directory"
        )
        descriptors.append(closure_fd)
        directories.append(
            (
                plan_fd,
                "source-closures",
                closure_fd,
                closure_identity,
                "M1 source-closure directory",
            )
        )
        for source in sources:
            name = f"{source['id']}.records"
            custody = open_regular_at(
                closure_fd,
                name,
                MAX_MATERIAL_BYTES,
                f"M1 {source['id']} source closure",
            )
            held.append((closure_fd, custody, f"M1 {source['id']} source closure"))
            if digest_bytes(custody[3]) != source["source_closure_sha256"]:
                fail(f"M1 source closure identity drifted: {source['id']}")
        artifact_fd, artifact_identity = open_directory_at(
            plan_fd, "artifacts", "M1 artifact directory"
        )
        descriptors.append(artifact_fd)
        directories.append(
            (
                plan_fd,
                "artifacts",
                artifact_fd,
                artifact_identity,
                "M1 artifact directory",
            )
        )
        for identifier, kind in TCB:
            name = f"artifact.{identifier}.tcb-report.json"
            custody = open_regular_at(
                artifact_fd,
                name,
                MAX_JSON_BYTES,
                f"M1 TCB report {identifier}",
                canonical_json=True,
            )
            held.append((artifact_fd, custody, f"M1 TCB report {identifier}"))
            expected = canonical_bytes(
                expected_tcb_report(
                    ferric, requirements, sources, validators, identifier, kind
                )
            )
            if custody[3] != expected:
                fail(
                    "M1 TCB report is not the exact authenticated projection: "
                    f"{identifier}"
                )
        return plan_fd, plan_identity, artifact_fd, directories, held
    except BaseException:
        for _, custody, _ in held:
            custody[1].close()
        for descriptor in reversed(descriptors):
            try:
                os.close(descriptor)
            except OSError:
                pass
        raise


def revalidate_child_custodies(custodies: list[ChildDirectoryCustody]) -> None:
    for parent_fd, name, descriptor, identity, description in custodies:
        revalidate_directory_at(parent_fd, name, descriptor, identity, description)


def load_plan(
    ferric: Path, fe2o3: Path, plan_root: Path, *, replay: bool
) -> tuple[JsonObject, JsonObject, JsonObject, bytes, bytes, list[JsonObject]]:
    requirements, requirements_raw = read_canonical(
        ferric / "proofs/M1_REQUIREMENTS.json", "M1 requirements manifest"
    )
    plan, plan_raw = read_canonical(plan_root / "plan.json", "M1 evidence plan")
    queue, queue_raw = read_canonical(
        plan_root / "missing-work.json", "M1 evidence work queue"
    )
    if (
        plan.get("format") != PLAN_FORMAT
        or plan.get("authority") != PLAN_AUTHORITY
        or plan.get("target") != TARGET
        or queue.get("format") != WORK_FORMAT
        or queue.get("authority") != PLAN_AUTHORITY
        or queue.get("status") != "INCOMPLETE"
        or queue.get("plan_sha256") != digest_bytes(plan_raw)
        or plan.get("requirements", {}).get("sha256") != digest_bytes(requirements_raw)
        or not isinstance(plan.get("binding_slots"), list)
        or len(plan["binding_slots"]) != 354
        or not isinstance(queue.get("items"), list)
        or len(queue["items"]) != 358
        or queue.get("counts")
        != {
            "available_producer_items": 358,
            "missing_items": 358,
            "missing_producer_items": 0,
        }
    ):
        fail("M1 plan or work queue identity drifted")
    sources = plan.get("sources")
    if not isinstance(sources, list) or [row.get("id") for row in sources] != list(
        SOURCE_IDS
    ):
        fail("M1 source roster drifted")
    identities = {
        "fe2o3": repository_identity(fe2o3, "fe2o3"),
        "ferric": repository_identity(ferric, "Ferric"),
    }
    for row in sources:
        repository = row.get("repository")
        if repository not in identities:
            fail("M1 source repository drifted")
        require_git_id(row.get("base_commit"), f"{repository} base commit")
        require_sha256(row.get("source_closure_sha256"), f"{repository} source closure")
        if (row.get("commit"), row.get("tree")) != identities[repository]:
            fail(f"M1 {repository} source identity changed after planning")
        closure_path = plan_root / "source-closures" / f"source.{repository}.records"
        if (
            digest_bytes(
                read_regular(closure_path, MAX_MATERIAL_BYTES, f"{repository} closure")
            )
            != row["source_closure_sha256"]
        ):
            fail(f"M1 {repository} source closure identity drifted")
    if (
        sources[0]["base_commit"] != requirements["m1_upstream_base_commit"]
        or sources[1]["base_commit"] != FERRIC_BASE_COMMIT
    ):
        fail("M1 source base identity drifted")
    if any(
        (plan_root / name).exists() for name in ("evidence-index.json", "receipt.json")
    ):
        fail(
            "independent-validator production refuses a plan containing a closure output"
        )
    if replay:
        with tempfile.TemporaryDirectory(
            prefix="ferric-m1-independent-plan-replay-"
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
            )
            if (
                read_regular(reproduced / "plan.json", MAX_JSON_BYTES, "rederived plan")
                != plan_raw
                or read_regular(
                    reproduced / "missing-work.json", MAX_JSON_BYTES, "rederived queue"
                )
                != queue_raw
            ):
                fail("M1 plan or work queue differs from exact rederivation")
            for source_id in SOURCE_IDS:
                name = f"{source_id}.records"
                if read_regular(
                    reproduced / "source-closures" / name,
                    MAX_MATERIAL_BYTES,
                    f"rederived {source_id} closure",
                ) != read_regular(
                    plan_root / "source-closures" / name,
                    MAX_MATERIAL_BYTES,
                    f"candidate {source_id} closure",
                ):
                    fail(f"M1 source closure differs from rederivation: {source_id}")
    return requirements, plan, queue, plan_raw, queue_raw, copy.deepcopy(sources)


def independent_slots(plan: JsonObject, queue: JsonObject) -> list[JsonObject]:
    slots = [
        slot
        for slot in plan["binding_slots"]
        if slot.get("binding", {}).get("evidence_kind") == "independent-validator"
    ]
    ids = [slot["binding"]["id"] for slot in slots]
    if (
        len(slots) != 44
        or ids != sorted(ids)
        or digest_bytes(("\n".join(ids) + "\n").encode("ascii"))
        != INDEPENDENT_ROSTER_SHA256
    ):
        fail("M1 independent-validator binding roster drifted")
    queue_by_id = {row.get("id"): row for row in queue["items"]}
    topology = []
    for slot in slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        artifact = {
            "id": artifact_id,
            "kind": "ValidatorTranscript",
            "path": f"artifacts/{artifact_id}.independent-validator.json",
        }
        work_id = binding["id"].replace("binding.", "work.", 1)
        work = queue_by_id.get(work_id)
        producer = {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                PRODUCER_PATH,
                "intake",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                "INDEPENDENT_REVIEW_ROOT",
                binding["id"],
            ],
            "role": PRODUCER_ROLE,
        }
        payload = {
            key: value for key, value in binding.items() if key != "binding_sha256"
        }
        if (
            canonical_digest(payload) != binding.get("binding_sha256")
            or binding.get("obligation_class") not in {"Assurance", "Roadmap"}
            or binding.get("profile_id")
            not in {"authentication", "kernel", "runtime", "qualification"}
            or binding.get("source_identity_id") not in SOURCE_IDS
            or binding.get("tcb_ids") != [identifier for identifier, _ in TCB]
            or slot.get("expected_artifact") != artifact
            or slot.get("producer") != producer
            or slot.get("state") != "missing"
            or not isinstance(work, dict)
            or work.get("expected_artifact") != artifact
            or work.get("producer") != producer
            or work.get("state") != "missing"
            or work.get("subject") != f"binding:{binding['id']}"
        ):
            fail(f"M1 independent-validator allocation drifted: {binding.get('id')}")
        topology.append(
            "|".join(
                [
                    binding["id"],
                    binding["obligation_class"],
                    binding["obligation_id"],
                    binding["profile_id"],
                    binding["path_id"],
                    binding["source_identity_id"],
                ]
            )
            + "\n"
        )
    if digest_bytes("".join(topology).encode("ascii")) != INDEPENDENT_TOPOLOGY_SHA256:
        fail("M1 independent-validator allocation topology drifted")
    return slots


def path_resolution(plan: JsonObject, binding: JsonObject) -> JsonObject:
    matches = [
        row for row in plan["path_resolutions"] if row["id"] == binding["path_id"]
    ]
    if len(matches) != 1:
        fail("selected independent-validator path resolution is missing")
    resolution = copy.deepcopy(matches[0])
    if (
        resolution.get("source_identity_id") != binding["source_identity_id"]
        or binding["source_identity_id"] != f"source.{resolution.get('repository')}"
    ):
        fail("selected independent-validator path resolution drifted")
    return resolution


def substituted_hash(domain: str, original: str) -> str:
    return digest_bytes((domain + "\0" + original).encode("ascii"))


def case_inputs(
    requirements: JsonObject,
    binding: JsonObject,
    properties: list[JsonObject],
    resolution: JsonObject,
    sources: list[JsonObject],
    tcb: list[JsonObject],
    requirements_sha256: str,
) -> list[tuple[str, str, bytes]]:
    subject = {
        "assurance_property_bindings": properties,
        "binding": binding,
        "path_resolution": resolution,
        "requirements_sha256": requirements_sha256,
        "sources": sources,
        "target": TARGET,
        "tcb": tcb,
        "validation_status": "PASS",
    }
    result = []
    for identifier, expected in CASE_MATRIX:
        candidate = copy.deepcopy(subject)
        operation = "validate-subject"
        if identifier == "boundary-conforming-subject":
            operation = "validate-boundary-conforming-subject"
        elif identifier == "obligation-substitution":
            candidate["binding"]["obligation_id"] = "substituted-obligation"
        elif identifier == "property-substitution":
            candidate["assurance_property_bindings"] = []
        elif identifier == "path-substitution":
            candidate["path_resolution"]["id"] = "substituted-path"
        elif identifier == "profile-substitution":
            candidate["binding"]["profile_id"] = "substituted-profile"
        elif identifier == "source-closure-substitution":
            candidate["sources"][0]["source_closure_sha256"] = substituted_hash(
                identifier, candidate["sources"][0]["source_closure_sha256"]
            )
        elif identifier == "target-substitution":
            candidate["target"] = "gfx950:xnack-"
        elif identifier == "tcb-substitution":
            candidate["tcb"][0]["identity_sha256"] = substituted_hash(
                identifier, candidate["tcb"][0]["identity_sha256"]
            )
        elif identifier == "malformed-status":
            candidate["validation_status"] = "MALFORMED"
        value = {
            "case_id": identifier,
            "expected_status": expected,
            "format": REQUEST_FORMAT + "-CASE",
            "operation": operation,
            "protocol": VALIDATOR_PROTOCOL,
            "subject": candidate,
        }
        result.append((identifier, expected, canonical_bytes(value)))
    hashes = [digest_bytes(raw) for _, _, raw in result]
    if len(hashes) != len(set(hashes)):
        fail("independent-validator case input identities are not unique")
    return result


def request_for(
    requirements: JsonObject,
    plan: JsonObject,
    binding: JsonObject,
    sources: list[JsonObject],
    tcb: list[JsonObject],
) -> tuple[JsonObject, bytes, list[tuple[str, str, bytes]]]:
    _, property_ids = requirement_spec(requirements, binding)
    properties = property_bindings(requirements, property_ids)
    resolution = path_resolution(plan, binding)
    requirements_sha256 = plan["requirements"]["sha256"]
    inputs = case_inputs(
        requirements,
        binding,
        properties,
        resolution,
        sources,
        tcb,
        requirements_sha256,
    )
    cases = [
        {
            "expected_status": expected,
            "id": identifier,
            "input_path": f"cases/{identifier}.input.json",
            "input_sha256": digest_bytes(raw),
            "input_size_bytes": len(raw),
        }
        for identifier, expected, raw in inputs
    ]
    request = {
        "assurance_property_bindings": properties,
        "binding": binding,
        "cases": cases,
        "format": REQUEST_FORMAT,
        "path_resolution": resolution,
        "protocol": VALIDATOR_PROTOCOL,
        "requirements_sha256": requirements_sha256,
        "sources": sources,
        "target": TARGET,
        "tcb": tcb,
    }
    return request, canonical_bytes(request), inputs


def create_handoff_directory_at(
    parent_fd: int, name: str, description: str
) -> tuple[int, os.stat_result]:
    if SAFE_SEGMENT.fullmatch(name) is None:
        fail(f"unsafe {description} name")
    mkdir_succeeded = False
    created: os.stat_result | None = None
    descriptor = -1
    try:
        os.mkdir(name, 0o700, dir_fd=parent_fd)
        mkdir_succeeded = True
        created = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        if (
            not stat.S_ISDIR(created.st_mode)
            or stat.S_IMODE(created.st_mode) != 0o700
            or created.st_uid != os.geteuid()
        ):
            fail(f"new {description} is not an exact owner-private directory")
        os.fsync(parent_fd)
        descriptor, opened = open_directory_at(parent_fd, name, description)
        if directory_binding(created) != directory_binding(opened):
            fail(f"new {description} binding changed while it was opened")
        return descriptor, opened
    except BaseException as error:
        rollback_failure: str | None = None
        if descriptor >= 0:
            rollback_failure = rollback_exact_directory(
                parent_fd, name, descriptor, description
            )
            os.close(descriptor)
        elif created is not None:
            try:
                named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
                if directory_binding(named) != directory_binding(created):
                    rollback_failure = f"cannot remove replaced failed {description}"
                else:
                    os.rmdir(name, dir_fd=parent_fd)
                    os.fsync(parent_fd)
            except FileNotFoundError:
                pass
            except OSError as rollback_error:
                rollback_failure = (
                    f"cannot remove failed {description}: {rollback_error}"
                )
        elif mkdir_succeeded:
            rollback_failure = (
                f"cannot identify failed {description} for exact rollback"
            )
        if rollback_failure is not None:
            fail(f"{description} creation rollback failed: {rollback_failure}")
        if isinstance(error, OSError):
            fail(f"cannot create {description} without replacement: {error}")
        raise


def rollback_exact_directory(
    parent_fd: int,
    name: str,
    descriptor: int,
    description: str,
) -> str | None:
    try:
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        held = os.fstat(descriptor)
    except FileNotFoundError:
        return None
    except OSError as error:
        return f"cannot inspect failed {description}: {error}"
    if directory_binding(named) != directory_binding(held):
        return f"cannot remove replaced failed {description}"
    try:
        os.rmdir(name, dir_fd=parent_fd)
        os.fsync(parent_fd)
    except OSError as error:
        return f"cannot remove failed {description}: {error}"
    return None


def publish_handoff(
    parent_fd: int,
    handoff_name: str,
    packages: list[tuple[JsonObject, bytes, list[tuple[str, str, bytes]]]],
    manifest_raw: bytes,
    custody_check: Any,
) -> None:
    directories: list[ChildDirectoryCustody] = []
    files: list[tuple[int, str, int, os.stat_result, bytes, str]] = []
    try:
        custody_check()
        root_fd, root_identity = create_handoff_directory_at(
            parent_fd, handoff_name, "independent-review handoff root"
        )
        directories.append(
            (
                parent_fd,
                handoff_name,
                root_fd,
                root_identity,
                "independent-review handoff root",
            )
        )
        requests_fd, requests_identity = create_handoff_directory_at(
            root_fd, "requests", "independent-review request directory"
        )
        directories.append(
            (
                root_fd,
                "requests",
                requests_fd,
                requests_identity,
                "independent-review request directory",
            )
        )
        for request, request_raw, inputs in packages:
            binding = request["binding"]
            custody_check()
            binding_fd, binding_identity = create_handoff_directory_at(
                requests_fd, binding["id"], f"request {binding['id']}"
            )
            directories.append(
                (
                    requests_fd,
                    binding["id"],
                    binding_fd,
                    binding_identity,
                    f"request {binding['id']}",
                )
            )
            cases_fd, cases_identity = create_handoff_directory_at(
                binding_fd, "cases", f"request cases {binding['id']}"
            )
            directories.append(
                (
                    binding_fd,
                    "cases",
                    cases_fd,
                    cases_identity,
                    f"request cases {binding['id']}",
                )
            )
            for identifier, _, raw in inputs:
                name = f"{identifier}.input.json"
                description = f"request case {binding['id']} {identifier}"
                descriptor, identity = create_new_at(cases_fd, name, raw, description)
                files.append((cases_fd, name, descriptor, identity, raw, description))
            descriptor, identity = create_new_at(
                binding_fd, "request.json", request_raw, f"request {binding['id']}"
            )
            files.append(
                (
                    binding_fd,
                    "request.json",
                    descriptor,
                    identity,
                    request_raw,
                    f"request {binding['id']}",
                )
            )
        custody_check()
        descriptor, identity = create_new_at(
            root_fd, "handoff.json", manifest_raw, "handoff manifest"
        )
        files.append(
            (
                root_fd,
                "handoff.json",
                descriptor,
                identity,
                manifest_raw,
                "handoff manifest",
            )
        )
        for _, _, directory_fd, _, _ in reversed(directories):
            os.fsync(directory_fd)
        os.fsync(parent_fd)
        custody_check()
        revalidate_child_custodies(directories)
        for directory_fd, name, descriptor, identity, raw, description in files:
            verify_published(directory_fd, name, descriptor, identity, raw, description)
    except BaseException:
        failures = []
        for directory_fd, name, descriptor, _, _, description in reversed(files):
            failure = rollback_exact(directory_fd, name, descriptor, description)
            if failure is not None:
                failures.append(failure)
        for parent, name, descriptor, _, description in reversed(directories):
            failure = rollback_exact_directory(parent, name, descriptor, description)
            if failure is not None:
                failures.append(failure)
        if failures:
            fail(
                "independent-review handoff rollback failures: " + " | ".join(failures)
            )
        raise
    finally:
        for _, _, descriptor, _, _, _ in files:
            try:
                os.close(descriptor)
            except OSError:
                pass
        for _, _, descriptor, _, _ in reversed(directories):
            try:
                os.close(descriptor)
            except OSError:
                pass


def _export_all(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    handoff_argument: str,
    absolute_custodies: list[AbsoluteDirectoryCustody],
) -> None:
    ferric = canonical_root(ferric_argument, "Ferric repository", private=False)
    fe2o3 = canonical_root(fe2o3_argument, "fe2o3 repository", private=False)
    plan_root = canonical_root(plan_argument, "M1 plan", private=True)
    handoff = Path(handoff_argument).absolute()
    if handoff.exists() or handoff.resolve(strict=False) != handoff:
        fail("independent-review handoff output must be a new canonical path")
    for root in (ferric, fe2o3, plan_root):
        if handoff == root or root in handoff.parents or handoff in root.parents:
            fail("independent-review handoff output overlaps an authenticated root")
    requirements, plan, queue, plan_raw, queue_raw, sources = load_plan(
        ferric, fe2o3, plan_root, replay=True
    )
    slots = independent_slots(plan, queue)
    validators = report_validators(plan)
    tcb = load_tcb(plan_root, ferric, requirements, sources, validators)
    manifest_rows = []
    packages = []
    for slot in slots:
        binding = slot["binding"]
        request, request_raw, inputs = request_for(
            requirements, plan, binding, sources, tcb
        )
        packages.append((request, request_raw, inputs))
        manifest_rows.append(
            {
                "binding_id": binding["id"],
                "binding_sha256": binding["binding_sha256"],
                "request_path": f"requests/{binding['id']}/request.json",
                "request_sha256": digest_bytes(request_raw),
                "request_size_bytes": len(request_raw),
            }
        )
        if request["cases"][-1]["id"] != CASE_MATRIX[-1][0]:
            fail("independent-review request case order drifted")
    manifest = {
        "format": HANDOFF_FORMAT,
        "plan_sha256": digest_bytes(plan_raw),
        "protocol": VALIDATOR_PROTOCOL,
        "requests": manifest_rows,
        "target": TARGET,
    }
    plan_fd = artifact_fd = ferric_fd = fe2o3_fd = parent_fd = -1
    plan_identity: os.stat_result | None = None
    plan_directories: list[ChildDirectoryCustody] = []
    plan_held: list[tuple[int, FileCustody, str]] = []
    parent = canonical_root(str(handoff.parent), "handoff parent", private=True)
    try:
        (
            plan_fd,
            plan_identity,
            artifact_fd,
            plan_directories,
            plan_held,
        ) = hold_plan_inputs(
            plan_root,
            plan_raw,
            queue_raw,
            ferric,
            requirements,
            sources,
            validators,
        )
        ferric_fd, _ = open_directory(ferric, "Ferric repository", private=False)
        fe2o3_fd, _ = open_directory(fe2o3, "fe2o3 repository", private=False)
        parent_fd, _ = open_directory(parent, "handoff parent", private=True)

        def custody_check() -> None:
            if plan_identity is None:
                fail("independent-review export custody was not initialized")
            reject_closure_outputs(plan_fd)
            for absolute_custody in absolute_custodies:
                revalidate_absolute_directory(absolute_custody)
            revalidate_directory(plan_root, plan_fd, plan_identity, "M1 plan")
            revalidate_child_custodies(plan_directories)
            for directory_fd, custody, description in plan_held:
                revalidate_file_at(directory_fd, custody, description)
            if repository_identity(ferric, "Ferric") != (
                sources[1]["commit"],
                sources[1]["tree"],
            ) or repository_identity(fe2o3, "fe2o3") != (
                sources[0]["commit"],
                sources[0]["tree"],
            ):
                fail("subject source identity changed during independent-review export")

        publish_handoff(
            parent_fd,
            handoff.name,
            packages,
            canonical_bytes(manifest),
            custody_check,
        )
    finally:
        for _, custody, _ in plan_held:
            custody[1].close()
        descriptors = {descriptor for _, _, descriptor, _, _ in plan_directories}
        descriptors.update(
            descriptor
            for descriptor in (plan_fd, artifact_fd, ferric_fd, fe2o3_fd, parent_fd)
            if descriptor >= 0
        )
        for descriptor in descriptors:
            try:
                os.close(descriptor)
            except OSError:
                pass
    print(
        "PASS: exported 44 independent-review requests and 440 case inputs "
        f"handoff_sha256={digest_bytes(canonical_bytes(manifest))}"
    )


def export_all(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    handoff_argument: str,
) -> None:
    ferric = canonical_root(ferric_argument, "Ferric repository", private=False)
    fe2o3 = canonical_root(fe2o3_argument, "fe2o3 repository", private=False)
    plan = canonical_root(plan_argument, "M1 plan", private=True)
    handoff = Path(handoff_argument).absolute()
    parent = canonical_root(str(handoff.parent), "handoff parent", private=True)
    roots = (
        (ferric, "Ferric repository", False),
        (fe2o3, "fe2o3 repository", False),
        (plan, "M1 plan", True),
        (parent, "handoff parent", True),
    )
    custodies: list[AbsoluteDirectoryCustody] = []
    try:
        for path, description, private in roots:
            custodies.append(
                open_absolute_directory(path, description, private=private)
            )
        _export_all(
            ferric_argument,
            fe2o3_argument,
            plan_argument,
            handoff_argument,
            custodies,
        )
    finally:
        for custody in reversed(custodies):
            close_absolute_directory(custody)


def validate_checker(checker: Any, sources: list[JsonObject]) -> JsonObject:
    value = exact_keys(checker, CHECKER_KEYS, "independent checker identity")
    require_id(value["id"], "independent checker id")
    organization = require_id(value["organization"], "checker organization")
    repository = require_id(value["repository"], "checker repository")
    commit = require_git_id(value["commit"], "checker commit")
    tree = require_git_id(value["tree"], "checker tree")
    closure = require_sha256(value["source_closure_sha256"], "checker source closure")
    executable = safe_relative(value["executable_path"], "checker executable path")
    executable_sha = require_sha256(value["executable_sha256"], "checker executable")
    require_sha256(value["input_schema_sha256"], "checker input schema")
    require_sha256(value["output_schema_sha256"], "checker output schema")
    if (
        not isinstance(value["version"], str)
        or VERSION.fullmatch(value["version"]) is None
    ):
        fail("invalid checker version")
    if value["protocol"] != VALIDATOR_PROTOCOL:
        fail("independent checker protocol drifted")
    if (
        organization in SUBJECT_ORGANIZATIONS
        or repository in SUBJECT_REPOSITORIES
        or value["id"] in SOURCE_IDS
        or executable.as_posix() == TRUSTED_CHECKER_PATH
        or commit in {row["commit"] for row in sources}
        or tree in {row["tree"] for row in sources}
        or closure in {row["source_closure_sha256"] for row in sources}
        or executable_sha in {row["source_closure_sha256"] for row in sources}
    ):
        fail("self-validation or subject-source checker substitution detected")
    identities = [
        value["source_closure_sha256"],
        value["executable_sha256"],
        value["input_schema_sha256"],
        value["output_schema_sha256"],
    ]
    if len(identities) != len(set(identities)):
        fail("independent checker material identities are not distinct")
    return value


def authenticate_response(
    response_root: Path,
    binding: JsonObject,
    request: JsonObject,
    request_raw: bytes,
    sources: list[JsonObject],
) -> tuple[
    JsonObject,
    list[JsonObject],
    list[tuple[int, FileCustody, str]],
    list[int],
    os.stat_result,
    list[ChildDirectoryCustody],
]:
    root_fd, root_identity = open_directory(
        response_root, "external independent-review response root", private=True
    )
    descriptors = [root_fd]
    held: list[tuple[int, FileCustody, str]] = []
    directories: list[ChildDirectoryCustody] = []
    try:
        if set(os.listdir(root_fd)) != {"responses"}:
            fail("external response root inventory drifted")
        responses_fd, responses_identity = open_directory_at(
            root_fd, "responses", "external response directory"
        )
        descriptors.append(responses_fd)
        directories.append(
            (
                root_fd,
                "responses",
                responses_fd,
                responses_identity,
                "external response directory",
            )
        )
        binding_fd, binding_identity = open_directory_at(
            responses_fd, binding["id"], "binding response directory"
        )
        descriptors.append(binding_fd)
        directories.append(
            (
                responses_fd,
                binding["id"],
                binding_fd,
                binding_identity,
                "binding response directory",
            )
        )
        if set(os.listdir(binding_fd)) != {"checker", "outputs", "response.json"}:
            fail("binding response inventory drifted")
        checker_fd, checker_identity = open_directory_at(
            binding_fd, "checker", "checker material directory"
        )
        outputs_fd, outputs_identity = open_directory_at(
            binding_fd, "outputs", "checker output directory"
        )
        descriptors.extend([checker_fd, outputs_fd])
        directories.extend(
            [
                (
                    binding_fd,
                    "checker",
                    checker_fd,
                    checker_identity,
                    "checker material directory",
                ),
                (
                    binding_fd,
                    "outputs",
                    outputs_fd,
                    outputs_identity,
                    "checker output directory",
                ),
            ]
        )
        expected_checker = {
            "executable.bin",
            "input-schema.json",
            "output-schema.json",
            "source-closure.records",
        }
        if set(os.listdir(checker_fd)) != expected_checker:
            fail("checker material inventory drifted")
        expected_outputs = {f"{identifier}.output" for identifier, _ in CASE_MATRIX}
        if set(os.listdir(outputs_fd)) != expected_outputs:
            fail("checker output inventory drifted")
        response_custody = open_regular_at(
            binding_fd,
            "response.json",
            MAX_JSON_BYTES,
            "external response manifest",
            canonical_json=True,
        )
        held.append((binding_fd, response_custody, "external response manifest"))
        response = exact_keys(
            parse_canonical(response_custody[3], "external response manifest"),
            RESPONSE_KEYS,
            "external response manifest",
        )
        checker = validate_checker(response["checker"], sources)
        materials = (
            ("source-closure.records", "source_closure_sha256", 0o600),
            ("executable.bin", "executable_sha256", 0o700),
            ("input-schema.json", "input_schema_sha256", 0o600),
            ("output-schema.json", "output_schema_sha256", 0o600),
        )
        for name, identity_key, mode in materials:
            custody = open_regular_at(
                checker_fd,
                name,
                MAX_MATERIAL_BYTES,
                f"checker material {name}",
                mode=mode,
            )
            held.append((checker_fd, custody, f"checker material {name}"))
            if digest_bytes(custody[3]) != checker[identity_key]:
                fail(f"checker material identity drifted: {name}")
        if (
            response["format"] != RESPONSE_FORMAT
            or response["binding_sha256"] != binding["binding_sha256"]
            or response["request_sha256"] != digest_bytes(request_raw)
            or response["independence_attestation"] != INDEPENDENCE_ATTESTATION
        ):
            fail("external response binding or attestation drifted")
        started = response["started_at_utc"]
        completed = response["completed_at_utc"]
        if (
            not isinstance(started, str)
            or UTC_TIME.fullmatch(started) is None
            or not isinstance(completed, str)
            or UTC_TIME.fullmatch(completed) is None
            or completed < started
        ):
            fail("external response time is malformed")
        results = response["results"]
        if not isinstance(results, list) or len(results) != len(CASE_MATRIX):
            fail("external response result roster is incomplete")
        inputs = {row["id"]: row for row in request["cases"]}
        normalized = []
        output_hashes = []
        for result, (identifier, expected) in zip(results, CASE_MATRIX, strict=True):
            exact_keys(result, RESPONSE_RESULT_KEYS, f"external result {identifier}")
            expected_exit = 0 if expected == "PASS" else 1
            output_name = f"{identifier}.output"
            if (
                result["id"] != identifier
                or result["expected_status"] != expected
                or result["observed_status"] != expected
                or result["exit_code"] != expected_exit
                or isinstance(result["exit_code"], bool)
                or result["input_sha256"] != inputs[identifier]["input_sha256"]
                or result["output_path"] != f"outputs/{output_name}"
                or not isinstance(result["output_size_bytes"], int)
                or isinstance(result["output_size_bytes"], bool)
                or result["output_size_bytes"] <= 0
            ):
                fail(f"external response result drifted: {identifier}")
            output_custody = open_regular_at(
                outputs_fd,
                output_name,
                MAX_OUTPUT_BYTES,
                f"external checker output {identifier}",
            )
            held.append(
                (outputs_fd, output_custody, f"external checker output {identifier}")
            )
            output_sha = digest_bytes(output_custody[3])
            if (
                result["output_size_bytes"] != len(output_custody[3])
                or require_sha256(result["output_sha256"], f"{identifier} output")
                != output_sha
            ):
                fail(f"external checker output identity drifted: {identifier}")
            output_hashes.append(output_sha)
            normalized.append(
                {
                    "exit_code": expected_exit,
                    "expected_status": expected,
                    "id": identifier,
                    "input_sha256": inputs[identifier]["input_sha256"],
                    "observed_status": expected,
                    "output_sha256": output_sha,
                }
            )
        input_hashes = [row["input_sha256"] for row in request["cases"]]
        if len(output_hashes) != len(set(output_hashes)) or set(input_hashes) & set(
            output_hashes
        ):
            fail("external case input/output identities are not exact")
        revalidate_directory_at(
            binding_fd, "checker", checker_fd, checker_identity, "checker material"
        )
        revalidate_directory_at(
            binding_fd, "outputs", outputs_fd, outputs_identity, "checker output"
        )
        revalidate_directory_at(
            responses_fd,
            binding["id"],
            binding_fd,
            binding_identity,
            "binding response",
        )
        revalidate_directory_at(
            root_fd, "responses", responses_fd, responses_identity, "external response"
        )
        revalidate_directory(
            response_root, root_fd, root_identity, "external response root"
        )
        for directory_fd, custody, description in held:
            revalidate_file_at(directory_fd, custody, description)
        return (
            response,
            normalized,
            held,
            descriptors,
            root_identity,
            directories,
        )
    except BaseException:
        for _, custody, _ in held:
            custody[1].close()
        for descriptor in reversed(descriptors):
            try:
                os.close(descriptor)
            except OSError:
                pass
        raise


def report_payloads(
    requirements: JsonObject,
    plan: JsonObject,
    binding: JsonObject,
    sources: list[JsonObject],
    tcb: list[JsonObject],
    request: JsonObject,
    response: JsonObject,
    results: list[JsonObject],
) -> tuple[bytes, bytes, bytes]:
    properties = request["assurance_property_bindings"]
    resolution = request["path_resolution"]
    checker = response["checker"]
    cases = [
        {
            "expected_status": row["expected_status"],
            "id": row["id"],
            "input_sha256": row["input_sha256"],
            "output_sha256": result["output_sha256"],
        }
        for row, result in zip(request["cases"], results, strict=True)
    ]
    roster = {
        "assurance_property_bindings_sha256": canonical_digest(properties),
        "binding_sha256": binding["binding_sha256"],
        "cases": cases,
        "checker": checker,
        "format": ROSTER_FORMAT,
        "path_resolution_sha256": canonical_digest(resolution),
        "profile_id": binding["profile_id"],
        "requirements_sha256": plan["requirements"]["sha256"],
        "source_roster_sha256": canonical_digest(sources),
        "target": TARGET,
        "tcb_roster_sha256": canonical_digest(tcb),
    }
    roster_raw = canonical_bytes(roster)
    transcript = {
        "binding_sha256": binding["binding_sha256"],
        "case_counts": copy.deepcopy(CASE_COUNTS),
        "checker_identity_sha256": canonical_digest(checker),
        "completed_at_utc": response["completed_at_utc"],
        "format": TRANSCRIPT_FORMAT,
        "results": results,
        "roster_sha256": digest_bytes(roster_raw),
        "started_at_utc": response["started_at_utc"],
        "validation_status": "PASS",
    }
    transcript_raw = canonical_bytes(transcript)
    artifact_id = binding["artifact_id"]
    report = {
        "assurance_property_bindings": properties,
        "authority": AUTHORITY,
        "binding_sha256": binding["binding_sha256"],
        "case_counts": copy.deepcopy(CASE_COUNTS),
        "checker_id": checker["id"],
        "checker_identity_sha256": canonical_digest(checker),
        "checker_organization": checker["organization"],
        "evidence_kind": "independent-validator",
        "format": REPORT_FORMAT,
        "independence_attestation": INDEPENDENCE_ATTESTATION,
        "nonclaim": NONCLAIM,
        "obligation_class": binding["obligation_class"],
        "obligation_id": binding["obligation_id"],
        "obligation_state": "Open",
        "path_id": binding["path_id"],
        "path_resolution_sha256": canonical_digest(resolution),
        "profile_id": binding["profile_id"],
        "requirements_sha256": plan["requirements"]["sha256"],
        "roster_path": f"validator-runs/{artifact_id}.independent-validator.roster.json",
        "roster_sha256": digest_bytes(roster_raw),
        "roster_size_bytes": len(roster_raw),
        "source_closure_sha256s": {
            row["id"]: row["source_closure_sha256"] for row in sources
        },
        "source_identity_id": binding["source_identity_id"],
        "source_identity_sha256s": {
            row["id"]: canonical_digest(row) for row in sources
        },
        "source_roster_sha256": canonical_digest(sources),
        "statement_sha256": binding["statement_sha256"],
        "target": TARGET,
        "tcb_identity_sha256s": {row["id"]: row["identity_sha256"] for row in tcb},
        "tcb_roster_sha256": canonical_digest(tcb),
        "transcript_path": f"validator-runs/{artifact_id}.independent-validator.transcript.json",
        "transcript_sha256": digest_bytes(transcript_raw),
        "transcript_size_bytes": len(transcript_raw),
        "validation_status": "PASS",
    }
    return roster_raw, transcript_raw, canonical_bytes(report)


def ensure_child_directory(
    parent_fd: int, name: str, description: str
) -> tuple[int, bool]:
    created = False
    created_identity: os.stat_result | None = None
    descriptor = -1
    try:
        os.mkdir(name, 0o700, dir_fd=parent_fd)
        created = True
        created_identity = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        if (
            not stat.S_ISDIR(created_identity.st_mode)
            or stat.S_IMODE(created_identity.st_mode) != 0o700
            or created_identity.st_uid != os.geteuid()
        ):
            fail(f"new {description} is not an exact owner-private directory")
        os.fsync(parent_fd)
    except FileExistsError:
        pass
    except BaseException as error:
        if not created:
            if isinstance(error, OSError):
                fail(f"cannot create {description}: {error}")
            raise
        rollback_failure = None
        if created_identity is None:
            rollback_failure = (
                f"cannot identify failed {description} for exact rollback"
            )
        else:
            try:
                named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
                if directory_binding(named) != directory_binding(created_identity):
                    rollback_failure = f"cannot remove replaced failed {description}"
                else:
                    os.rmdir(name, dir_fd=parent_fd)
                    os.fsync(parent_fd)
            except FileNotFoundError:
                pass
            except OSError as rollback_error:
                rollback_failure = (
                    f"cannot remove failed {description}: {rollback_error}"
                )
        if rollback_failure is not None:
            fail(f"{description} creation rollback failed: {rollback_failure}")
        if isinstance(error, OSError):
            fail(f"cannot initialize {description}: {error}")
        raise
    try:
        descriptor, opened = open_directory_at(parent_fd, name, description)
        if created_identity is not None and directory_binding(
            created_identity
        ) != directory_binding(opened):
            fail(f"new {description} binding changed while it was opened")
        return descriptor, created
    except BaseException:
        rollback_failure = None
        if created and descriptor >= 0:
            rollback_failure = rollback_exact_directory(
                parent_fd, name, descriptor, description
            )
        elif created and created_identity is not None:
            try:
                named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
                if directory_binding(named) != directory_binding(created_identity):
                    rollback_failure = f"cannot remove replaced failed {description}"
                else:
                    os.rmdir(name, dir_fd=parent_fd)
                    os.fsync(parent_fd)
            except FileNotFoundError:
                pass
            except OSError as error:
                rollback_failure = f"cannot remove failed {description}: {error}"
        elif created:
            rollback_failure = (
                f"cannot identify failed {description} for exact rollback"
            )
        if descriptor >= 0:
            os.close(descriptor)
        if rollback_failure is not None:
            fail(f"{description} creation rollback failed: {rollback_failure}")
        raise


def reject_closure_outputs(plan_fd: int) -> None:
    for name in ("evidence-index.json", "receipt.json"):
        try:
            os.stat(name, dir_fd=plan_fd, follow_symlinks=False)
        except FileNotFoundError:
            continue
        except OSError as error:
            fail(f"cannot inspect forbidden M1 closure output {name}: {error}")
        fail(
            "independent-validator production refuses a plan containing a closure output"
        )


def create_new_at(
    directory_fd: int, name: str, raw: bytes, description: str
) -> tuple[int, os.stat_result]:
    flags = os.O_RDWR | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    descriptor = -1
    try:
        descriptor = os.open(name, flags, 0o600, dir_fd=directory_fd)
        remaining = memoryview(raw)
        while remaining:
            written = os.write(descriptor, remaining)
            if written <= 0:
                fail(f"cannot completely write {description}")
            remaining = remaining[written:]
        os.fsync(descriptor)
        metadata = os.fstat(descriptor)
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except BaseException as error:
        if descriptor >= 0:
            failure = rollback_exact(directory_fd, name, descriptor, description)
            os.close(descriptor)
            if failure is not None:
                fail(
                    f"cannot create {description}: {error}; rollback failed: {failure}"
                )
        if isinstance(error, OSError):
            fail(f"cannot create {description} without replacement: {error}")
        raise
    try:
        if (
            not stat.S_ISREG(metadata.st_mode)
            or stat.S_IMODE(metadata.st_mode) != 0o600
            or metadata.st_uid != os.geteuid()
            or metadata.st_nlink != 1
            or metadata.st_size != len(raw)
            or stable_binding(named) != stable_binding(metadata)
        ):
            fail(f"new {description} identity drifted")
    except BaseException:
        failure = rollback_exact(directory_fd, name, descriptor, description)
        os.close(descriptor)
        if failure is not None:
            fail(f"{description} publication rollback failed: {failure}")
        raise
    return descriptor, metadata


def rollback_exact(
    directory_fd: int, name: str, descriptor: int, description: str
) -> str | None:
    try:
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        held = os.fstat(descriptor)
    except FileNotFoundError:
        return None
    except OSError as error:
        return f"cannot inspect {description}: {error}"
    if stable_binding(named) != stable_binding(held):
        return f"cannot remove replaced {description}"
    try:
        os.unlink(name, dir_fd=directory_fd)
        os.fsync(directory_fd)
    except OSError as error:
        return f"cannot remove {description}: {error}"
    return None


def verify_published(
    directory_fd: int,
    name: str,
    descriptor: int,
    authenticated: os.stat_result,
    expected: bytes,
    description: str,
) -> None:
    try:
        before = os.fstat(descriptor)
        os.lseek(descriptor, 0, os.SEEK_SET)
        raw = os.read(descriptor, len(expected) + 1)
        after = os.fstat(descriptor)
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except OSError as error:
        fail(f"cannot revalidate published {description}: {error}")
    if (
        raw != expected
        or file_identity(authenticated) != file_identity(before)
        or file_identity(before) != file_identity(after)
        or stable_binding(after) != stable_binding(named)
    ):
        fail(f"published {description} changed after directory sync")


def publish(
    plan_fd: int,
    artifact_fd: int,
    run_fd: int,
    artifact_id: str,
    roster: bytes,
    transcript: bytes,
    report: bytes,
    custody_check: Any,
) -> None:
    files = (
        (
            run_fd,
            f"{artifact_id}.independent-validator.roster.json",
            roster,
            "validator roster",
        ),
        (
            run_fd,
            f"{artifact_id}.independent-validator.transcript.json",
            transcript,
            "validator transcript",
        ),
        (
            artifact_fd,
            f"{artifact_id}.independent-validator.json",
            report,
            "validator report",
        ),
    )
    published: list[tuple[int, str, int, os.stat_result, bytes, str]] = []
    try:
        for directory_fd, name, raw, description in files:
            custody_check()
            descriptor, identity = create_new_at(directory_fd, name, raw, description)
            published.append(
                (directory_fd, name, descriptor, identity, raw, description)
            )
        os.fsync(run_fd)
        os.fsync(artifact_fd)
        os.fsync(plan_fd)
        custody_check()
        for directory_fd, name, descriptor, identity, raw, description in published:
            verify_published(
                directory_fd,
                name,
                descriptor,
                identity,
                raw,
                description,
            )
    except BaseException:
        failures = []
        for directory_fd, name, descriptor, _, _, description in reversed(published):
            failure = rollback_exact(directory_fd, name, descriptor, description)
            if failure is not None:
                failures.append(failure)
        if failures:
            fail(
                "independent-validator publication rollback failures: "
                + " | ".join(failures)
            )
        raise
    finally:
        for _, _, descriptor, _, _, _ in published:
            try:
                os.close(descriptor)
            except OSError:
                pass


def _intake(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    response_argument: str,
    binding_id: str,
    absolute_custodies: list[AbsoluteDirectoryCustody],
) -> None:
    ferric = canonical_root(ferric_argument, "Ferric repository", private=False)
    fe2o3 = canonical_root(fe2o3_argument, "fe2o3 repository", private=False)
    plan_root = canonical_root(plan_argument, "M1 plan", private=True)
    response_root = canonical_root(
        response_argument, "external independent-review response", private=True
    )
    roots = (ferric, fe2o3, plan_root, response_root)
    if any(
        left == right or left in right.parents or right in left.parents
        for index, left in enumerate(roots)
        for right in roots[index + 1 :]
    ):
        fail("independent-review roots must be mutually disjoint")
    requirements, plan, queue, plan_raw, queue_raw, sources = load_plan(
        ferric, fe2o3, plan_root, replay=True
    )
    slots = independent_slots(plan, queue)
    matches = [slot for slot in slots if slot["binding"]["id"] == binding_id]
    if len(matches) != 1:
        fail(f"unknown M1 independent-validator binding: {binding_id}")
    binding = matches[0]["binding"]
    tcb = load_tcb(plan_root, ferric, requirements, sources, report_validators(plan))
    request, request_raw, _ = request_for(requirements, plan, binding, sources, tcb)
    (
        response,
        results,
        response_held,
        response_descriptors,
        response_root_identity,
        response_directories,
    ) = authenticate_response(response_root, binding, request, request_raw, sources)
    plan_fd = artifact_fd = run_fd = ferric_fd = fe2o3_fd = -1
    plan_identity: os.stat_result | None = None
    ferric_identity: os.stat_result | None = None
    fe2o3_identity: os.stat_result | None = None
    plan_directories: list[ChildDirectoryCustody] = []
    plan_held: list[tuple[int, FileCustody, str]] = []
    try:
        validators = report_validators(plan)
        (
            plan_fd,
            plan_identity,
            artifact_fd,
            plan_directories,
            plan_held,
        ) = hold_plan_inputs(
            plan_root,
            plan_raw,
            queue_raw,
            ferric,
            requirements,
            sources,
            validators,
        )
        ferric_fd, ferric_identity = open_directory(
            ferric, "Ferric repository", private=False
        )
        fe2o3_fd, fe2o3_identity = open_directory(
            fe2o3, "fe2o3 repository", private=False
        )
        run_fd, _ = ensure_child_directory(
            plan_fd, "validator-runs", "M1 validator-run directory"
        )
        plan_directories.append(
            (
                plan_fd,
                "validator-runs",
                run_fd,
                os.fstat(run_fd),
                "M1 validator-run directory",
            )
        )
        roster, transcript, report = report_payloads(
            requirements, plan, binding, sources, tcb, request, response, results
        )

        def custody_check() -> None:
            if (
                plan_identity is None
                or ferric_identity is None
                or fe2o3_identity is None
            ):
                fail("independent-review custody was not initialized")
            reject_closure_outputs(plan_fd)
            for absolute_custody in absolute_custodies:
                revalidate_absolute_directory(absolute_custody)
            revalidate_directory(plan_root, plan_fd, plan_identity, "M1 plan")
            revalidate_directory(
                response_root,
                response_descriptors[0],
                response_root_identity,
                "external response root",
            )
            revalidate_directory(
                ferric, ferric_fd, ferric_identity, "Ferric repository"
            )
            revalidate_directory(fe2o3, fe2o3_fd, fe2o3_identity, "fe2o3 repository")
            revalidate_child_custodies(plan_directories)
            revalidate_child_custodies(response_directories)
            for directory_fd, custody, description in plan_held:
                revalidate_file_at(directory_fd, custody, description)
            for directory_fd, custody, description in response_held:
                revalidate_file_at(directory_fd, custody, description)
            if repository_identity(ferric, "Ferric") != (
                sources[1]["commit"],
                sources[1]["tree"],
            ) or repository_identity(fe2o3, "fe2o3") != (
                sources[0]["commit"],
                sources[0]["tree"],
            ):
                fail("subject source identity changed during independent-review intake")

        publish(
            plan_fd,
            artifact_fd,
            run_fd,
            binding["artifact_id"],
            roster,
            transcript,
            report,
            custody_check,
        )
    finally:
        for _, custody, _ in plan_held:
            custody[1].close()
        for _, custody, _ in response_held:
            custody[1].close()
        for descriptor in reversed(response_descriptors):
            try:
                os.close(descriptor)
            except OSError:
                pass
        plan_descriptors = {descriptor for _, _, descriptor, _, _ in plan_directories}
        plan_descriptors.update(
            descriptor
            for descriptor in (run_fd, artifact_fd, plan_fd, ferric_fd, fe2o3_fd)
            if descriptor >= 0
        )
        for descriptor in plan_descriptors:
            if descriptor >= 0:
                try:
                    os.close(descriptor)
                except OSError:
                    pass
    print(
        "PASS: ingested external independent-review response "
        f"binding={binding_id} artifact_sha256={digest_bytes(report)}"
    )


def intake(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    response_argument: str,
    binding_id: str,
) -> None:
    roots = (
        (
            canonical_root(ferric_argument, "Ferric repository", private=False),
            "Ferric repository",
            False,
        ),
        (
            canonical_root(fe2o3_argument, "fe2o3 repository", private=False),
            "fe2o3 repository",
            False,
        ),
        (canonical_root(plan_argument, "M1 plan", private=True), "M1 plan", True),
        (
            canonical_root(
                response_argument,
                "external independent-review response",
                private=True,
            ),
            "external independent-review response",
            True,
        ),
    )
    custodies: list[AbsoluteDirectoryCustody] = []
    try:
        for path, description, private in roots:
            custodies.append(
                open_absolute_directory(path, description, private=private)
            )
        for custody in custodies:
            revalidate_absolute_directory(custody)
        _intake(
            ferric_argument,
            fe2o3_argument,
            plan_argument,
            response_argument,
            binding_id,
            custodies,
        )
    finally:
        for custody in reversed(custodies):
            close_absolute_directory(custody)


def main() -> None:
    if len(sys.argv) == 7 and sys.argv[1] == "intake":
        intake(*sys.argv[2:])
        return
    if len(sys.argv) == 6 and sys.argv[1] == "export-all":
        export_all(*sys.argv[2:])
        return
    fail(
        f"usage: {sys.argv[0]} export-all FERRIC_REPO FE2O3_REPO PLAN_DIR HANDOFF_DIR\n"
        f"   or: {sys.argv[0]} intake FERRIC_REPO FE2O3_REPO PLAN_DIR "
        "INDEPENDENT_REVIEW_ROOT BINDING_ID"
    )


if __name__ == "__main__":
    main()
