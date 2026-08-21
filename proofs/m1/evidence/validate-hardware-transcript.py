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
    "This report authenticates bounded observations from the exact named "
    "MI300X hardware run. Observations are not proofs, do not establish "
    "machine refinement, and do not establish performance or M1 qualification."
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
    "case_id",
    "completion_count",
    "gpu_observation_sha256",
    "launch_count",
    "result",
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
TOOL_KEYS = {"binary_sha256", "name", "protocol", "version"}


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


def open_regular(path: Path, description: str) -> tuple[BinaryIO, os.stat_result]:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
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
        or before.st_nlink != 1
        or opened.st_nlink != 1
        or (before.st_dev, before.st_ino) != (opened.st_dev, opened.st_ino)
    ):
        source.close()
        fail(f"{description} must be a stable regular non-symlink file")
    return source, opened


def read_bounded(path: Path, limit: int, description: str) -> bytes:
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


def reject_symlink_components(
    root: Path, relative: PurePosixPath, description: str
) -> Path:
    candidate = root.joinpath(*relative.parts)
    current = root
    try:
        if root.resolve(strict=True) != root:
            fail(f"{description} root must not contain a symlink")
        for part in relative.parts:
            current = current / part
            metadata = current.lstat()
            if stat.S_ISLNK(metadata.st_mode):
                fail(f"{description} path contains a symlink")
        if candidate.resolve(strict=True) != candidate:
            fail(f"{description} path escaped its evidence root")
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    return candidate


def evidence_root(report_path: Path, report_relative: PurePosixPath) -> Path:
    if not report_path.is_absolute() or report_path.as_posix() != str(report_path):
        fail("hardware-transcript report absolute path is not canonical")
    root = report_path
    for _ in report_relative.parts:
        root = root.parent
    if root.joinpath(*report_relative.parts) != report_path:
        fail("hardware-transcript report absolute and relative paths disagree")
    reject_symlink_components(root, report_relative, "hardware-transcript report")
    return root


def load_canonical_json(
    path: Path, limit: int, description: str
) -> tuple[dict[str, Any], bytes]:
    raw = read_bounded(path, limit, description)
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
    return value, raw


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


def validate_environment(value: Any) -> dict[str, Any]:
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
    require_sha256(tool["binary_sha256"], "hardware tool binary SHA-256")
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
) -> list[dict[str, Any]]:
    if not isinstance(value, list) or not value or len(value) > MAX_CASES:
        fail("hardware case roster is empty or oversized")
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
    return validate_cases(roster["cases"], binding, assurance_property_ids)


def validate_transcript(
    transcript: dict[str, Any],
    roster_bytes: bytes,
    cases: list[dict[str, Any]],
    binding: dict[str, Any],
    requirements_sha256: str,
    expected_sources: list[dict[str, Any]],
    expected_tcb: dict[str, str],
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
    environment = validate_environment(transcript["environment"])
    results = transcript["case_results"]
    if not isinstance(results, list) or len(results) != len(cases):
        fail("hardware case-result roster is incomplete")
    expected_ids = [case["id"] for case in cases]
    observed_ids: list[str] = []
    total_launches = 0
    total_completions = 0
    for result in results:
        exact_keys(result, RESULT_KEYS, "hardware case result")
        case_id = require_id(result["case_id"], "hardware case result id")
        require_sha256(
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
        if result["result"] != "pass" or completions != launches:
            fail(
                f"hardware case did not complete every submitted GPU launch: {case_id}"
            )
        observed_ids.append(case_id)
        total_launches += launches
        total_completions += completions
        if total_launches > MAX_COUNT or total_completions > MAX_COUNT:
            fail("hardware aggregate GPU-work count overflowed")
    if observed_ids != expected_ids:
        fail("hardware case results are duplicated, reordered, omitted, or injected")
    if total_launches == 0 or total_completions != total_launches:
        fail("hardware run contains no completed GPU work")
    return device, environment, total_launches, total_completions


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("hardware-transcript context index format drifted")
    repo = Path(__file__).resolve(strict=True).parents[3]
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements, requirements_raw = load_canonical_json(
        requirements_path, MAX_REPORT_BYTES, "M1 requirements manifest"
    )
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("hardware-transcript context requirements identity drifted")

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
    if not isinstance(context["artifact_absolute_path"], str):
        fail("hardware-transcript report absolute path is invalid")
    report_path = Path(context["artifact_absolute_path"])
    root = evidence_root(report_path, report_relative)
    report, report_bytes = load_canonical_json(
        report_path, MAX_REPORT_BYTES, "hardware-transcript report"
    )
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
    roster_path = reject_symlink_components(
        root, roster_relative, "hardware case roster"
    )
    transcript_path = reject_symlink_components(
        root, transcript_relative, "hardware run transcript"
    )
    roster, roster_bytes = load_canonical_json(
        roster_path, MAX_ROSTER_BYTES, "hardware case roster"
    )
    transcript, transcript_bytes = load_canonical_json(
        transcript_path, MAX_TRANSCRIPT_BYTES, "hardware run transcript"
    )
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
    )
    device, environment, launches, completions = validate_transcript(
        transcript,
        roster_bytes,
        cases,
        binding,
        requirements_sha256,
        expected_sources,
        expected_tcb,
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
    ):
        fail("hardware-transcript report content or identity drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("hardware-transcript validator protocol mismatch")
    context, context_payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(context_payload)}"
    )


if __name__ == "__main__":
    main()
