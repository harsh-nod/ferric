#!/usr/bin/env python3
"""Validate one identity-bound M1 canonical-structure transcript."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, BinaryIO, NoReturn


PROTOCOL = "ferric.m1-validator.canonical-structure.v1"
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REPORT_FORMAT = "FERRIC-M1-CANONICAL-STRUCTURE-V1"
PAYLOAD_FORMAT = "FERRIC-M1-CANONICAL-RECORDS-V1"
PAYLOAD_SCHEMA_ID = "ferric.m1-canonical-records.v1"
ARTIFACT_TARGET = "gfx942:xnack-"
AUTHORITY = "canonical-structure-only"
NONCLAIM = (
    "This transcript establishes only that the referenced bytes conform to "
    "the checker-owned canonical record schema and exact evidence binding. "
    "It grants no semantic correctness, theorem, machine, load, launch, "
    "hardware, performance, or qualification authority."
)
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_FIELD = re.compile(r"[a-z][a-z0-9_.-]{0,127}\Z")
PRINTABLE_ASCII = re.compile(r"[\x20-\x7e]{1,4096}\Z")
MAX_CONTEXT_BYTES = 1_000_000
MAX_REPORT_BYTES = 128_000
MAX_PAYLOAD_BYTES = 1_000_000
MAX_RECORDS = 1_024
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
TCB_KEYS = {"artifact_id", "id", "identity_sha256", "kind"}
REPORT_KEYS = {
    "assurance_property_ids",
    "authority",
    "binding_sha256",
    "canonical_payload_format",
    "canonical_payload_relative_path",
    "canonical_payload_sha256",
    "canonical_payload_size_bytes",
    "canonical_schema_id",
    "canonical_schema_sha256",
    "evidence_kind",
    "format",
    "nonclaim",
    "obligation_class",
    "obligation_id",
    "obligation_state",
    "path_id",
    "path_resolution_sha256",
    "profile_id",
    "record_count",
    "requirements_sha256",
    "result",
    "source_identity_id",
    "source_roster_sha256",
    "statement_sha256",
    "tcb_identity_sha256s",
    "tcb_roster_sha256",
}
PAYLOAD_KEYS = {
    "binding_sha256",
    "format",
    "obligation_class",
    "obligation_id",
    "path_id",
    "profile_id",
    "records",
    "source_identity_id",
    "target",
}
RECORD_KEYS = {"name", "type", "value"}
RECORD_TYPES = {"boolean", "count", "identifier", "sha256", "text"}
PAYLOAD_SCHEMA = {
    "format": PAYLOAD_FORMAT,
    "record_fields": ["name", "type", "value"],
    "record_types": sorted(RECORD_TYPES),
    "required_fields": sorted(PAYLOAD_KEYS),
    "schema_id": PAYLOAD_SCHEMA_ID,
    "target": ARTIFACT_TARGET,
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
        fail("canonical-structure context is empty or oversized")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("canonical-structure context must have one trailing newline")
    payload = raw[:-1]
    try:
        source = payload.decode("ascii")
        value = json.loads(source, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"canonical-structure context is not canonical ASCII JSON: {error}")
    canonical = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    if source != canonical:
        fail("canonical-structure context is not canonical JSON")
    return exact_keys(value, CONTEXT_KEYS, "canonical-structure context"), payload


def file_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
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
        fail("canonical-structure report absolute path is not canonical")
    root = report_path
    for _ in report_relative.parts:
        root = root.parent
    if root.joinpath(*report_relative.parts) != report_path:
        fail("canonical-structure report absolute and relative paths disagree")
    reject_symlink_components(root, report_relative, "canonical-structure report")
    return root


def load_canonical_json(
    path: Path, limit: int, description: str
) -> tuple[dict[str, Any], bytes]:
    raw = read_bounded(path, limit, description)
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail(f"{description} must have one trailing newline")
    try:
        value = json.loads(raw, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"{description} is invalid JSON: {error}")
    expected = (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")
    if raw != expected:
        fail(f"{description} is not canonical JSON")
    if not isinstance(value, dict):
        fail(f"{description} must be an object")
    return value, raw


def validate_sources(value: Any, requirements: dict[str, Any]) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail("canonical-structure source roster is incomplete")
    for record, expected_id in zip(value, SOURCE_IDS, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source context {expected_id}")
        if record["id"] != expected_id:
            fail("canonical-structure source roster order or identity drifted")
        if record["repository"] != SOURCE_REPOSITORIES[expected_id]:
            fail(f"canonical-structure source repository drifted: {expected_id}")
        require_git_id(record["base_commit"], f"{expected_id} base commit")
        require_git_id(record["commit"], f"{expected_id} commit")
        require_git_id(record["tree"], f"{expected_id} tree")
        require_id(record["source_closure_artifact_id"], f"{expected_id} closure")
        require_sha256(record["source_closure_sha256"], f"{expected_id} source closure")
    if value[0]["base_commit"] != requirements["m1_upstream_base_commit"]:
        fail("canonical-structure fe2o3 base identity drifted")
    if value[1]["base_commit"] != FERRIC_BASE_COMMIT:
        fail("canonical-structure Ferric base identity drifted")
    return value


def validate_tcb(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("canonical-structure TCB roster is incomplete")
    for record, expected_id in zip(value, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, f"TCB context {expected_id}")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("canonical-structure TCB order, identity, or kind drifted")
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
        fail("canonical-structure requires every M1 obligation to remain Open")
    if obligation_class == "Roadmap":
        matches = [record for record in roadmaps if record["id"] == obligation_id]
        if len(matches) != 1:
            fail("canonical-structure binding names an unknown roadmap obligation")
        record = matches[0]
        return record, record["title"], record["assurance_properties"]
    if obligation_class == "Assurance":
        matches = [record for record in properties if record["name"] == obligation_id]
        if len(matches) != 1:
            fail("canonical-structure binding names an unknown assurance property")
        record = matches[0]
        return record, record["boundary"], [obligation_id]
    fail("canonical-structure obligation class drifted")


def validate_record(record: Any) -> None:
    exact_keys(record, RECORD_KEYS, "canonical payload record")
    name = record["name"]
    record_type = record["type"]
    value = record["value"]
    if not isinstance(name, str) or SAFE_FIELD.fullmatch(name) is None:
        fail("canonical payload record name is invalid")
    if record_type not in RECORD_TYPES:
        fail(f"canonical payload record type is invalid: {name}")
    if record_type == "boolean":
        valid = isinstance(value, bool)
    elif record_type == "count":
        valid = (
            isinstance(value, int)
            and not isinstance(value, bool)
            and 0 <= value <= MAX_COUNT
        )
    elif record_type == "identifier":
        valid = isinstance(value, str) and SAFE_NAME.fullmatch(value) is not None
    elif record_type == "sha256":
        valid = (
            isinstance(value, str)
            and SHA256.fullmatch(value) is not None
            and len(set(value)) != 1
        )
    else:
        valid = isinstance(value, str) and PRINTABLE_ASCII.fullmatch(value) is not None
    if not valid:
        fail(f"canonical payload record value is invalid: {name}")


def validate_payload(
    payload: dict[str, Any], binding: dict[str, Any]
) -> list[dict[str, Any]]:
    exact_keys(payload, PAYLOAD_KEYS, "canonical payload")
    if (
        payload["format"] != PAYLOAD_FORMAT
        or payload["target"] != ARTIFACT_TARGET
        or payload["binding_sha256"] != binding["binding_sha256"]
        or payload["obligation_class"] != binding["obligation_class"]
        or payload["obligation_id"] != binding["obligation_id"]
        or payload["path_id"] != binding["path_id"]
        or payload["profile_id"] != binding["profile_id"]
        or payload["source_identity_id"] != binding["source_identity_id"]
    ):
        fail("canonical payload identity or target drifted")
    records = payload["records"]
    if not isinstance(records, list) or not records or len(records) > MAX_RECORDS:
        fail("canonical payload record roster is empty or oversized")
    for record in records:
        validate_record(record)
    names = [record["name"] for record in records]
    if names != sorted(names) or len(names) != len(set(names)):
        fail("canonical payload records are duplicated or not canonically ordered")
    return records


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("canonical-structure context index format drifted")
    repo = Path(__file__).resolve(strict=True).parents[3]
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements, requirements_raw = load_canonical_json(
        requirements_path, MAX_REPORT_BYTES, "M1 requirements manifest"
    )
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("canonical-structure context requirements identity drifted")

    artifact = exact_keys(context["artifact"], ARTIFACT_KEYS, "artifact context")
    artifact_id = require_id(artifact["id"], "artifact id")
    if artifact["kind"] != "CheckerTranscript":
        fail("canonical-structure transcript kind drifted")
    report_relative = safe_relative(artifact["path"], "report relative path")
    expected_report_path = f"artifacts/{artifact_id}.canonical-structure.json"
    if report_relative.as_posix() != expected_report_path:
        fail("canonical-structure report path is not canonical for its artifact id")
    report_sha256 = require_sha256(artifact["sha256"], "report SHA-256")
    if (
        not isinstance(artifact["size_bytes"], int)
        or isinstance(artifact["size_bytes"], bool)
        or artifact["size_bytes"] <= 0
    ):
        fail("canonical-structure report size is invalid")
    if not isinstance(context["artifact_absolute_path"], str):
        fail("canonical-structure report absolute path is invalid")
    report_path = Path(context["artifact_absolute_path"])
    root = evidence_root(report_path, report_relative)
    report, report_bytes = load_canonical_json(
        report_path, MAX_REPORT_BYTES, "canonical-structure report"
    )
    exact_keys(report, REPORT_KEYS, "canonical-structure report")
    if (
        len(report_bytes) != artifact["size_bytes"]
        or digest_bytes(report_bytes) != report_sha256
    ):
        fail("canonical-structure report bytes do not match their context identity")

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
        or binding["evidence_kind"] != "canonical-structure-check"
        or binding["source_identity_id"] not in SOURCE_IDS
        or binding["tcb_ids"] != list(TCB_IDS)
    ):
        fail("canonical-structure binding context drifted")
    binding_payload = {
        key: value for key, value in binding.items() if key != "binding_sha256"
    }
    if binding["binding_sha256"] != canonical_digest(binding_payload):
        fail("canonical-structure binding identity mismatch")

    spec, statement, assurance_property_ids = requirements_spec(
        requirements, binding["obligation_class"], binding["obligation_id"]
    )
    profiles = {
        record["id"]: record["kinds"] for record in requirements["evidence_profiles"]
    }
    if (
        binding["profile_id"] not in spec["evidence_profiles"]
        or "canonical-structure-check" not in profiles.get(binding["profile_id"], [])
        or binding["path_id"] not in spec["path_obligations"]
        or binding["statement_sha256"] != digest_bytes(statement.encode("utf-8"))
    ):
        fail("canonical-structure obligation, profile, path, or statement drifted")

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
        fail("canonical-structure path resolution drifted")

    sources = validate_sources(context["sources"], requirements)
    tcb = validate_tcb(context["tcb"])
    expected_tcb_identities = {
        record["id"]: record["identity_sha256"] for record in tcb
    }

    payload_relative = safe_relative(
        report["canonical_payload_relative_path"], "canonical payload relative path"
    )
    expected_payload_path = f"canonical-payloads/{artifact_id}.json"
    if payload_relative.as_posix() != expected_payload_path:
        fail("canonical payload path is not canonical for its artifact id")
    payload_path = reject_symlink_components(
        root, payload_relative, "canonical payload"
    )
    payload, payload_bytes = load_canonical_json(
        payload_path, MAX_PAYLOAD_BYTES, "canonical payload"
    )
    records = validate_payload(payload, binding)
    payload_sha256 = require_sha256(
        report["canonical_payload_sha256"], "canonical payload SHA-256"
    )
    if (
        not isinstance(report["canonical_payload_size_bytes"], int)
        or isinstance(report["canonical_payload_size_bytes"], bool)
        or report["canonical_payload_size_bytes"] <= 0
        or report["canonical_payload_size_bytes"] != len(payload_bytes)
        or payload_sha256 != digest_bytes(payload_bytes)
    ):
        fail("canonical payload bytes do not match the report identity")

    if (
        report["format"] != REPORT_FORMAT
        or report["authority"] != AUTHORITY
        or report["nonclaim"] != NONCLAIM
        or report["evidence_kind"] != "canonical-structure-check"
        or report["result"] != "canonical"
        or report["canonical_payload_format"] != PAYLOAD_FORMAT
        or report["canonical_schema_id"] != PAYLOAD_SCHEMA_ID
        or report["canonical_schema_sha256"] != canonical_digest(PAYLOAD_SCHEMA)
        or report["record_count"] != len(records)
        or isinstance(report["record_count"], bool)
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
        or report["source_roster_sha256"] != canonical_digest(sources)
        or report["statement_sha256"] != binding["statement_sha256"]
        or report["tcb_identity_sha256s"] != expected_tcb_identities
        or report["tcb_roster_sha256"] != canonical_digest(tcb)
    ):
        fail("canonical-structure report content or identity drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("canonical-structure validator protocol mismatch")
    context, context_payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(context_payload)}"
    )


if __name__ == "__main__":
    main()
