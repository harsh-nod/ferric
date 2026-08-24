#!/usr/bin/env python3
"""Produce one source-authenticated M1 hardware-transcript report."""

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
from datetime import datetime, timezone
from typing import Any, BinaryIO, Callable, NoReturn


PLAN_FORMAT = "FERRIC-M1-EVIDENCE-PLAN-V1"
WORK_FORMAT = "FERRIC-M1-EVIDENCE-WORK-QUEUE-V1"
PLAN_AUTHORITY = "planning-only-no-evidence"
PLAN_NONCLAIM = (
    "This bundle allocates external M1 evidence work only. It is not an evidence "
    "index, qualification receipt, validation result, or M1 closure claim."
)
REPORT_FORMAT = "FERRIC-M1-HARDWARE-TRANSCRIPT-REPORT-V1"
TRANSCRIPT_FORMAT = "FERRIC-M1-MI300X-HARDWARE-RUN-V1"
ROSTER_FORMAT = "FERRIC-M1-HARDWARE-CASE-ROSTER-V1"
PROCEDURE_FORMAT = "FERRIC-M1-HARDWARE-PROCEDURE-V1"
REQUEST_FORMAT = "FERRIC-M1-HARDWARE-HARNESS-REQUEST-V1"
RESULT_FORMAT = "FERRIC-M1-HARDWARE-HARNESS-RESULT-V1"
ENVIRONMENT_FORMAT = "FERRIC-M1-HARDWARE-ENVIRONMENT-V1"
TEST_PROTOCOL = "ferric.m1.mi300x-hardware-test.v1"
REPORT_TARGET = "gfx942:xnack-"
REPORT_AUTHORITY = "hardware-observation-only"
REPORT_NONCLAIM = (
    "This report authenticates one bounded binding-local observation from the "
    "exact named MI300X hardware run. It does not establish path-specific "
    "semantics, reproducible binary provenance, independently attest "
    "operator-declared environment identities, prove machine refinement, or "
    "establish performance or M1 qualification."
)
TCB_REPORT_FORMAT = "FERRIC-M1-TCB-REPORT-V1"
TCB_REPORT_AUTHORITY = "trusted-boundary-declaration-only"
TCB_REPORT_NONCLAIM = (
    "This report authenticates the declared M1 trusted boundary only. It does "
    "not establish component presence, version provenance, compiler or runtime "
    "correctness, hardware behavior, theorem truth, machine refinement, load, "
    "launch, performance, or qualification authority and closes no obligation."
)
HARDWARE_TEST_ROSTER_SHA256 = (
    "50ab14c739eb88d8ded5becc86ccf5420386e905ab2d583463da4dfbf82f17cb"
)
HARDWARE_TEST_TSV_SHA256 = (
    "b860743335a8be9deb576f82b17612c0a009b6caf7adad86b5f34d6500f1e480"
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
    "assurance_property_ids",
    "authority",
    "binding_sha256",
    "case_count",
    "case_roster_relative_path",
    "case_roster_sha256",
    "case_roster_size_bytes",
    "device_identity_sha256",
    "evidence_kind",
    "environment_identity_sha256",
    "format",
    "gpu_work_observed",
    "kernel_catalog_sha256",
    "kernel_manifest_sha256",
    "nonclaim",
    "obligation_class",
    "obligation_id",
    "obligation_state",
    "passed_case_count",
    "path_id",
    "path_resolution_sha256",
    "profile_id",
    "requirements_sha256",
    "result",
    "source_closure_sha256s",
    "source_identity_id",
    "source_roster_sha256",
    "statement_sha256",
    "target",
    "tcb_identity_sha256s",
    "tcb_roster_sha256",
    "test_protocol",
    "total_gpu_completions",
    "total_gpu_launches",
    "transcript_relative_path",
    "transcript_sha256",
    "transcript_size_bytes",
}
ROSTER_KEYS = {
    "binding_sha256",
    "cases",
    "device_uuid",
    "format",
    "obligation_class",
    "obligation_id",
    "path_id",
    "profile_id",
    "protocol",
    "requirements_sha256",
    "source_closures",
    "source_identity_id",
    "target",
    "tcb_identity_sha256s",
}
CASE_KEYS = {
    "assurance_property_ids",
    "id",
    "obligation_class",
    "obligation_id",
    "path_id",
    "procedure_sha256",
    "profile_id",
    "requires_gpu_work",
}
TRANSCRIPT_KEYS = {
    "binding_sha256",
    "case_results",
    "case_roster_sha256",
    "case_roster_size_bytes",
    "device",
    "environment",
    "finished_at_utc",
    "format",
    "gpu_work_completed",
    "gpu_work_submitted",
    "kernel_catalog_sha256",
    "kernel_manifest_sha256",
    "no_gpu_work",
    "protocol",
    "requirements_sha256",
    "result",
    "run_id",
    "source_closures",
    "started_at_utc",
    "target",
    "tcb_identity_sha256s",
}
HARNESS_REQUEST_KEYS = {"case", "format", "protocol", "target"}
HARNESS_REQUEST_CASE_KEYS = {"binding_sha256", "case_id", "procedure_sha256"}
HARNESS_RESULT_KEYS = {
    "case_result",
    "device",
    "environment",
    "finished_at_utc",
    "format",
    "gpu_work_completed",
    "gpu_work_submitted",
    "kernel_catalog_sha256",
    "kernel_manifest_sha256",
    "no_gpu_work",
    "protocol",
    "run_id",
    "started_at_utc",
    "status",
    "target",
    "tool_source_sha256s",
    "tool_version",
}
HARNESS_CASE_RESULT_KEYS = {
    "binding_sha256",
    "case_id",
    "completion_count",
    "generation",
    "gpu_observation_sha256",
    "grid",
    "launch_count",
    "output_tokens",
    "output_verified",
    "procedure_sha256",
    "program",
    "queue_released",
    "workgroup",
}
DEVICE_KEYS = {
    "device_count",
    "device_uuid",
    "marketing_name",
    "pci_bdf",
    "processor",
    "vendor_id",
    "xnack",
}
ENVIRONMENT_INPUT_KEYS = {
    "device",
    "driver",
    "firmware",
    "format",
    "gpu_unique_id",
    "rocm",
    "target",
}
HARNESS_ENVIRONMENT_KEYS = {"driver", "firmware", "rocm"}
ROCM_KEYS = {"installation_sha256", "version"}
DRIVER_KEYS = {"module_sha256", "name", "version"}
FIRMWARE_KEYS = {"bundle_sha256", "package_version"}
TOOL_KEYS = {
    "binary_sha256",
    "binary_size_bytes",
    "name",
    "protocol",
    "source_sha256s",
    "version",
}
HARNESS_BINARY_KEYS = {"sha256", "size_bytes"}
TOOL_SOURCE_PATHS = {
    "cargo_lock": "Cargo.lock",
    "hardware_harness": "crates/ferric-engine/src/bin/ferric-m1-hardware-harness.rs",
    "package_manifest": "crates/ferric-engine/Cargo.toml",
    "packet_execution": "crates/ferric-engine/src/m1_packet_diagnostic_execution.rs",
    "persisted_kernel_artifacts": "crates/ferric-engine/src/persisted_kernel_artifacts.rs",
}
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
UUID = re.compile(
    r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-"
    r"[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z"
)
PCI_BDF = re.compile(r"[0-9a-f]{4}:[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]\Z")
PRINTABLE_ASCII = re.compile(r"[\x20-\x7e]{1,256}\Z")
UTC_TIMESTAMP = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z\Z")
MAX_JSON_BYTES = 16_000_000
MAX_FILE_BYTES = 64_000_000
MAX_HARNESS_OUTPUT_BYTES = 2_000_000
MAX_KERNEL_FILES = 64
MAX_KERNEL_TOTAL_BYTES = 512_000_000


