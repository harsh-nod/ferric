#!/usr/bin/env python3
"""Validate one identity-bound M1 MI300X hardware transcript."""

from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, BinaryIO, NoReturn


PROTOCOL = "ferric.m1-validator.hardware-transcript.v1"
OBLIGATION_CLASSES = ("Assurance", "Roadmap")
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REPORT_FORMAT = "FERRIC-M1-HARDWARE-TRANSCRIPT-REPORT-V1"
TRANSCRIPT_FORMAT = "FERRIC-M1-MI300X-HARDWARE-RUN-V1"
ROSTER_FORMAT = "FERRIC-M1-HARDWARE-CASE-ROSTER-V1"
TEST_PROTOCOL = "ferric.m1.mi300x-hardware-test.v1"
ARTIFACT_TARGET = "gfx942:xnack-"
DEVICE_MARKETING_NAME = "AMD Instinct MI300X"
DEVICE_PROCESSOR = "gfx942"
DEVICE_VENDOR_ID = "1002"
AUTHORITY = "hardware-observation-only"
NONCLAIM = (
    "This report authenticates one bounded binding-local observation from the "
    "exact named MI300X hardware run. It does not establish path-specific "
    "semantics, reproducible binary provenance, independently attest "
    "operator-declared environment identities, prove machine refinement, or "
    "establish performance or M1 qualification."
)
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
PRINTABLE_ASCII = re.compile(r"[\x20-\x7e]{1,256}\Z")
UUID = re.compile(
    r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-"
    r"[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z"
)
PCI_BDF = re.compile(r"[0-9a-f]{4}:[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]\Z")
UTC_TIMESTAMP = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z\Z")
MAX_CONTEXT_BYTES = 1_000_000
MAX_REPORT_BYTES = 128_000
MAX_TRANSCRIPT_BYTES = 2_000_000
MAX_ROSTER_BYTES = 1_000_000
MAX_CASES = 1_024
MAX_COUNT = (1 << 63) - 1
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {"source.fe2o3": "fe2o3", "source.ferric": "ferric"}

