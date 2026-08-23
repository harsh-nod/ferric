#!/usr/bin/env python3
"""Validate one canonical, identity-only M1 artifact report."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, BinaryIO, NoReturn


PROTOCOL = "ferric.m1-validator.artifact-identity.v1"
OBLIGATION_CLASSES = ("Assurance", "Roadmap")
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REPORT_FORMAT = "FERRIC-M1-ARTIFACT-IDENTITY-V1"
ARTIFACT_KIND = "M1ImmutablePayload"
ARTIFACT_TARGET = "gfx942:xnack-"
AUTHORITY = "identity-and-structure-only"
NONCLAIM = (
    "This report authenticates byte identity and canonical structure only. "
    "It grants no semantic correctness, theorem, machine, load, launch, "
    "hardware, performance, or qualification authority."
)
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
MAX_CONTEXT_BYTES = 1_000_000
MAX_REPORT_BYTES = 128_000
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
    "artifact_kind",
    "artifact_profile_id",
    "artifact_relative_path",
    "artifact_sha256",
    "artifact_size_bytes",
    "artifact_target",
    "assurance_property_ids",
    "authority",
    "binding_sha256",
    "evidence_kind",
    "format",
    "nonclaim",
    "obligation_class",
    "obligation_id",
    "obligation_state",
    "path_id",
    "path_resolution_sha256",
    "requirements_sha256",
    "source_identity_id",
    "source_roster_sha256",
    "statement_sha256",
    "tcb_identity_sha256s",
    "tcb_roster_sha256",
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
        fail("artifact-identity context is empty or oversized")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("artifact-identity context must have one trailing newline")
    payload = raw[:-1]
    try:
        source = payload.decode("ascii")
        value = json.loads(source, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"artifact-identity context is not canonical ASCII JSON: {error}")
    canonical = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    if source != canonical:
        fail("artifact-identity context is not canonical JSON")
    return exact_keys(value, CONTEXT_KEYS, "artifact-identity context"), payload


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


def hash_regular(path: Path, description: str) -> tuple[int, str]:
    source, before = open_regular(path, description)
    hasher = hashlib.sha256()
    observed = 0
    try:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            observed += len(block)
            hasher.update(block)
        after = os.fstat(source.fileno())
    except OSError as error:
        fail(f"cannot hash {description}: {error}")
    finally:
        source.close()
    if (
        before.st_size <= 0
        or observed != before.st_size
        or file_identity(before) != file_identity(after)
    ):
        fail(f"{description} changed while it was hashed")
    return observed, hasher.hexdigest()


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
        fail("artifact-identity report absolute path is not canonical")
    root = report_path
    for _ in report_relative.parts:
        root = root.parent
    if root.joinpath(*report_relative.parts) != report_path:
        fail("artifact-identity report absolute and relative paths disagree")
    reject_symlink_components(root, report_relative, "artifact-identity report")
    return root


def load_report(path: Path) -> tuple[dict[str, Any], bytes]:
    raw = read_bounded(path, MAX_REPORT_BYTES, "artifact-identity report")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("artifact-identity report must have one trailing newline")
    try:
        value = json.loads(raw, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"artifact-identity report is invalid JSON: {error}")
    expected = (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")
    if raw != expected:
        fail("artifact-identity report is not canonical JSON")
    return exact_keys(value, REPORT_KEYS, "artifact-identity report"), raw


def validate_sources(value: Any, requirements: dict[str, Any]) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail("artifact-identity source roster is incomplete")
    for record, expected_id in zip(value, SOURCE_IDS, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source context {expected_id}")
        if record["id"] != expected_id:
            fail("artifact-identity source roster order or identity drifted")
        if record["repository"] != SOURCE_REPOSITORIES[expected_id]:
            fail(f"artifact-identity source repository drifted: {expected_id}")
        require_git_id(record["base_commit"], f"{expected_id} base commit")
        require_git_id(record["commit"], f"{expected_id} commit")
        require_git_id(record["tree"], f"{expected_id} tree")
        require_id(
            record["source_closure_artifact_id"], f"{expected_id} closure artifact"
        )
        require_sha256(record["source_closure_sha256"], f"{expected_id} source closure")
    if value[0]["base_commit"] != requirements["m1_upstream_base_commit"]:
        fail("artifact-identity fe2o3 base identity drifted")
    if value[1]["base_commit"] != FERRIC_BASE_COMMIT:
        fail("artifact-identity Ferric base identity drifted")
    return value


def validate_tcb(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("artifact-identity TCB roster is incomplete")
    for record, expected_id in zip(value, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, f"TCB context {expected_id}")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("artifact-identity TCB order, identity, or kind drifted")
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
        fail(
            "artifact-identity requires all M1 obligations and properties to remain Open"
        )
    if obligation_class == "Roadmap":
        matches = [record for record in roadmaps if record["id"] == obligation_id]
        if len(matches) != 1:
            fail("artifact-identity binding names an unknown roadmap obligation")
        record = matches[0]
        return record, record["title"], record["assurance_properties"]
    if obligation_class == "Assurance":
        matches = [record for record in properties if record["name"] == obligation_id]
        if len(matches) != 1:
            fail("artifact-identity binding names an unknown assurance property")
        record = matches[0]
        return record, record["boundary"], [obligation_id]
    fail("artifact-identity obligation class drifted")


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("artifact-identity context index format drifted")
    repo = Path(__file__).resolve(strict=True).parents[3]
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements_raw = read_bounded(
        requirements_path, MAX_REPORT_BYTES, "M1 requirements manifest"
    )
    try:
        requirements = json.loads(
            requirements_raw, object_pairs_hook=reject_duplicate_key
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"M1 requirements manifest is invalid JSON: {error}")
    expected_requirements = (
        json.dumps(requirements, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")
    if requirements_raw != expected_requirements:
        fail("M1 requirements manifest is not canonical JSON")
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("artifact-identity context requirements identity drifted")

    artifact = exact_keys(context["artifact"], ARTIFACT_KEYS, "artifact context")
    artifact_id = require_id(artifact["id"], "artifact id")
    if artifact["kind"] != "ArtifactIdentityReport":
        fail("artifact-identity report kind drifted")
    report_relative = safe_relative(artifact["path"], "report relative path")
    expected_report_path = f"artifacts/{artifact_id}.artifact-identity.json"
    if report_relative.as_posix() != expected_report_path:
        fail("artifact-identity report path is not canonical for its artifact id")
    report_sha256 = require_sha256(artifact["sha256"], "report SHA-256")
    if (
        not isinstance(artifact["size_bytes"], int)
        or isinstance(artifact["size_bytes"], bool)
        or artifact["size_bytes"] <= 0
    ):
        fail("artifact-identity report size is invalid")
    if not isinstance(context["artifact_absolute_path"], str):
        fail("artifact-identity report absolute path is invalid")
    report_path = Path(context["artifact_absolute_path"])
    root = evidence_root(report_path, report_relative)
    report, report_bytes = load_report(report_path)
    if (
        len(report_bytes) != artifact["size_bytes"]
        or digest_bytes(report_bytes) != report_sha256
    ):
        fail("artifact-identity report bytes do not match their context identity")

    binding = exact_keys(context["binding"], BINDING_KEYS, "binding context")
    for key in ("artifact_id", "id"):
        require_id(binding[key], f"binding {key}")
    for key in (
        "obligation_id",
        "path_id",
        "profile_id",
        "source_identity_id",
    ):
        require_name(binding[key], f"binding {key}")
    require_sha256(binding["binding_sha256"], "binding SHA-256")
    require_sha256(binding["statement_sha256"], "binding statement SHA-256")
    if (
        context["subject"] != f"binding:{binding['id']}"
        or binding["artifact_id"] != artifact_id
        or binding["evidence_kind"] != "artifact-identity"
        or binding["source_identity_id"] not in SOURCE_IDS
        or binding["tcb_ids"] != list(TCB_IDS)
    ):
        fail("artifact-identity binding context drifted")
    binding_payload = {
        key: value for key, value in binding.items() if key != "binding_sha256"
    }
    if binding["binding_sha256"] != canonical_digest(binding_payload):
        fail("artifact-identity binding identity mismatch")

    spec, statement, assurance_property_ids = requirements_spec(
        requirements, binding["obligation_class"], binding["obligation_id"]
    )
    profiles = {
        record["id"]: record["kinds"] for record in requirements["evidence_profiles"]
    }
    if (
        binding["profile_id"] not in spec["evidence_profiles"]
        or "artifact-identity" not in profiles.get(binding["profile_id"], [])
        or binding["path_id"] not in spec["path_obligations"]
        or binding["statement_sha256"] != digest_bytes(statement.encode("utf-8"))
    ):
        fail("artifact-identity obligation, profile, path, or statement drifted")

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
        fail("artifact-identity path resolution drifted")

    sources = validate_sources(context["sources"], requirements)
    tcb = validate_tcb(context["tcb"])
    expected_tcb_identities = {
        record["id"]: record["identity_sha256"] for record in tcb
    }

    payload_relative = safe_relative(
        report["artifact_relative_path"], "identified artifact relative path"
    )
    expected_payload_path = f"identified-artifacts/{artifact_id}.bin"
    payload_path = reject_symlink_components(
        root, payload_relative, "identified artifact"
    )
    payload_size, payload_sha256 = hash_regular(payload_path, "identified artifact")
    if (
        report["format"] != REPORT_FORMAT
        or report["authority"] != AUTHORITY
        or report["nonclaim"] != NONCLAIM
        or report["evidence_kind"] != "artifact-identity"
        or report["artifact_kind"] != ARTIFACT_KIND
        or report["artifact_target"] != ARTIFACT_TARGET
        or report["artifact_profile_id"] != binding["profile_id"]
        or payload_relative.as_posix() != expected_payload_path
        or not isinstance(report["artifact_size_bytes"], int)
        or isinstance(report["artifact_size_bytes"], bool)
        or report["artifact_size_bytes"] <= 0
        or report["artifact_size_bytes"] != payload_size
        or require_sha256(report["artifact_sha256"], "identified artifact SHA-256")
        != payload_sha256
        or report["binding_sha256"] != binding["binding_sha256"]
        or report["obligation_class"] != binding["obligation_class"]
        or report["obligation_id"] != binding["obligation_id"]
        or report["obligation_state"] != "Open"
        or report["assurance_property_ids"] != assurance_property_ids
        or report["path_id"] != binding["path_id"]
        or report["path_resolution_sha256"] != canonical_digest(resolution)
        or report["requirements_sha256"] != requirements_sha256
        or report["source_identity_id"] != binding["source_identity_id"]
        or report["source_roster_sha256"] != canonical_digest(sources)
        or report["statement_sha256"] != binding["statement_sha256"]
        or report["tcb_identity_sha256s"] != expected_tcb_identities
        or report["tcb_roster_sha256"] != canonical_digest(tcb)
    ):
        fail("artifact-identity report content or identity drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("artifact-identity validator protocol mismatch")
    context, context_payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(context_payload)}"
    )


if __name__ == "__main__":
    main()