JsonObject = dict[str, Any]
HeldFile = tuple[str, BinaryIO, os.stat_result, bytes, str]
HeldDirectoryFiles = tuple[int, list[HeldFile]]
HeldDirectoryComponent = tuple[int, str, int, os.stat_result, str]
AbsoluteDirectoryCustody = tuple[int, list[HeldDirectoryComponent], str]
HeldComponentFile = tuple[int | None, int, list[HeldDirectoryComponent], HeldFile]
KernelDirectory = tuple[str, int, tuple[str, ...]]
KernelFile = tuple[str, int, HeldFile]
KernelCustody = tuple[
    AbsoluteDirectoryCustody,
    list[HeldDirectoryComponent],
    list[KernelDirectory],
    list[KernelFile],
]


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


def regular_open_flags(*, writable: bool = False) -> int:
    required = ("O_NOFOLLOW", "O_CLOEXEC")
    if any(not hasattr(os, name) for name in required):
        fail("descriptor-relative file custody requires O_NOFOLLOW/O_CLOEXEC")
    return (os.O_RDWR if writable else os.O_RDONLY) | os.O_NOFOLLOW | os.O_CLOEXEC


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


def path_components(path: Path, description: str) -> tuple[Path, tuple[str, ...]]:
    absolute = path.absolute()
    if not absolute.is_absolute() or not absolute.parts or absolute.parts[0] != "/":
        fail(f"{description} must resolve from the filesystem root")
    components = absolute.parts[1:]
    for component in components:
        single_component(component, description)
    return absolute, components


def walk_directory_components(
    root_fd: int, components: tuple[str, ...], description: str
) -> list[HeldDirectoryComponent]:
    chain: list[HeldDirectoryComponent] = []
    parent_fd = root_fd
    try:
        for ordinal, component in enumerate(components, 1):
            held = open_directory_component_at(
                parent_fd,
                component,
                f"{description} component {ordinal}",
            )
            chain.append(held)
            parent_fd = held[2]
    except BaseException:
        for held in reversed(chain):
            os.close(held[2])
        raise
    return chain


def authenticate_absolute_directory(
    path: Path, description: str, *, private: bool = False
) -> AbsoluteDirectoryCustody:
    _, components = path_components(path, description)
    try:
        root_fd = os.open("/", directory_open_flags())
        root_metadata = os.fstat(root_fd)
    except OSError as error:
        fail(f"filesystem root is unavailable for {description}: {error}")
    if not stat.S_ISDIR(root_metadata.st_mode):
        os.close(root_fd)
        fail(f"filesystem root is not a directory for {description}")
    chain: list[HeldDirectoryComponent] = []
    try:
        chain = walk_directory_components(root_fd, components, description)
        final_metadata = os.fstat(chain[-1][2] if chain else root_fd)
        if private:
            verify_private_directory(final_metadata, description)
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
    name = single_component(name, description)
    flags = regular_open_flags(writable=writable)
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
    name = single_component(name, description)
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
    name = single_component(name, description)
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
    single_component(name, description)
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
        or named.st_nlink != 1
        or raw != expected
        or file_identity(authenticated) != file_identity(before)
        or file_identity(before) != file_identity(after)
        or file_identity(after) != file_identity(named)
    ):
        fail(f"{description} changed after authentication")


def authenticate_file_at(
    directory_fd: int,
    name: str,
    limit: int,
    description: str,
    *,
    executable: bool = False,
) -> HeldFile:
    name = single_component(name, description)
    source, before = open_regular_at(directory_fd, name, description)
    try:
        if (
            before.st_size <= 0
            or before.st_size > limit
            or before.st_nlink != 1
            or (executable and stat.S_IMODE(before.st_mode) & 0o111 == 0)
        ):
            fail(f"{description} metadata is outside the admitted policy")
        raw = source.read(limit + 1)
        after = os.fstat(source.fileno())
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if (
            len(raw) != before.st_size
            or file_identity(before) != file_identity(after)
            or file_identity(after) != file_identity(named)
        ):
            fail(f"{description} changed while it was authenticated")
        return name, source, after, raw, description
    except BaseException:
        source.close()
        raise


def authenticate_relative_component_file(
    root_fd: int,
    relative: str,
    limit: int,
    description: str,
    *,
    executable: bool = False,
) -> HeldComponentFile:
    path = PurePosixPath(relative)
    if (
        path.is_absolute()
        or path.as_posix() != relative
        or len(path.parts) == 0
        or any(part in {"", ".", ".."} for part in path.parts)
    ):
        fail(f"{description} relative path is unsafe")
    parent_components = tuple(path.parts[:-1])
    chain = walk_directory_components(root_fd, parent_components, description)
    parent_fd = chain[-1][2] if chain else root_fd
    try:
        held = authenticate_file_at(
            parent_fd,
            path.parts[-1],
            limit,
            description,
            executable=executable,
        )
        return None, root_fd, chain, held
    except BaseException:
        for component in reversed(chain):
            os.close(component[2])
        raise