CONTEXT_KEYS = {
    "artifact",
    "artifact_absolute_path",
    "binding",
    "format",
    "path_resolution",
    "requirements_sha256",
    "sources",
    "subject",
    "tcb",
}
ARTIFACT_KEYS = {"id", "kind", "path", "sha256", "size_bytes"}
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
PATH_KEYS = {"availability", "id", "path", "repository", "source_identity_id"}
SOURCE_KEYS = {
    "base_commit",
    "commit",
    "id",
    "repository",
    "source_closure_artifact_id",
    "source_closure_sha256",
    "tree",
}
SOURCE_CLOSURE_KEYS = {
    "commit",
    "id",
    "repository",
    "source_closure_sha256",
    "tree",
}
TCB_KEYS = {"artifact_id", "id", "identity_sha256", "kind"}
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
RESULT_KEYS = {
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
    "result",
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
ENVIRONMENT_KEYS = {"driver", "firmware", "rocm", "tool"}
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
PROCEDURE_KEYS = {
    "case_id_prefix",
    "format",
    "grid",
    "harness_binary",
    "harness_request",
    "harness_result",
    "kernel",
    "launch_count",
    "nonclaim",
    "output_tokens",
    "program",
    "protocol",
    "target",
    "workgroup",
}
TOOL_SOURCE_PATHS = {
    "cargo_lock": "Cargo.lock",
    "hardware_harness": "crates/ferric-engine/src/bin/ferric-m1-hardware-harness.rs",
    "package_manifest": "crates/ferric-engine/Cargo.toml",
    "packet_execution": "crates/ferric-engine/src/m1_packet_diagnostic_execution.rs",
    "persisted_kernel_artifacts": "crates/ferric-engine/src/persisted_kernel_artifacts.rs",
}
K7_PROGRAM = "k7-speculative-token-assembly-s1k4"
K7_GRID = [64, 1, 1]
K7_WORKGROUP = [64, 1, 1]
K7_OUTPUT_TOKENS = [10, 11, 12, 13, 14]


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
    payload = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    return digest_bytes(payload)


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


def require_text(value: Any, description: str) -> str:
    if not isinstance(value, str) or PRINTABLE_ASCII.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_count(value: Any, description: str, *, positive: bool = False) -> int:
    minimum = 1 if positive else 0
    if (
        not isinstance(value, int)
        or isinstance(value, bool)
        or value < minimum
        or value > MAX_COUNT
    ):
        fail(f"invalid {description}")
    return value


def require_exact_counts(value: Any, expected: list[int], description: str) -> None:
    if (
        not isinstance(value, list)
        or len(value) != len(expected)
        or any(not isinstance(item, int) or isinstance(item, bool) for item in value)
        or value != expected
    ):
        fail(f"invalid {description}")


def reject_duplicate_key(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def safe_relative(value: Any, description: str) -> PurePosixPath:
    if not isinstance(value, str) or not value or len(value) > 4096:
        fail(f"invalid {description}")
    path = PurePosixPath(value)
    if (
        path.is_absolute()
        or path.as_posix() != value
        or any(
            part in {"", ".", ".."} or SAFE_SEGMENT.fullmatch(part) is None
            for part in path.parts
        )
    ):
        fail(f"unsafe {description}")
    return path


def load_context() -> tuple[dict[str, Any], bytes]:
    raw = sys.stdin.buffer.read(MAX_CONTEXT_BYTES + 1)
    if not raw or len(raw) > MAX_CONTEXT_BYTES:
        fail("hardware-transcript context is empty or oversized")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("hardware-transcript context must have one trailing newline")
    payload = raw[:-1]
    try:
        source = payload.decode("ascii")
        value = json.loads(source, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"hardware-transcript context is not canonical ASCII JSON: {error}")
    canonical = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    if source != canonical:
        fail("hardware-transcript context is not canonical JSON")
    return exact_keys(value, CONTEXT_KEYS, "hardware-transcript context"), payload


def file_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_nlink,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def directory_identity(
    metadata: os.stat_result,
) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_nlink,
        metadata.st_uid,
        metadata.st_gid,
    )


class DirectoryRoot:
    def __init__(
        self,
        descriptor: int,
        identity: tuple[int, int, int, int, int, int],
        description: str,
    ) -> None:
        self.descriptor = descriptor
        self.identity = identity
        self.description = description


class DirectoryRelation:
    def __init__(
        self,
        parent_descriptor: int,
        name: str,
        descriptor: int,
        identity: tuple[int, int, int, int, int, int],
        description: str,
    ) -> None:
        self.parent_descriptor = parent_descriptor
        self.name = name
        self.descriptor = descriptor
        self.identity = identity
        self.description = description


class HeldRegularFile:
    def __init__(
        self,
        parent_descriptor: int,
        name: str,
        source: BinaryIO,
        identity: tuple[int, int, int, int, int, int, int],
        raw: bytes,
        description: str,
    ) -> None:
        self.parent_descriptor = parent_descriptor
        self.name = name
        self.source = source
        self.identity = identity
        self.raw = raw
        self.description = description


class InputCustody:
    def __init__(self) -> None:
        self.roots: list[DirectoryRoot] = []
        self.directories: list[DirectoryRelation] = []
        self.files: list[HeldRegularFile] = []

    @staticmethod
    def directory_flags() -> int:
        return os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC

    @staticmethod
    def regular_flags() -> int:
        return os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC

    def open_absolute_directory(self, path: Path, description: str) -> int:
        if (
            not path.is_absolute()
            or path.as_posix() != str(path)
            or any(part in {"", ".", ".."} for part in path.parts[1:])
        ):
            fail(f"{description} path must be canonical and absolute")
        try:
            descriptor = os.open("/", self.directory_flags())
            metadata = os.fstat(descriptor)
        except OSError as error:
            fail(f"cannot open filesystem root for {description}: {error}")
        if not stat.S_ISDIR(metadata.st_mode):
            os.close(descriptor)
            fail(f"filesystem root for {description} is not a directory")
        self.roots.append(
            DirectoryRoot(descriptor, directory_identity(metadata), description)
        )
        for part in path.parts[1:]:
            descriptor = self.open_directory_at(descriptor, part, description)
        return descriptor

    def open_directory_at(
        self, parent_descriptor: int, name: str, description: str
    ) -> int:
        if name in {"", ".", ".."} or "/" in name:
            fail(f"invalid directory component for {description}")
        try:
            before = os.stat(name, dir_fd=parent_descriptor, follow_symlinks=False)
            descriptor = os.open(
                name, self.directory_flags(), dir_fd=parent_descriptor
            )
            opened = os.fstat(descriptor)
        except OSError as error:
            fail(f"{description} directory component {name!r} is unavailable: {error}")
        if (
            stat.S_ISLNK(before.st_mode)
            or not stat.S_ISDIR(before.st_mode)
            or not stat.S_ISDIR(opened.st_mode)
            or directory_identity(before) != directory_identity(opened)
        ):
            os.close(descriptor)
            fail(f"{description} directory component {name!r} was substituted")
        self.directories.append(
            DirectoryRelation(
                parent_descriptor,
                name,
                descriptor,
                directory_identity(opened),
                description,
            )
        )
        return descriptor

    def hold_relative_regular(
        self,
        root_descriptor: int,
        relative: PurePosixPath,
        limit: int,
        description: str,
    ) -> HeldRegularFile:
        parent_descriptor = root_descriptor
        for part in relative.parts[:-1]:
            parent_descriptor = self.open_directory_at(
                parent_descriptor, part, description
            )
        name = relative.parts[-1]
        try:
            before = os.stat(name, dir_fd=parent_descriptor, follow_symlinks=False)
            descriptor = os.open(
                name, self.regular_flags(), dir_fd=parent_descriptor
            )
            source = os.fdopen(descriptor, "rb")
            opened = os.fstat(source.fileno())
        except OSError as error:
            fail(f"{description} is unavailable: {error}")
        try:
            if (
                stat.S_ISLNK(before.st_mode)
                or not stat.S_ISREG(before.st_mode)
                or not stat.S_ISREG(opened.st_mode)
                or before.st_nlink != 1
                or opened.st_nlink != 1
                or before.st_size <= 0
                or before.st_size > limit
                or file_identity(before) != file_identity(opened)
            ):
                fail(
                    f"{description} must be a bounded stable regular single-link file"
                )
            raw = source.read(limit + 1)
            after = os.fstat(source.fileno())
            named = os.stat(name, dir_fd=parent_descriptor, follow_symlinks=False)
            if (
                len(raw) != before.st_size
                or len(raw) > limit
                or file_identity(before) != file_identity(after)
                or file_identity(after) != file_identity(named)
            ):
                fail(f"{description} changed while it was read")
        except BaseException:
            source.close()
            raise
        held = HeldRegularFile(
            parent_descriptor,
            name,
            source,
            file_identity(after),
            raw,
            description,
        )
        self.files.append(held)
        return held

    def revalidate(self) -> None:
        for root in self.roots:
            try:
                current = os.fstat(root.descriptor)
            except OSError as error:
                fail(f"cannot revalidate {root.description} root: {error}")
            if (
                not stat.S_ISDIR(current.st_mode)
                or directory_identity(current) != root.identity
            ):
                fail(f"{root.description} root changed during validation")
        for relation in self.directories:
            try:
                named = os.stat(
                    relation.name,
                    dir_fd=relation.parent_descriptor,
                    follow_symlinks=False,
                )
                held = os.fstat(relation.descriptor)
            except OSError as error:
                fail(f"cannot revalidate {relation.description} directory: {error}")
            if (
                stat.S_ISLNK(named.st_mode)
                or not stat.S_ISDIR(named.st_mode)
                or directory_identity(named) != relation.identity
                or directory_identity(held) != relation.identity
            ):
                fail(f"{relation.description} directory relation changed")
        for held_file in self.files:
            try:
                held_file.source.seek(0)
                raw = held_file.source.read(len(held_file.raw) + 1)
                current = os.fstat(held_file.source.fileno())
                named = os.stat(
                    held_file.name,
                    dir_fd=held_file.parent_descriptor,
                    follow_symlinks=False,
                )
            except OSError as error:
                fail(f"cannot revalidate {held_file.description}: {error}")
            if (
                stat.S_ISLNK(named.st_mode)
                or not stat.S_ISREG(named.st_mode)
                or raw != held_file.raw
                or file_identity(current) != held_file.identity
                or file_identity(named) != held_file.identity
            ):
                fail(f"{held_file.description} changed during validation")

    def close(self) -> None:
        for held_file in reversed(self.files):
            held_file.source.close()
        for relation in reversed(self.directories):
            os.close(relation.descriptor)
        for root in reversed(self.roots):
            os.close(root.descriptor)


def lexical_evidence_root(
    report_path: Path, report_relative: PurePosixPath
) -> Path:
    root = report_path
    for _ in report_relative.parts:
        root = root.parent
    if root.joinpath(*report_relative.parts) != report_path:
        fail("hardware-transcript report absolute and relative paths disagree")
    return root


def decode_canonical_json(raw: bytes, description: str) -> dict[str, Any]:
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail(f"{description} must have one trailing newline")
    try:
        value = json.loads(raw, object_pairs_hook=reject_duplicate_key)
        expected = (
            json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
        ).encode("ascii")
    except (UnicodeDecodeError, UnicodeEncodeError, json.JSONDecodeError) as error:
        fail(f"{description} is invalid canonical ASCII JSON: {error}")
    if raw != expected:
        fail(f"{description} is not canonical JSON")
    if not isinstance(value, dict):
        fail(f"{description} must be an object")
    return value


def validate_sources(value: Any, requirements: dict[str, Any]) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail("hardware-transcript source roster is incomplete")
    for record, expected_id in zip(value, SOURCE_IDS, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source context {expected_id}")
        if record["id"] != expected_id:
            fail("hardware-transcript source roster order or identity drifted")
        if record["repository"] != SOURCE_REPOSITORIES[expected_id]:
            fail(f"hardware-transcript source repository drifted: {expected_id}")
        require_git_id(record["base_commit"], f"{expected_id} base commit")
        require_git_id(record["commit"], f"{expected_id} commit")
        require_git_id(record["tree"], f"{expected_id} tree")
        require_id(record["source_closure_artifact_id"], f"{expected_id} closure")
        require_sha256(record["source_closure_sha256"], f"{expected_id} closure")
    if value[0]["base_commit"] != requirements["m1_upstream_base_commit"]:
        fail("hardware-transcript fe2o3 base identity drifted")
    if value[1]["base_commit"] != FERRIC_BASE_COMMIT:
        fail("hardware-transcript Ferric base identity drifted")
    return value


def source_closures(sources: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        {
            "commit": record["commit"],
            "id": record["id"],
            "repository": record["repository"],
            "source_closure_sha256": record["source_closure_sha256"],
            "tree": record["tree"],
        }
        for record in sources
    ]


def validate_tcb(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("hardware-transcript TCB roster is incomplete")
    for record, expected_id in zip(value, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, f"TCB context {expected_id}")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("hardware-transcript TCB order, identity, or kind drifted")
        require_id(record["artifact_id"], f"{expected_id} artifact")
        require_sha256(record["identity_sha256"], f"{expected_id} identity")
    return value


def requirements_spec(
    requirements: dict[str, Any], obligation_class: str, obligation_id: str
) -> tuple[dict[str, Any], str, list[str]]:
    roadmaps = requirements["roadmap_requirements"]
    properties = requirements["assurance_properties"]
    if (
        len(roadmaps) != 33
        or len(properties) != 17
        or any(record["obligation_state"] != "Open" for record in roadmaps)
        or any(record["obligation_state"] != "Open" for record in properties)
    ):
        fail("hardware-transcript requires every M1 obligation to remain Open")
    if obligation_class == "Roadmap":
        matches = [record for record in roadmaps if record["id"] == obligation_id]
        if len(matches) != 1:
            fail("hardware-transcript binding names an unknown roadmap obligation")
        record = matches[0]
        return record, record["title"], record["assurance_properties"]
    if obligation_class == "Assurance":
        matches = [record for record in properties if record["name"] == obligation_id]
        if len(matches) != 1:
            fail("hardware-transcript binding names an unknown assurance property")
        record = matches[0]
        return record, record["boundary"], [obligation_id]
    fail("hardware-transcript obligation class drifted")


def validate_source_closures(value: Any, expected: list[dict[str, Any]]) -> None:
    if value != expected:
        fail("hardware transcript Ferric or fe2o3 source closure drifted")
    for record in value:
        exact_keys(record, SOURCE_CLOSURE_KEYS, "hardware source closure")


def validate_device(value: Any) -> dict[str, Any]:
    device = exact_keys(value, DEVICE_KEYS, "hardware device identity")
    uuid = device["device_uuid"]
    if (
        not isinstance(uuid, str)
        or UUID.fullmatch(uuid) is None
        or len(set(uuid.replace("-", ""))) == 1
    ):
        fail("hardware device UUID is invalid or a placeholder")
    if (
        device["device_count"] != 1
        or isinstance(device["device_count"], bool)
        or device["marketing_name"] != DEVICE_MARKETING_NAME
        or device["processor"] != DEVICE_PROCESSOR
        or device["vendor_id"] != DEVICE_VENDOR_ID
        or device["xnack"] != "disabled"
        or not isinstance(device["pci_bdf"], str)
        or PCI_BDF.fullmatch(device["pci_bdf"]) is None
    ):
        fail("hardware device is not exactly one MI300X gfx942:xnack- device")
    return device


def validate_environment(
    value: Any,
    repo_descriptor: int,
    custody: InputCustody,
    reviewed_binary_sha256: str,
    reviewed_binary_size: int,
) -> dict[str, Any]:
    environment = exact_keys(value, ENVIRONMENT_KEYS, "hardware environment")
    rocm = exact_keys(environment["rocm"], ROCM_KEYS, "ROCm identity")
    driver = exact_keys(environment["driver"], DRIVER_KEYS, "driver identity")
    firmware = exact_keys(environment["firmware"], FIRMWARE_KEYS, "firmware identity")
    tool = exact_keys(environment["tool"], TOOL_KEYS, "hardware tool identity")
    require_text(rocm["version"], "ROCm version")
    require_sha256(rocm["installation_sha256"], "ROCm installation SHA-256")
    if driver["name"] != "amdgpu":
        fail("hardware transcript driver must be amdgpu")
    require_text(driver["version"], "driver version")
    require_sha256(driver["module_sha256"], "driver module SHA-256")
    require_text(firmware["package_version"], "firmware package version")
    require_sha256(firmware["bundle_sha256"], "firmware bundle SHA-256")
    if (
        tool["name"] != "ferric-m1-hardware-harness"
        or tool["protocol"] != TEST_PROTOCOL
    ):
        fail("hardware transcript tool identity or protocol drifted")
    require_text(tool["version"], "hardware tool version")
    if (
        require_sha256(tool["binary_sha256"], "hardware tool binary SHA-256")
        != reviewed_binary_sha256
        or require_count(
            tool["binary_size_bytes"], "hardware tool binary size", positive=True
        )
        != reviewed_binary_size
    ):
        fail("hardware tool binary does not match the reviewed procedure pin")
    source_sha256s = exact_keys(
        tool["source_sha256s"],
        set(TOOL_SOURCE_PATHS),
        "hardware tool source identities",
    )
    for key, relative in TOOL_SOURCE_PATHS.items():
        observed = require_sha256(source_sha256s[key], f"hardware tool source {key}")
        held_source = custody.hold_relative_regular(
            repo_descriptor,
            safe_relative(relative, f"hardware tool source path {key}"),
            MAX_TRANSCRIPT_BYTES,
            f"hardware tool source {key}",
        )
        expected = digest_bytes(held_source.raw)
        if observed != expected:
            fail(f"hardware tool source identity drifted: {key}")
    return environment


def parse_utc(value: Any, description: str) -> datetime:
    if not isinstance(value, str) or UTC_TIMESTAMP.fullmatch(value) is None:
        fail(f"invalid {description}")
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(
            tzinfo=timezone.utc
        )
    except ValueError as error:
        fail(f"invalid {description}: {error}")


def validate_cases(
    value: Any,
    binding: dict[str, Any],
    assurance_property_ids: list[str],
    procedure_sha256: str,
) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != 1:
        fail("hardware case roster must contain exactly one K7 case")
    case_ids: list[str] = []
    for case in value:
        exact_keys(case, CASE_KEYS, "hardware case")
        case_id = require_id(case["id"], "hardware case id")
        require_sha256(case["procedure_sha256"], f"hardware case {case_id} procedure")
        if (
            case["assurance_property_ids"] != assurance_property_ids
            or case["obligation_class"] != binding["obligation_class"]
            or case["obligation_id"] != binding["obligation_id"]
            or case["path_id"] != binding["path_id"]
            or case["profile_id"] != binding["profile_id"]
            or case["procedure_sha256"] != procedure_sha256
            or case["requires_gpu_work"] is not True
        ):
            fail(f"hardware case binding or GPU-work requirement drifted: {case_id}")
        case_ids.append(case_id)
    if case_ids != sorted(case_ids) or len(case_ids) != len(set(case_ids)):
        fail("hardware cases are duplicated or not canonically ordered")
    return value


def validate_roster(
    roster: dict[str, Any],
    binding: dict[str, Any],
    requirements_sha256: str,
    expected_sources: list[dict[str, Any]],
    expected_tcb: dict[str, str],
    assurance_property_ids: list[str],
    procedure_sha256: str,
) -> list[dict[str, Any]]:
    exact_keys(roster, ROSTER_KEYS, "hardware case roster")
    if (
        roster["format"] != ROSTER_FORMAT
        or roster["protocol"] != TEST_PROTOCOL
        or roster["target"] != ARTIFACT_TARGET
        or roster["binding_sha256"] != binding["binding_sha256"]
        or roster["obligation_class"] != binding["obligation_class"]
        or roster["obligation_id"] != binding["obligation_id"]
        or roster["path_id"] != binding["path_id"]
        or roster["profile_id"] != binding["profile_id"]
        or roster["source_identity_id"] != binding["source_identity_id"]
        or roster["requirements_sha256"] != requirements_sha256
        or roster["tcb_identity_sha256s"] != expected_tcb
    ):
        fail("hardware case roster identity or target drifted")
    validate_source_closures(roster["source_closures"], expected_sources)
    uuid = roster["device_uuid"]
    if (
        not isinstance(uuid, str)
        or UUID.fullmatch(uuid) is None
        or len(set(uuid.replace("-", ""))) == 1
    ):
        fail("hardware case roster device UUID is invalid")
    cases = validate_cases(
        roster["cases"], binding, assurance_property_ids, procedure_sha256
    )
    expected_case_id = f"case.k7.{binding['id'].replace('.', '-')}"
    if cases[0]["id"] != expected_case_id:
        fail("hardware K7 case ID does not name its exact binding")
    return cases


def validate_transcript(
    transcript: dict[str, Any],
    roster_bytes: bytes,
    cases: list[dict[str, Any]],
    binding: dict[str, Any],
    requirements_sha256: str,
    expected_sources: list[dict[str, Any]],
    expected_tcb: dict[str, str],
    repo_descriptor: int,
    custody: InputCustody,
    reviewed_binary_sha256: str,
    reviewed_binary_size: int,
) -> tuple[dict[str, Any], dict[str, Any], int, int]:
    exact_keys(transcript, TRANSCRIPT_KEYS, "hardware run transcript")
    if (
        transcript["format"] != TRANSCRIPT_FORMAT
        or transcript["protocol"] != TEST_PROTOCOL
        or transcript["target"] != ARTIFACT_TARGET
        or transcript["binding_sha256"] != binding["binding_sha256"]
        or transcript["requirements_sha256"] != requirements_sha256
        or transcript["case_roster_sha256"] != digest_bytes(roster_bytes)
        or transcript["case_roster_size_bytes"] != len(roster_bytes)
        or isinstance(transcript["case_roster_size_bytes"], bool)
        or transcript["tcb_identity_sha256s"] != expected_tcb
        or transcript["gpu_work_submitted"] is not True
        or transcript["gpu_work_completed"] is not True
        or transcript["no_gpu_work"] is not False
        or transcript["result"] != "pass"
    ):
        fail("hardware run identity, result, or GPU-work assertion drifted")
    validate_source_closures(transcript["source_closures"], expected_sources)
    require_id(transcript["run_id"], "hardware run id")
    started = parse_utc(transcript["started_at_utc"], "hardware start timestamp")
    finished = parse_utc(transcript["finished_at_utc"], "hardware finish timestamp")
    if finished <= started:
        fail("hardware run finish must follow its start")
    device = validate_device(transcript["device"])
    environment = validate_environment(
        transcript["environment"],
        repo_descriptor,
        custody,
        reviewed_binary_sha256,
        reviewed_binary_size,
    )
    kernel_manifest_sha256 = require_sha256(
        transcript["kernel_manifest_sha256"], "kernel manifest identity"
    )
    kernel_catalog_sha256 = require_sha256(
        transcript["kernel_catalog_sha256"], "kernel catalog identity"
    )
    results = transcript["case_results"]
    if not isinstance(results, list) or len(results) != 1 or len(cases) != 1:
        fail("hardware run must contain one binding-local K7 result")
    total_launches = 0
    total_completions = 0
    for result, case in zip(results, cases, strict=True):
        exact_keys(result, RESULT_KEYS, "hardware case result")
        case_id = require_id(result["case_id"], "hardware case result id")
        observation_sha256 = require_sha256(
            result["gpu_observation_sha256"],
            f"hardware case {case_id} observation SHA-256",
        )
        launches = require_count(
            result["launch_count"], f"hardware case {case_id} launches", positive=True
        )
        completions = require_count(
            result["completion_count"],
            f"hardware case {case_id} completions",
            positive=True,
        )
        generation = require_count(
            result["generation"], f"hardware case {case_id} generation", positive=True
        )
        require_exact_counts(result["grid"], K7_GRID, "K7 grid")
        require_exact_counts(result["workgroup"], K7_WORKGROUP, "K7 workgroup")
        require_exact_counts(
            result["output_tokens"], K7_OUTPUT_TOKENS, "K7 output tokens"
        )
        if (
            result["binding_sha256"] != binding["binding_sha256"]
            or case_id != case["id"]
            or result["procedure_sha256"] != case["procedure_sha256"]
            or result["program"] != K7_PROGRAM
            or result["output_verified"] is not True
            or result["queue_released"] is not True
            or result["result"] != "pass"
            or launches != 1
            or completions != 1
        ):
            fail(f"hardware singleton K7 result drifted: {case_id}")
        observation = (
            "ferric-m1-k7-observation-v1|"
            f"{binding['binding_sha256']}|{case_id}|{case['procedure_sha256']}|"
            f"{kernel_manifest_sha256}|{kernel_catalog_sha256}|"
            f"{device['device_uuid']}|{device['pci_bdf']}|{generation}|"
            "10,11,12,13,14\n"
        ).encode("ascii")
        if observation_sha256 != digest_bytes(observation):
            fail(f"hardware K7 observation digest drifted: {case_id}")
        total_launches += launches
        total_completions += completions
    if total_launches != 1 or total_completions != 1:
        fail("hardware run is not exactly one completed GPU launch")
    return device, environment, total_launches, total_completions


def validate(
    context: dict[str, Any], context_payload: bytes | None = None
) -> None:
    custody = InputCustody()
    try:
        validate_with_custody(context, custody)
        custody.revalidate()
        if context_payload is not None:
            print(
                f"PASS: {PROTOCOL} artifact_sha256="
                f"{context['artifact']['sha256']} "
                f"context_sha256={digest_bytes(context_payload)}"
            )
    finally:
        custody.close()


def validate_with_custody(
    context: dict[str, Any], custody: InputCustody
) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("hardware-transcript context index format drifted")
    validator_path = Path(__file__)
    if not validator_path.is_absolute() or validator_path.as_posix() != str(
        validator_path
    ):
        fail("hardware validator source path must be canonical and absolute")
    repo = validator_path.parents[3]
    repo_descriptor = custody.open_absolute_directory(
        repo, "Ferric source repository"
    )
    requirements_file = custody.hold_relative_regular(
        repo_descriptor,
        safe_relative("proofs/M1_REQUIREMENTS.json", "M1 requirements path"),
        MAX_REPORT_BYTES,
        "M1 requirements manifest",
    )
    requirements_raw = requirements_file.raw
    requirements = decode_canonical_json(
        requirements_raw, "M1 requirements manifest"
    )
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("hardware-transcript context requirements identity drifted")
    procedure_file = custody.hold_relative_regular(
        repo_descriptor,
        safe_relative(
            "proofs/m1-qualification/hardware-k7-procedure.json",
            "checked-in K7 hardware procedure path",
        ),
        MAX_REPORT_BYTES,
        "checked-in K7 hardware procedure",
    )
    procedure_raw = procedure_file.raw
    procedure = decode_canonical_json(
        procedure_raw, "checked-in K7 hardware procedure"
    )
    exact_keys(procedure, PROCEDURE_KEYS, "checked-in K7 hardware procedure")
    if procedure["format"] != "FERRIC-M1-HARDWARE-PROCEDURE-V1":
        fail("checked-in K7 hardware procedure format drifted")
    reviewed_binary = exact_keys(
        procedure["harness_binary"],
        HARNESS_BINARY_KEYS,
        "reviewed hardware harness pin",
    )
    reviewed_binary_sha256 = require_sha256(
        reviewed_binary["sha256"], "reviewed hardware harness SHA-256"
    )
    reviewed_binary_size = require_count(
        reviewed_binary["size_bytes"], "reviewed hardware harness size", positive=True
    )
    procedure_sha256 = digest_bytes(procedure_raw)

    artifact = exact_keys(context["artifact"], ARTIFACT_KEYS, "artifact context")
    artifact_id = require_id(artifact["id"], "artifact id")
    if artifact["kind"] != "HardwareTranscript":
        fail("hardware-transcript artifact kind drifted")
    report_relative = safe_relative(artifact["path"], "report relative path")
    expected_report_path = f"artifacts/{artifact_id}.hardware-transcript.json"
    if report_relative.as_posix() != expected_report_path:
        fail("hardware-transcript report path is not canonical for its artifact id")
    report_sha256 = require_sha256(artifact["sha256"], "report SHA-256")
    if (
        not isinstance(artifact["size_bytes"], int)
        or isinstance(artifact["size_bytes"], bool)
        or artifact["size_bytes"] <= 0
    ):
        fail("hardware-transcript report size is invalid")
    artifact_absolute_path = context["artifact_absolute_path"]
    if not isinstance(artifact_absolute_path, str):
        fail("hardware-transcript report absolute path is invalid")
    report_path = Path(artifact_absolute_path)
    if (
        not report_path.is_absolute()
        or report_path.as_posix() != artifact_absolute_path
    ):
        fail("hardware-transcript report absolute path is not canonical")
    root = lexical_evidence_root(report_path, report_relative)
    evidence_descriptor = custody.open_absolute_directory(
        root, "hardware evidence root"
    )
    report_file = custody.hold_relative_regular(
        evidence_descriptor,
        report_relative,
        MAX_REPORT_BYTES,
        "hardware-transcript report",
    )
    report_bytes = report_file.raw
    report = decode_canonical_json(report_bytes, "hardware-transcript report")
    exact_keys(report, REPORT_KEYS, "hardware-transcript report")
    if (
        len(report_bytes) != artifact["size_bytes"]
        or digest_bytes(report_bytes) != report_sha256
    ):
        fail("hardware-transcript report bytes do not match their context identity")

    binding = exact_keys(context["binding"], BINDING_KEYS, "binding context")
    for key in ("artifact_id", "id"):
        require_id(binding[key], f"binding {key}")
    for key in ("obligation_id", "path_id", "profile_id", "source_identity_id"):
        require_name(binding[key], f"binding {key}")
    require_sha256(binding["binding_sha256"], "binding SHA-256")
    require_sha256(binding["statement_sha256"], "binding statement SHA-256")
    if (
        context["subject"] != f"binding:{binding['id']}"
        or binding["artifact_id"] != artifact_id
        or binding["evidence_kind"] != "hardware-test"
        or binding["source_identity_id"] not in SOURCE_IDS
        or binding["tcb_ids"] != list(TCB_IDS)
    ):
        fail("hardware-transcript binding context drifted")
    binding_payload = {
        key: value for key, value in binding.items() if key != "binding_sha256"
    }
    if binding["binding_sha256"] != canonical_digest(binding_payload):
        fail("hardware-transcript binding identity mismatch")

    spec, statement, assurance_property_ids = requirements_spec(
        requirements, binding["obligation_class"], binding["obligation_id"]
    )
    profiles = {
        record["id"]: record["kinds"] for record in requirements["evidence_profiles"]
    }
    if (
        binding["profile_id"] not in spec["evidence_profiles"]
        or "hardware-test" not in profiles.get(binding["profile_id"], [])
        or binding["path_id"] not in spec["path_obligations"]
        or binding["statement_sha256"] != digest_bytes(statement.encode("utf-8"))
    ):
        fail("hardware-transcript obligation, profile, path, or statement drifted")

    resolution = exact_keys(
        context["path_resolution"], PATH_KEYS, "path-resolution context"
    )
    paths = {record["id"]: record for record in requirements["path_obligations"]}
    expected_path = paths.get(binding["path_id"])
    if (
        expected_path is None
        or expected_path["obligation_state"] != "Open"
        or resolution["id"] != binding["path_id"]
        or resolution["availability"] != expected_path["availability"]
        or resolution["path"] != expected_path["path"]
        or resolution["repository"] != expected_path["repository"]
        or resolution["source_identity_id"] != binding["source_identity_id"]
        or binding["source_identity_id"] != f"source.{expected_path['repository']}"
    ):
        fail("hardware-transcript path resolution drifted")

    sources = validate_sources(context["sources"], requirements)
    expected_sources = source_closures(sources)
    tcb = validate_tcb(context["tcb"])
    expected_tcb = {record["id"]: record["identity_sha256"] for record in tcb}

    roster_relative = safe_relative(
        report["case_roster_relative_path"], "hardware case-roster relative path"
    )
    transcript_relative = safe_relative(
        report["transcript_relative_path"], "hardware transcript relative path"
    )
    if roster_relative.as_posix() != f"hardware-rosters/{artifact_id}.json":
        fail("hardware case-roster path is not canonical for its artifact id")
    if transcript_relative.as_posix() != f"hardware-transcripts/{artifact_id}.json":
        fail("hardware run path is not canonical for its artifact id")
    roster_file = custody.hold_relative_regular(
        evidence_descriptor,
        roster_relative,
        MAX_ROSTER_BYTES,
        "hardware case roster",
    )
    transcript_file = custody.hold_relative_regular(
        evidence_descriptor,
        transcript_relative,
        MAX_TRANSCRIPT_BYTES,
        "hardware run transcript",
    )
    roster_bytes = roster_file.raw
    transcript_bytes = transcript_file.raw
    roster = decode_canonical_json(roster_bytes, "hardware case roster")
    transcript = decode_canonical_json(transcript_bytes, "hardware run transcript")
    if (
        report["case_roster_sha256"] != digest_bytes(roster_bytes)
        or report["case_roster_size_bytes"] != len(roster_bytes)
        or isinstance(report["case_roster_size_bytes"], bool)
        or report["transcript_sha256"] != digest_bytes(transcript_bytes)
        or report["transcript_size_bytes"] != len(transcript_bytes)
        or isinstance(report["transcript_size_bytes"], bool)
    ):
        fail("hardware companion bytes do not match the report identities")

    cases = validate_roster(
        roster,
        binding,
        requirements_sha256,
        expected_sources,
        expected_tcb,
        assurance_property_ids,
        procedure_sha256,
    )
    device, environment, launches, completions = validate_transcript(
        transcript,
        roster_bytes,
        cases,
        binding,
        requirements_sha256,
        expected_sources,
        expected_tcb,
        repo_descriptor,
        custody,
        reviewed_binary_sha256,
        reviewed_binary_size,
    )
    if roster["device_uuid"] != device["device_uuid"]:
        fail("hardware roster and transcript device UUIDs disagree")

    expected_source_digests = {
        record["id"]: record["source_closure_sha256"] for record in sources
    }
    if (
        report["format"] != REPORT_FORMAT
        or report["authority"] != AUTHORITY
        or report["nonclaim"] != NONCLAIM
        or report["evidence_kind"] != "hardware-test"
        or report["result"] != "observed-pass"
        or report["target"] != ARTIFACT_TARGET
        or report["test_protocol"] != TEST_PROTOCOL
        or report["gpu_work_observed"] is not True
        or report["binding_sha256"] != binding["binding_sha256"]
        or report["obligation_class"] != binding["obligation_class"]
        or report["obligation_id"] != binding["obligation_id"]
        or report["obligation_state"] != "Open"
        or report["assurance_property_ids"] != assurance_property_ids
        or report["profile_id"] != binding["profile_id"]
        or report["path_id"] != binding["path_id"]
        or report["path_resolution_sha256"] != canonical_digest(resolution)
        or report["requirements_sha256"] != requirements_sha256
        or report["source_identity_id"] != binding["source_identity_id"]
        or report["source_closure_sha256s"] != expected_source_digests
        or report["source_roster_sha256"] != canonical_digest(sources)
        or report["statement_sha256"] != binding["statement_sha256"]
        or report["tcb_identity_sha256s"] != expected_tcb
        or report["tcb_roster_sha256"] != canonical_digest(tcb)
        or report["case_count"] != len(cases)
        or isinstance(report["case_count"], bool)
        or report["passed_case_count"] != len(cases)
        or isinstance(report["passed_case_count"], bool)
        or report["total_gpu_launches"] != launches
        or isinstance(report["total_gpu_launches"], bool)
        or report["total_gpu_completions"] != completions
        or isinstance(report["total_gpu_completions"], bool)
        or report["device_identity_sha256"] != canonical_digest(device)
        or report["environment_identity_sha256"] != canonical_digest(environment)
        or report["kernel_manifest_sha256"] != transcript["kernel_manifest_sha256"]
        or report["kernel_catalog_sha256"] != transcript["kernel_catalog_sha256"]
    ):
        fail("hardware-transcript report content or identity drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("hardware-transcript validator protocol mismatch")
    context, context_payload = load_context()
    validate(context, context_payload)


if __name__ == "__main__":
    main()