def authenticate_absolute_component_file(
    path: Path,
    limit: int,
    description: str,
    *,
    executable: bool = False,
) -> HeldComponentFile:
    _, components = path_components(path, description)
    if not components:
        fail(f"{description} must name a regular file")
    try:
        root_fd = os.open("/", directory_open_flags())
    except OSError as error:
        fail(f"filesystem root is unavailable for {description}: {error}")
    try:
        chain = walk_directory_components(root_fd, components[:-1], description)
    except BaseException:
        os.close(root_fd)
        raise
    parent_fd = chain[-1][2] if chain else root_fd
    try:
        held = authenticate_file_at(
            parent_fd,
            components[-1],
            limit,
            description,
            executable=executable,
        )
        return root_fd, root_fd, chain, held
    except BaseException:
        for component in reversed(chain):
            os.close(component[2])
        os.close(root_fd)
        raise


def component_file_data(custody: HeldComponentFile) -> bytes:
    return custody[3][3]


def component_file_descriptor(custody: HeldComponentFile) -> int:
    return custody[3][1].fileno()


def revalidate_component_file(custody: HeldComponentFile) -> None:
    _, root_fd, chain, held = custody
    for component in chain:
        revalidate_directory_component(component)
    parent_fd = chain[-1][2] if chain else root_fd
    revalidate_held_file(parent_fd, held)


def close_component_file(custody: HeldComponentFile) -> None:
    owned_root_fd, _, chain, held = custody
    held[1].close()
    for component in reversed(chain):
        os.close(component[2])
    if owned_root_fd is not None:
        os.close(owned_root_fd)


def enumerate_directory_names(directory_fd: int, description: str) -> tuple[str, ...]:
    try:
        with os.scandir(directory_fd) as entries:
            names = tuple(sorted(entry.name for entry in entries))
    except OSError as error:
        fail(f"cannot enumerate {description}: {error}")
    for name in names:
        single_component(name, description)
    return names


def authenticate_kernel_tree(path: Path) -> KernelCustody:
    root = authenticate_absolute_directory(path, "kernel artifact root")
    root_fd = directory_custody_fd(root)
    directory_components: list[HeldDirectoryComponent] = []
    directories: list[KernelDirectory] = []
    files: list[KernelFile] = []
    total = 0
    try:
        pending = [("", root_fd)]
        while pending:
            relative, directory_fd = pending.pop()
            names = enumerate_directory_names(directory_fd, "kernel artifact directory")
            directories.append((relative, directory_fd, names))
            for name in names:
                try:
                    metadata = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
                except OSError as error:
                    fail(f"cannot inspect kernel artifact: {error}")
                child_relative = f"{relative}/{name}" if relative else name
                if stat.S_ISLNK(metadata.st_mode):
                    fail("kernel artifact tree contains a symlink")
                if stat.S_ISDIR(metadata.st_mode):
                    component = open_directory_component_at(
                        directory_fd, name, "kernel artifact directory"
                    )
                    directory_components.append(component)
                    pending.append((child_relative, component[2]))
                elif stat.S_ISREG(metadata.st_mode):
                    held = authenticate_file_at(
                        directory_fd, name, MAX_FILE_BYTES, "kernel artifact file"
                    )
                    files.append((child_relative, directory_fd, held))
                    total += len(held[3])
                    if len(files) > MAX_KERNEL_FILES or total > MAX_KERNEL_TOTAL_BYTES:
                        fail("kernel artifact tree exceeds the admitted bound")
                else:
                    fail("kernel artifact tree contains a non-regular entry")
        if not files or not any(
            relative == "m1-kernel-artifacts.manifest.bin" for relative, _, _ in files
        ):
            fail("kernel artifact tree is missing its canonical manifest")
        return root, directory_components, directories, files
    except BaseException:
        for _, _, held in files:
            held[1].close()
        for component in reversed(directory_components):
            os.close(component[2])
        close_absolute_directory(root)
        raise


def kernel_root_fd(custody: KernelCustody) -> int:
    return directory_custody_fd(custody[0])


def revalidate_kernel_tree(custody: KernelCustody) -> None:
    root, directory_components, directories, files = custody
    revalidate_absolute_directory(root)
    for component in directory_components:
        revalidate_directory_component(component)
    for _, directory_fd, expected_names in directories:
        if (
            enumerate_directory_names(directory_fd, "kernel artifact directory")
            != expected_names
        ):
            fail("kernel artifact tree membership changed after authentication")
    for _, directory_fd, held in files:
        revalidate_held_file(directory_fd, held)


def close_kernel_tree(custody: KernelCustody) -> None:
    root, directory_components, _, files = custody
    for _, _, held in files:
        held[1].close()
    for component in reversed(directory_components):
        os.close(component[2])
    close_absolute_directory(root)


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
    name = single_component(name, description)

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


def parse_canonical_json_bytes(raw: bytes, description: str) -> JsonObject:
    def unique(pairs: list[tuple[str, Any]]) -> JsonObject:
        value: JsonObject = {}
        for key, item in pairs:
            if key in value:
                fail(f"{description} contains a duplicate JSON key: {key}")
            value[key] = item
        return value

    try:
        value = json.loads(raw, object_pairs_hook=unique)
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse {description}: {error}")
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"{description} is not a canonical JSON object")
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


def require_text(value: Any, description: str) -> str:
    if not isinstance(value, str) or PRINTABLE_ASCII.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_positive_count(value: Any, description: str) -> int:
    if (
        not isinstance(value, int)
        or isinstance(value, bool)
        or value <= 0
        or value > (1 << 63) - 1
    ):
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


def open_private_directory_at(parent_fd: int, name: str, description: str) -> int:
    held = open_directory_component_at(parent_fd, name, description)
    try:
        verify_private_directory(held[3], description)
        return held[2]
    except BaseException:
        os.close(held[2])
        raise


def revalidate_child_directory(
    parent_fd: int, name: str, descriptor: int, description: str
) -> None:
    name = single_component(name, description)
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
        fail(
            "hardware-transcript production requires every M1 obligation to remain Open"
        )
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
    name = single_component(name, "directory entry")
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
        prefix="ferric-m1-hardware-transcript-planner-replay-"
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
            "available_producer_items": 277,
            "missing_items": 358,
            "missing_producer_items": 81,
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
        fail(
            "hardware-transcript production refuses a plan containing a closure output"
        )
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


def select_hardware_transcript_binding(
    plan: JsonObject, queue: JsonObject, binding_id: str
) -> tuple[JsonObject, JsonObject]:
    if not isinstance(binding_id, str) or not binding_id.startswith("binding."):
        fail(f"unknown M1 hardware-transcript binding: {binding_id}")
    slots = [
        slot
        for slot in plan["binding_slots"]
        if slot.get("binding", {}).get("evidence_kind") == "hardware-test"
    ]
    if len(slots) != 58:
        fail("M1 hardware-transcript binding roster is incomplete")
    ids = [slot["binding"]["id"] for slot in slots]
    if ids != sorted(ids) or digest_bytes(("\n".join(ids) + "\n").encode("ascii")) != (
        HARDWARE_TEST_ROSTER_SHA256
    ):
        fail("M1 hardware-transcript binding ID roster drifted")

    queue_by_id = {item["id"]: item for item in queue["items"]}
    rows = []
    for slot in slots:
        binding = slot["binding"]
        artifact_id = binding["artifact_id"]
        artifact = {
            "id": artifact_id,
            "kind": "HardwareTranscript",
            "path": f"artifacts/{artifact_id}.hardware-transcript.json",
        }
        producer = {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-hardware-transcript.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                "HARDWARE_HARNESS",
                "KERNEL_ARTIFACTS",
                "HARDWARE_ENVIRONMENT",
                binding["id"],
            ],
            "role": "ferric-mi300x-hardware-harness",
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
            or binding["source_identity_id"] not in SOURCE_IDS
            or binding["tcb_ids"] != [identifier for identifier, _ in TCB]
            or slot["expected_artifact"] != artifact
            or slot["producer"] != producer
            or slot["state"] != "missing"
            or slot["foundation_selectors"] != []
            or queue_by_id.get(work_id) != work
        ):
            fail(f"M1 hardware-transcript producer contract drifted: {binding['id']}")
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
    if digest_bytes("".join(rows).encode("ascii")) != HARDWARE_TEST_TSV_SHA256:
        fail("M1 hardware-transcript allocation topology drifted")
    matches = [slot for slot in slots if slot["binding"]["id"] == binding_id]
    if len(matches) != 1:
        fail(f"unknown M1 hardware-transcript binding: {binding_id}")
    slot = matches[0]
    binding = slot["binding"]
    resolutions = [
        row for row in plan["path_resolutions"] if row["id"] == binding["path_id"]
    ]
    if len(resolutions) != 1:
        fail("selected M1 hardware-transcript path resolution is missing")
    resolution = resolutions[0]
    if (
        resolution["id"] != binding["path_id"]
        or resolution["source_identity_id"] != binding["source_identity_id"]
        or binding["source_identity_id"] != f"source.{resolution['repository']}"
        or resolution["availability"] not in {"ExistingFoundation", "RequiredFuture"}
    ):
        fail("selected M1 hardware-transcript path resolution drifted")
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
            fail(
                "selected hardware-transcript binding names an unknown roadmap obligation"
            )
        spec = matches[0]
        return spec, spec["title"], spec["assurance_properties"]
    if obligation_class == "Assurance":
        matches = [
            row
            for row in requirements["assurance_properties"]
            if row["name"] == obligation_id
        ]
        if len(matches) != 1:
            fail(
                "selected hardware-transcript binding names an unknown assurance property"
            )
        spec = matches[0]
        return spec, spec["boundary"], [obligation_id]
    fail("selected hardware-transcript obligation class drifted")


def source_closure_projection(sources: list[JsonObject]) -> list[JsonObject]:
    return [
        {
            "commit": row["commit"],
            "id": row["id"],
            "repository": row["repository"],
            "source_closure_sha256": row["source_closure_sha256"],
            "tree": row["tree"],
        }
        for row in sources
    ]


def authenticate_tool_sources(
    ferric_fd: int,
) -> tuple[dict[str, str], list[HeldComponentFile]]:
    identities: dict[str, str] = {}
    held: list[HeldComponentFile] = []
    try:
        for key, relative in sorted(TOOL_SOURCE_PATHS.items()):
            custody = authenticate_relative_component_file(
                ferric_fd,
                relative,
                MAX_FILE_BYTES,
                f"hardware harness source {key}",
            )
            held.append(custody)
            identities[key] = digest_bytes(component_file_data(custody))
    except BaseException:
        for custody in held:
            close_component_file(custody)
        raise
    return identities, held


def amd_smi_uuid(gpu_unique_id: int) -> str:
    top_byte = gpu_unique_id >> 56
    next_byte = (gpu_unique_id >> 48) & 0xFF
    low_48_bits = gpu_unique_id & 0x0000FFFFFFFFFFFF
    return f"{top_byte:02x}ff74a1-0000-1000-80{next_byte:02x}-{low_48_bits:012x}"


def validate_procedure(value: JsonObject) -> tuple[str, int]:
    expected = {
        "case_id_prefix": "case.k7.binding-",
        "format": PROCEDURE_FORMAT,
        "grid": [64, 1, 1],
        "harness_request": {
            "case_fields": sorted(HARNESS_REQUEST_CASE_KEYS),
            "fields": sorted(HARNESS_REQUEST_KEYS),
            "format": REQUEST_FORMAT,
        },
        "harness_result": {
            "case_result_fields": sorted(HARNESS_CASE_RESULT_KEYS),
            "fields": sorted(HARNESS_RESULT_KEYS),
            "format": RESULT_FORMAT,
        },
        "harness_binary": value.get("harness_binary"),
        "kernel": "K7",
        "launch_count": 1,
        "nonclaim": (
            "This procedure records one bounded K7 hardware observation only. "
            "It does not establish path-specific semantics, machine refinement, "
            "performance, or M1 qualification."
        ),
        "output_tokens": [10, 11, 12, 13, 14],
        "program": "k7-speculative-token-assembly-s1k4",
        "protocol": TEST_PROTOCOL,
        "target": REPORT_TARGET,
        "workgroup": [64, 1, 1],
    }
    if value != expected:
        fail("checked-in K7 hardware procedure contract drifted")
    binary = exact_keys(
        value["harness_binary"], HARNESS_BINARY_KEYS, "reviewed hardware harness pin"
    )
    sha256 = require_sha256(binary["sha256"], "reviewed hardware harness SHA-256")
    size_bytes = require_positive_count(
        binary["size_bytes"], "reviewed hardware harness size"
    )
    return sha256, size_bytes


def validate_device(value: Any) -> JsonObject:
    device = exact_keys(value, DEVICE_KEYS, "selected hardware device")
    uuid = device["device_uuid"]
    if (
        not isinstance(uuid, str)
        or UUID.fullmatch(uuid) is None
        or len(set(uuid.replace("-", ""))) == 1
        or device["device_count"] != 1
        or isinstance(device["device_count"], bool)
        or device["marketing_name"] != "AMD Instinct MI300X"
        or device["processor"] != "gfx942"
        or device["vendor_id"] != "1002"
        or device["xnack"] != "disabled"
        or not isinstance(device["pci_bdf"], str)
        or PCI_BDF.fullmatch(device["pci_bdf"]) is None
    ):
        fail("selected device is not exactly one measured MI300X gfx942:xnack- device")
    return device


def validate_measured_environment(value: JsonObject) -> JsonObject:
    exact_keys(value, ENVIRONMENT_INPUT_KEYS, "hardware environment input")
    if (
        value["format"] != ENVIRONMENT_FORMAT
        or value["target"] != REPORT_TARGET
        or not isinstance(value["gpu_unique_id"], int)
        or isinstance(value["gpu_unique_id"], bool)
        or value["gpu_unique_id"] <= 0
        or value["gpu_unique_id"] > (1 << 64) - 1
    ):
        fail("hardware environment target or KFD unique ID drifted")
    device = validate_device(value["device"])
    driver = exact_keys(value["driver"], DRIVER_KEYS, "amdgpu driver identity")
    firmware = exact_keys(value["firmware"], FIRMWARE_KEYS, "firmware package identity")
    rocm = exact_keys(value["rocm"], ROCM_KEYS, "ROCm installation identity")
    if driver["name"] != "amdgpu":
        fail("hardware environment driver must be amdgpu")
    require_sha256(driver["module_sha256"], "amdgpu module SHA-256")
    require_text(driver["version"], "amdgpu version")
    require_sha256(firmware["bundle_sha256"], "firmware bundle SHA-256")
    require_text(firmware["package_version"], "firmware package version")
    require_sha256(rocm["installation_sha256"], "ROCm installation SHA-256")
    require_text(rocm["version"], "ROCm version")
    if device["device_uuid"] != amd_smi_uuid(value["gpu_unique_id"]):
        fail("hardware environment UUID does not match its KFD GPU unique ID")
    return value


def parse_utc(value: Any, description: str) -> datetime:
    if not isinstance(value, str) or UTC_TIMESTAMP.fullmatch(value) is None:
        fail(f"invalid {description}")
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(
            tzinfo=timezone.utc
        )
    except ValueError as error:
        fail(f"invalid {description}: {error}")


def invoke_harness(
    harness: HeldComponentFile,
    kernel: KernelCustody,
    environment_file: HeldComponentFile,
    environment: JsonObject,
    request: JsonObject,
    binding_id: str,
    procedure: JsonObject,
    tool_source_sha256s: dict[str, str],
) -> JsonObject:
    request_bytes = canonical_bytes(
        exact_keys(request, HARNESS_REQUEST_KEYS, "hardware harness request")
    )
    harness_fd = component_file_descriptor(harness)
    kernel_fd = kernel_root_fd(kernel)
    environment_fd = component_file_descriptor(environment_file)
    environment_file[3][1].seek(0)
    try:
        completed = subprocess.run(
            [
                f"/proc/self/fd/{harness_fd}",
                f"/proc/self/fd/{kernel_fd}",
                f"/proc/self/fd/{environment_fd}",
            ],
            input=request_bytes,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=900,
            env={"PATH": os.environ.get("PATH", "")},
            pass_fds=(harness_fd, kernel_fd, environment_fd),
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"hardware harness could not complete: {error}")
    if completed.returncode != 0:
        diagnostic = completed.stderr[:4096].decode("utf-8", errors="replace")
        fail(
            f"hardware harness failed with status {completed.returncode}: {diagnostic}"
        )
    if not completed.stdout or len(completed.stdout) > MAX_HARNESS_OUTPUT_BYTES:
        fail("hardware harness result is empty or oversized")
    result = parse_canonical_json_bytes(completed.stdout, "hardware harness result")
    exact_keys(result, HARNESS_RESULT_KEYS, "hardware harness result")
    case = exact_keys(
        result["case_result"], HARNESS_CASE_RESULT_KEYS, "hardware harness case result"
    )
    request_case = exact_keys(
        request["case"], HARNESS_REQUEST_CASE_KEYS, "hardware harness request case"
    )
    device = validate_device(result["device"])
    returned_environment = exact_keys(
        result["environment"], HARNESS_ENVIRONMENT_KEYS, "returned environment"
    )
    expected_environment = {
        "driver": environment["driver"],
        "firmware": environment["firmware"],
        "rocm": environment["rocm"],
    }
    if device != environment["device"] or returned_environment != expected_environment:
        fail("hardware harness device or measured environment disagrees with its input")
    for key in ("kernel_manifest_sha256", "kernel_catalog_sha256"):
        require_sha256(result[key], f"hardware harness {key}")
    if result["tool_source_sha256s"] != tool_source_sha256s:
        fail("hardware harness source identities disagree with held Ferric sources")
    require_text(result["tool_version"], "hardware harness emitted version")
    generation = require_positive_count(case["generation"], "K7 dispatch generation")
    if (
        result["format"] != RESULT_FORMAT
        or result["protocol"] != TEST_PROTOCOL
        or result["target"] != REPORT_TARGET
        or result["status"] != "pass"
        or result["gpu_work_submitted"] is not True
        or result["gpu_work_completed"] is not True
        or result["no_gpu_work"] is not False
        or case["binding_sha256"] != request_case["binding_sha256"]
        or case["case_id"] != request_case["case_id"]
        or case["procedure_sha256"] != request_case["procedure_sha256"]
        or case["program"] != procedure["program"]
        or case["grid"] != procedure["grid"]
        or case["workgroup"] != procedure["workgroup"]
        or case["output_tokens"] != procedure["output_tokens"]
        or case["output_verified"] is not True
        or case["queue_released"] is not True
        or case["launch_count"] != 1
        or isinstance(case["launch_count"], bool)
        or case["completion_count"] != 1
        or isinstance(case["completion_count"], bool)
    ):
        fail("hardware harness singleton K7 result drifted")
    run_id = require_id(result["run_id"], "hardware run ID")
    binding_token = binding_id.replace(".", "-")
    if binding_token not in run_id:
        fail("hardware run ID is not distinct for its exact binding")
    started = parse_utc(result["started_at_utc"], "hardware start timestamp")
    finished = parse_utc(result["finished_at_utc"], "hardware finish timestamp")
    if finished <= started:
        fail("hardware run finish must follow its start")
    observation = (
        "ferric-m1-k7-observation-v1|"
        f"{case['binding_sha256']}|{case['case_id']}|{case['procedure_sha256']}|"
        f"{result['kernel_manifest_sha256']}|{result['kernel_catalog_sha256']}|"
        f"{device['device_uuid']}|{device['pci_bdf']}|{generation}|10,11,12,13,14\n"
    ).encode("ascii")
    if case["gpu_observation_sha256"] != digest_bytes(observation):
        fail("hardware K7 observation identity drifted")
    return result


def hardware_documents(
    requirements_sha256: str,
    requirements: JsonObject,
    sources: list[JsonObject],
    tcb: list[JsonObject],
    slot: JsonObject,
    resolution: JsonObject,
    procedure_sha256: str,
    environment: JsonObject,
    harness_sha256: str,
    harness_size_bytes: int,
    harness_result: JsonObject,
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
        fail("selected hardware-transcript obligation or path drifted")
    profiles = {row["id"]: row["kinds"] for row in requirements["evidence_profiles"]}
    if "hardware-test" not in profiles.get(binding["profile_id"], []):
        fail("selected profile does not admit hardware-test evidence")
    source_closures = source_closure_projection(sources)
    tcb_identities = {row["id"]: row["identity_sha256"] for row in tcb}
    case_id = f"case.k7.{binding['id'].replace('.', '-')}"
    roster = exact_keys(
        {
            "binding_sha256": binding["binding_sha256"],
            "cases": [
                exact_keys(
                    {
                        "assurance_property_ids": assurance_property_ids,
                        "id": case_id,
                        "obligation_class": binding["obligation_class"],
                        "obligation_id": binding["obligation_id"],
                        "path_id": binding["path_id"],
                        "procedure_sha256": procedure_sha256,
                        "profile_id": binding["profile_id"],
                        "requires_gpu_work": True,
                    },
                    CASE_KEYS,
                    "hardware case",
                )
            ],
            "device_uuid": environment["device"]["device_uuid"],
            "format": ROSTER_FORMAT,
            "obligation_class": binding["obligation_class"],
            "obligation_id": binding["obligation_id"],
            "path_id": binding["path_id"],
            "profile_id": binding["profile_id"],
            "protocol": TEST_PROTOCOL,
            "requirements_sha256": requirements_sha256,
            "source_closures": source_closures,
            "source_identity_id": binding["source_identity_id"],
            "target": REPORT_TARGET,
            "tcb_identity_sha256s": tcb_identities,
        },
        ROSTER_KEYS,
        "M1 hardware case roster",
    )
    roster_bytes = canonical_bytes(roster)
    case_result = harness_result["case_result"]
    tool = exact_keys(
        {
            "binary_sha256": harness_sha256,
            "binary_size_bytes": harness_size_bytes,
            "name": "ferric-m1-hardware-harness",
            "protocol": TEST_PROTOCOL,
            "source_sha256s": harness_result["tool_source_sha256s"],
            "version": harness_result["tool_version"],
        },
        TOOL_KEYS,
        "hardware tool identity",
    )
    transcript_environment = {
        **harness_result["environment"],
        "tool": tool,
    }
    transcript = exact_keys(
        {
            "binding_sha256": binding["binding_sha256"],
            "case_results": [{**case_result, "result": "pass"}],
            "case_roster_sha256": digest_bytes(roster_bytes),
            "case_roster_size_bytes": len(roster_bytes),
            "device": harness_result["device"],
            "environment": transcript_environment,
            "finished_at_utc": harness_result["finished_at_utc"],
            "format": TRANSCRIPT_FORMAT,
            "gpu_work_completed": True,
            "gpu_work_submitted": True,
            "kernel_catalog_sha256": harness_result["kernel_catalog_sha256"],
            "kernel_manifest_sha256": harness_result["kernel_manifest_sha256"],
            "no_gpu_work": False,
            "protocol": TEST_PROTOCOL,
            "requirements_sha256": requirements_sha256,
            "result": "pass",
            "run_id": harness_result["run_id"],
            "source_closures": source_closures,
            "started_at_utc": harness_result["started_at_utc"],
            "target": REPORT_TARGET,
            "tcb_identity_sha256s": tcb_identities,
        },
        TRANSCRIPT_KEYS,
        "M1 hardware run transcript",
    )
    transcript_bytes = canonical_bytes(transcript)
    artifact_id = binding["artifact_id"]
    report = exact_keys(
        {
            "assurance_property_ids": assurance_property_ids,
            "authority": REPORT_AUTHORITY,
            "binding_sha256": binding["binding_sha256"],
            "case_count": 1,
            "case_roster_relative_path": f"hardware-rosters/{artifact_id}.json",
            "case_roster_sha256": digest_bytes(roster_bytes),
            "case_roster_size_bytes": len(roster_bytes),
            "device_identity_sha256": canonical_digest(harness_result["device"]),
            "evidence_kind": "hardware-test",
            "environment_identity_sha256": canonical_digest(transcript_environment),
            "format": REPORT_FORMAT,
            "gpu_work_observed": True,
            "kernel_catalog_sha256": harness_result["kernel_catalog_sha256"],
            "kernel_manifest_sha256": harness_result["kernel_manifest_sha256"],
            "nonclaim": REPORT_NONCLAIM,
            "obligation_class": binding["obligation_class"],
            "obligation_id": binding["obligation_id"],
            "obligation_state": "Open",
            "passed_case_count": 1,
            "path_id": binding["path_id"],
            "path_resolution_sha256": canonical_digest(resolution),
            "profile_id": binding["profile_id"],
            "requirements_sha256": requirements_sha256,
            "result": "observed-pass",
            "source_closure_sha256s": {
                row["id"]: row["source_closure_sha256"] for row in sources
            },
            "source_identity_id": binding["source_identity_id"],
            "source_roster_sha256": canonical_digest(sources),
            "statement_sha256": binding["statement_sha256"],
            "target": REPORT_TARGET,
            "tcb_identity_sha256s": tcb_identities,
            "tcb_roster_sha256": canonical_digest(tcb),
            "test_protocol": TEST_PROTOCOL,
            "total_gpu_completions": 1,
            "total_gpu_launches": 1,
            "transcript_relative_path": f"hardware-transcripts/{artifact_id}.json",
            "transcript_sha256": digest_bytes(transcript_bytes),
            "transcript_size_bytes": len(transcript_bytes),
        },
        REPORT_KEYS,
        "M1 hardware-transcript report",
    )
    return roster, transcript, report


def ensure_artifact_directory(plan_fd: int) -> int:
    return open_private_directory_at(plan_fd, "artifacts", "M1 artifact directory")


def ensure_private_child_directory(
    plan_fd: int, name: str, description: str
) -> tuple[int, bool]:
    name = single_component(name, description)
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
        if created:
            try:
                os.rmdir(name, dir_fd=plan_fd)
            except OSError:
                pass
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
        name = single_component(name, description)
    except SystemExit:
        return f"cannot remove failed {description} publication with unsafe name"
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
    name = single_component(name, description)
    flags = regular_open_flags(writable=True) | os.O_CREAT | os.O_EXCL
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
    name = single_component(name, description)
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


def publish_hardware_transcript(
    plan_custody: AbsoluteDirectoryCustody,
    plan_fd: int,
    artifact_fd: int,
    roster_fd: int,
    transcript_fd: int,
    artifact_id: str,
    roster: bytes,
    transcript: bytes,
    report: bytes,
    custody_check: Callable[[], None],
) -> None:
    report_name = f"{artifact_id}.hardware-transcript.json"
    roster_name = f"{artifact_id}.json"
    transcript_name = f"{artifact_id}.json"
    published: list[tuple[int, str, int, bytes, str, os.stat_result]] = []
    try:
        revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
        )
        revalidate_child_directory(
            plan_fd, "hardware-rosters", roster_fd, "M1 hardware roster directory"
        )
        revalidate_child_directory(
            plan_fd,
            "hardware-transcripts",
            transcript_fd,
            "M1 hardware transcript directory",
        )
        revalidate_absolute_directory(plan_custody, private=True)
        if (
            entry_exists_at(artifact_fd, report_name)
            or entry_exists_at(roster_fd, roster_name)
            or entry_exists_at(transcript_fd, transcript_name)
        ):
            fail("hardware-transcript publication refuses a preexisting output")
        custody_check()
        roster_file_fd = create_new_file_at(
            roster_fd, roster_name, roster, "M1 hardware case roster"
        )
        published.append(
            (
                roster_fd,
                roster_name,
                roster_file_fd,
                roster,
                "M1 hardware case roster",
                os.fstat(roster_file_fd),
            )
        )
        custody_check()
        transcript_file_fd = create_new_file_at(
            transcript_fd,
            transcript_name,
            transcript,
            "M1 hardware run transcript",
        )
        published.append(
            (
                transcript_fd,
                transcript_name,
                transcript_file_fd,
                transcript,
                "M1 hardware run transcript",
                os.fstat(transcript_file_fd),
            )
        )
        custody_check()
        report_file_fd = create_new_file_at(
            artifact_fd, report_name, report, "M1 hardware-transcript report"
        )
        published.append(
            (
                artifact_fd,
                report_name,
                report_file_fd,
                report,
                "M1 hardware-transcript report",
                os.fstat(report_file_fd),
            )
        )
        os.fsync(roster_fd)
        os.fsync(transcript_fd)
        os.fsync(artifact_fd)
        os.fsync(plan_fd)
        revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
        )
        revalidate_child_directory(
            plan_fd, "hardware-rosters", roster_fd, "M1 hardware roster directory"
        )
        revalidate_child_directory(
            plan_fd,
            "hardware-transcripts",
            transcript_fd,
            "M1 hardware transcript directory",
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
                directory_fd,
                name,
                descriptor,
                expected,
                identity,
                description,
            )
    except OSError as error:
        rollback_failures = rollback_publications(published)
        if rollback_failures:
            fail(
                f"cannot durably publish M1 hardware transcript: {error}; "
                f"rollback failures: {' | '.join(rollback_failures)}"
            )
        fail(f"cannot durably publish M1 hardware transcript: {error}")
    except BaseException:
        rollback_failures = rollback_publications(published)
        if rollback_failures:
            fail(
                "M1 hardware transcript publication rollback failures: "
                + " | ".join(rollback_failures)
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
    harness_argument: str,
    kernel_argument: str,
    environment_argument: str,
    binding_id: str,
) -> None:
    ferric = Path(ferric_argument).absolute()
    fe2o3 = Path(fe2o3_argument).absolute()
    plan_root = Path(plan_argument).absolute()
    ferric_custody: AbsoluteDirectoryCustody | None = None
    fe2o3_custody: AbsoluteDirectoryCustody | None = None
    plan_custody: AbsoluteDirectoryCustody | None = None
    ferric_fd = -1
    plan_fd = -1
    artifact_fd = -1
    roster_fd = -1
    transcript_fd = -1
    roster_dir_created = False
    transcript_dir_created = False
    publication_complete = False
    tcb_files: list[tuple[str, str, BinaryIO, os.stat_result, bytes]] = []
    plan_files: list[HeldFile] = []
    closure_custody: HeldDirectoryFiles | None = None
    procedure_file: HeldComponentFile | None = None
    harness_file: HeldComponentFile | None = None
    environment_file: HeldComponentFile | None = None
    kernel_custody: KernelCustody | None = None
    tool_source_files: list[HeldComponentFile] = []
    report_bytes = b""
    roster_bytes = b""
    transcript_bytes = b""
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
        ferric_fd = directory_custody_fd(ferric_custody)
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
        slot, resolution = select_hardware_transcript_binding(plan, queue, binding_id)
        tcb, tcb_files = authenticate_tcb_reports(
            artifact_fd, ferric, requirements, sources, validators
        )
        procedure_file = authenticate_relative_component_file(
            ferric_fd,
            "proofs/m1-qualification/hardware-k7-procedure.json",
            MAX_JSON_BYTES,
            "checked-in K7 hardware procedure",
        )
        procedure = parse_canonical_json_bytes(
            component_file_data(procedure_file), "checked-in K7 hardware procedure"
        )
        reviewed_harness_sha256, reviewed_harness_size = validate_procedure(procedure)
        procedure_sha256 = digest_bytes(component_file_data(procedure_file))
        harness_path = Path(harness_argument)
        if harness_path.name != "ferric-m1-hardware-harness":
            fail("hardware harness must have the canonical executable name")
        harness_file = authenticate_absolute_component_file(
            harness_path,
            MAX_FILE_BYTES,
            "Ferric M1 hardware harness",
            executable=True,
        )
        harness_sha256 = digest_bytes(component_file_data(harness_file))
        harness_size_bytes = len(component_file_data(harness_file))
        if (
            harness_sha256 != reviewed_harness_sha256
            or harness_size_bytes != reviewed_harness_size
        ):
            fail("hardware harness does not match the reviewed procedure binary pin")
        tool_source_sha256s, tool_source_files = authenticate_tool_sources(ferric_fd)
        environment_file = authenticate_absolute_component_file(
            Path(environment_argument),
            MAX_JSON_BYTES,
            "measured hardware environment",
        )
        environment = validate_measured_environment(
            parse_canonical_json_bytes(
                component_file_data(environment_file), "measured hardware environment"
            )
        )
        kernel_custody = authenticate_kernel_tree(Path(kernel_argument))
        binding = slot["binding"]
        case_id = f"case.k7.{binding['id'].replace('.', '-')}"
        request = exact_keys(
            {
                "case": exact_keys(
                    {
                        "binding_sha256": binding["binding_sha256"],
                        "case_id": case_id,
                        "procedure_sha256": procedure_sha256,
                    },
                    HARNESS_REQUEST_CASE_KEYS,
                    "hardware harness request case",
                ),
                "format": REQUEST_FORMAT,
                "protocol": TEST_PROTOCOL,
                "target": REPORT_TARGET,
            },
            HARNESS_REQUEST_KEYS,
            "hardware harness request",
        )
        harness_result = invoke_harness(
            harness_file,
            kernel_custody,
            environment_file,
            environment,
            request,
            binding_id,
            procedure,
            tool_source_sha256s,
        )
        roster, transcript, report = hardware_documents(
            plan["requirements"]["sha256"],
            requirements,
            sources,
            tcb,
            slot,
            resolution,
            procedure_sha256,
            environment,
            harness_sha256,
            harness_size_bytes,
            harness_result,
        )
        roster_bytes = canonical_bytes(roster)
        transcript_bytes = canonical_bytes(transcript)
        report_bytes = canonical_bytes(report)

        repeated = validate_plan(ferric, fe2o3, plan_fd, replay=False)
        if repeated[5] != plan_raw or repeated[6] != queue_raw:
            fail("M1 plan or work queue changed during hardware-transcript production")
        repeated_slot, repeated_resolution = select_hardware_transcript_binding(
            repeated[1], repeated[2], binding_id
        )
        repeated_documents = hardware_documents(
            repeated[1]["requirements"]["sha256"],
            repeated[0],
            repeated[3],
            tcb,
            repeated_slot,
            repeated_resolution,
            procedure_sha256,
            environment,
            harness_sha256,
            harness_size_bytes,
            harness_result,
        )
        if tuple(canonical_bytes(item) for item in repeated_documents) != (
            roster_bytes,
            transcript_bytes,
            report_bytes,
        ):
            fail("M1 hardware-transcript inputs changed during production")
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
            revalidate_component_file(procedure_file)
            revalidate_component_file(harness_file)
            revalidate_component_file(environment_file)
            for custody in tool_source_files:
                revalidate_component_file(custody)
            revalidate_kernel_tree(kernel_custody)
            if any(
                entry_exists_at(plan_fd, name)
                for name in ("evidence-index.json", "receipt.json")
            ):
                fail("hardware-transcript producer created a forbidden closure output")
            revalidate_child_directory(
                plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
            )
            revalidate_absolute_directory(plan_custody, private=True)

        revalidate_completion_inputs()
        roster_fd, roster_dir_created = ensure_private_child_directory(
            plan_fd, "hardware-rosters", "M1 hardware roster directory"
        )
        transcript_fd, transcript_dir_created = ensure_private_child_directory(
            plan_fd,
            "hardware-transcripts",
            "M1 hardware transcript directory",
        )
        publish_hardware_transcript(
            plan_custody,
            plan_fd,
            artifact_fd,
            roster_fd,
            transcript_fd,
            slot["binding"]["artifact_id"],
            roster_bytes,
            transcript_bytes,
            report_bytes,
            revalidate_completion_inputs,
        )
        publication_complete = True
    finally:
        if kernel_custody is not None:
            close_kernel_tree(kernel_custody)
        for held in (environment_file, harness_file, procedure_file):
            if held is not None:
                close_component_file(held)
        for held in tool_source_files:
            close_component_file(held)
        if closure_custody is not None:
            close_source_closures(closure_custody)
        for _, source, _, _, _ in plan_files:
            source.close()
        for _, _, source, _, _ in tcb_files:
            source.close()
        if transcript_fd >= 0:
            os.close(transcript_fd)
        if roster_fd >= 0:
            os.close(roster_fd)
        if not publication_complete:
            for name, created in (
                ("hardware-transcripts", transcript_dir_created),
                ("hardware-rosters", roster_dir_created),
            ):
                if created:
                    try:
                        os.rmdir(name, dir_fd=plan_fd)
                    except OSError:
                        pass
        if artifact_fd >= 0:
            os.close(artifact_fd)
        if plan_custody is not None:
            close_absolute_directory(plan_custody)
        if fe2o3_custody is not None:
            close_absolute_directory(fe2o3_custody)
        if ferric_custody is not None:
            close_absolute_directory(ferric_custody)
    print(
        f"PASS: produced M1 MI300X hardware transcript binding={binding_id} "
        f"report_sha256={digest_bytes(report_bytes)}"
    )


def main() -> None:
    if len(sys.argv) != 8:
        fail(
            f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_REPO PLAN_DIR "
            "HARDWARE_HARNESS KERNEL_ARTIFACTS HARDWARE_ENVIRONMENT binding.NNNNN"
        )
    produce(*sys.argv[1:8])


if __name__ == "__main__":
    main()
