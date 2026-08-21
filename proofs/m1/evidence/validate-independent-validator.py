#!/usr/bin/env python3
"""Validate one canonical, identity-bound M1 independent-validator report."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, BinaryIO, NoReturn


PROTOCOL = "ferric.m1-validator.independent-validator.v1"
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REPORT_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-REPORT-V1"
ROSTER_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-ROSTER-V1"
TRANSCRIPT_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATOR-TRANSCRIPT-V1"
VALIDATOR_PROTOCOL = "ferric.external-independent-validation.v1"
TARGET = "gfx942:xnack-"
AUTHORITY = "independent-validation-observation-only"
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
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+\Z")
UTC_TIME = re.compile(
    r"20[0-9]{2}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12][0-9]|3[01])"
    r"T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z\Z"
)
MAX_CONTEXT_BYTES = 1_000_000
MAX_REPORT_BYTES = 128_000
MAX_ROSTER_BYTES = 128_000
MAX_TRANSCRIPT_BYTES = 256_000
MAX_REQUIREMENTS_BYTES = 1_000_000
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {"source.fe2o3": "fe2o3", "source.ferric": "ferric"}
SUBJECT_ORGANIZATIONS = {"fe2o3", "ferric", "harsh-nod"}
SUBJECT_REPOSITORIES = {"fe2o3", "ferric"}
TRUSTED_CHECKER_PATH = "proofs/m1/evidence/validate-independent-validator.py"
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
PROPERTY_KEYS = {
    "boundary_sha256",
    "fe2o3_kind",
    "name",
    "obligation_state",
    "required_status_at_closure",
}
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
CASE_KEYS = {"expected_status", "id", "input_sha256", "output_sha256"}
RESULT_KEYS = {
    "exit_code",
    "expected_status",
    "id",
    "input_sha256",
    "observed_status",
    "output_sha256",
}
ROSTER_KEYS = {
    "assurance_property_bindings_sha256",
    "binding_sha256",
    "cases",
    "checker",
    "format",
    "path_resolution_sha256",
    "profile_id",
    "requirements_sha256",
    "source_roster_sha256",
    "target",
    "tcb_roster_sha256",
}
TRANSCRIPT_KEYS = {
    "binding_sha256",
    "case_counts",
    "checker_identity_sha256",
    "completed_at_utc",
    "format",
    "results",
    "roster_sha256",
    "started_at_utc",
    "validation_status",
}
REPORT_KEYS = {
    "assurance_property_bindings",
    "authority",
    "binding_sha256",
    "case_counts",
    "checker_id",
    "checker_identity_sha256",
    "checker_organization",
    "evidence_kind",
    "format",
    "independence_attestation",
    "nonclaim",
    "obligation_class",
    "obligation_id",
    "obligation_state",
    "path_id",
    "path_resolution_sha256",
    "profile_id",
    "requirements_sha256",
    "roster_path",
    "roster_sha256",
    "roster_size_bytes",
    "source_closure_sha256s",
    "source_identity_id",
    "source_identity_sha256s",
    "source_roster_sha256",
    "statement_sha256",
    "target",
    "tcb_identity_sha256s",
    "tcb_roster_sha256",
    "transcript_path",
    "transcript_sha256",
    "transcript_size_bytes",
    "validation_status",
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


def require_size(value: Any, description: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
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
        fail("independent-validator context is empty or oversized")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("independent-validator context must have one trailing newline")
    payload = raw[:-1]
    try:
        source = payload.decode("ascii")
        value = json.loads(source, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"independent-validator context is not canonical ASCII JSON: {error}")
    canonical = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    if source != canonical:
        fail("independent-validator context is not canonical JSON")
    return exact_keys(value, CONTEXT_KEYS, "independent-validator context"), payload


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
        fail("independent-validator report absolute path is not canonical")
    root = report_path
    for _ in report_relative.parts:
        root = root.parent
    if root.joinpath(*report_relative.parts) != report_path:
        fail("independent-validator report absolute and relative paths disagree")
    reject_symlink_components(root, report_relative, "independent-validator report")
    return root


def load_canonical(
    path: Path, limit: int, description: str, keys: set[str]
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
    return exact_keys(value, keys, description), raw


def validate_sources(value: Any, requirements: dict[str, Any]) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail("independent-validator source roster is incomplete")
    for record, expected_id in zip(value, SOURCE_IDS, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source context {expected_id}")
        if record["id"] != expected_id:
            fail("independent-validator source roster order or identity drifted")
        if record["repository"] != SOURCE_REPOSITORIES[expected_id]:
            fail(f"independent-validator source repository drifted: {expected_id}")
        require_git_id(record["base_commit"], f"{expected_id} base commit")
        require_git_id(record["commit"], f"{expected_id} commit")
        require_git_id(record["tree"], f"{expected_id} tree")
        require_id(record["source_closure_artifact_id"], f"{expected_id} closure")
        require_sha256(record["source_closure_sha256"], f"{expected_id} closure")
    if value[0]["base_commit"] != requirements["m1_upstream_base_commit"]:
        fail("independent-validator fe2o3 base identity drifted")
    if value[1]["base_commit"] != FERRIC_BASE_COMMIT:
        fail("independent-validator Ferric base identity drifted")
    return value


def validate_tcb(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("independent-validator TCB roster is incomplete")
    for record, expected_id in zip(value, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, f"TCB context {expected_id}")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("independent-validator TCB order, identity, or kind drifted")
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
        fail("independent validation requires all M1 states to remain Open")
    if obligation_class == "Roadmap":
        matches = [record for record in roadmaps if record["id"] == obligation_id]
        if len(matches) != 1:
            fail("independent-validator names an unknown roadmap obligation")
        record = matches[0]
        return record, record["title"], record["assurance_properties"]
    if obligation_class == "Assurance":
        matches = [record for record in properties if record["name"] == obligation_id]
        if len(matches) != 1:
            fail("independent-validator names an unknown assurance property")
        record = matches[0]
        return record, record["boundary"], [obligation_id]
    fail("independent-validator obligation class drifted")


def property_bindings(
    requirements: dict[str, Any], identifiers: list[str]
) -> list[dict[str, Any]]:
    by_name = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    return [
        {
            "boundary_sha256": digest_bytes(by_name[identifier]["boundary"].encode()),
            "fe2o3_kind": by_name[identifier]["fe2o3_kind"],
            "name": identifier,
            "obligation_state": "Open",
            "required_status_at_closure": by_name[identifier][
                "required_status_at_closure"
            ],
        }
        for identifier in identifiers
    ]


def validate_checker(value: Any, sources: list[dict[str, Any]]) -> dict[str, Any]:
    checker = exact_keys(value, CHECKER_KEYS, "independent checker identity")
    require_id(checker["id"], "independent checker id")
    organization = require_id(checker["organization"], "checker organization")
    repository = require_id(checker["repository"], "checker repository")
    commit = require_git_id(checker["commit"], "checker commit")
    tree = require_git_id(checker["tree"], "checker tree")
    closure = require_sha256(checker["source_closure_sha256"], "checker closure")
    executable = safe_relative(checker["executable_path"], "checker executable path")
    executable_sha = require_sha256(
        checker["executable_sha256"], "checker executable identity"
    )
    require_sha256(checker["input_schema_sha256"], "checker input schema")
    require_sha256(checker["output_schema_sha256"], "checker output schema")
    if (
        not isinstance(checker["version"], str)
        or VERSION.fullmatch(checker["version"]) is None
    ):
        fail("invalid checker version")
    if checker["protocol"] != VALIDATOR_PROTOCOL:
        fail("independent checker protocol drifted")
    if (
        organization in SUBJECT_ORGANIZATIONS
        or repository in SUBJECT_REPOSITORIES
        or checker["id"] in SOURCE_IDS
        or executable.as_posix() == TRUSTED_CHECKER_PATH
        or commit in {record["commit"] for record in sources}
        or tree in {record["tree"] for record in sources}
        or closure in {record["source_closure_sha256"] for record in sources}
        or executable_sha in {record["source_closure_sha256"] for record in sources}
    ):
        fail("self-validation or subject-source checker substitution detected")
    identities = [
        checker["executable_sha256"],
        checker["input_schema_sha256"],
        checker["output_schema_sha256"],
        checker["source_closure_sha256"],
    ]
    if len(identities) != len(set(identities)):
        fail("independent checker identities are not distinct")
    return checker


def validate_cases(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(CASE_MATRIX):
        fail("independent-validator case roster is incomplete")
    inputs: list[str] = []
    outputs: list[str] = []
    for record, (expected_id, expected_status) in zip(value, CASE_MATRIX, strict=True):
        exact_keys(record, CASE_KEYS, f"case roster {expected_id}")
        if record["id"] != expected_id or record["expected_status"] != expected_status:
            fail("independent-validator case identity, order, or expectation drifted")
        inputs.append(require_sha256(record["input_sha256"], f"{expected_id} input"))
        outputs.append(require_sha256(record["output_sha256"], f"{expected_id} output"))
    if (
        len(inputs) != len(set(inputs))
        or len(outputs) != len(set(outputs))
        or set(inputs) & set(outputs)
    ):
        fail("independent-validator case input/output identities are not exact")
    return value


def validate_results(value: Any, cases: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(CASE_MATRIX):
        fail("independent-validator transcript skipped a required case")
    for result, case in zip(value, cases, strict=True):
        exact_keys(result, RESULT_KEYS, f"case result {case['id']}")
        expected_exit = 0 if case["expected_status"] == "PASS" else 1
        if (
            result["id"] != case["id"]
            or result["expected_status"] != case["expected_status"]
            or result["observed_status"] != case["expected_status"]
            or result["input_sha256"] != case["input_sha256"]
            or result["output_sha256"] != case["output_sha256"]
            or not isinstance(result["exit_code"], int)
            or isinstance(result["exit_code"], bool)
            or result["exit_code"] != expected_exit
        ):
            fail("independent-validator case result or status drifted")
    return value


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("independent-validator context index format drifted")
    repo = Path(__file__).resolve(strict=True).parents[3]
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements, requirements_raw = load_canonical(
        requirements_path,
        MAX_REQUIREMENTS_BYTES,
        "M1 requirements manifest",
        {
            "assurance_properties",
            "evidence_kinds",
            "evidence_profiles",
            "format",
            "m0_contracts_commit",
            "m1_upstream_base_commit",
            "m1_upstream_base_tree",
            "milestone",
            "path_obligations",
            "roadmap_requirements",
        },
    )
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("independent-validator context requirements identity drifted")

    artifact = exact_keys(context["artifact"], ARTIFACT_KEYS, "artifact context")
    artifact_id = require_id(artifact["id"], "artifact id")
    if artifact["kind"] != "ValidatorTranscript":
        fail("independent-validator report kind drifted")
    report_relative = safe_relative(artifact["path"], "report relative path")
    expected_report_path = f"artifacts/{artifact_id}.independent-validator.json"
    if report_relative.as_posix() != expected_report_path:
        fail("independent-validator report path is not canonical")
    report_sha256 = require_sha256(artifact["sha256"], "report SHA-256")
    report_size = require_size(artifact["size_bytes"], "report size")
    if not isinstance(context["artifact_absolute_path"], str):
        fail("independent-validator report absolute path is invalid")
    report_path = Path(context["artifact_absolute_path"])
    root = evidence_root(report_path, report_relative)
    report, report_raw = load_canonical(
        report_path, MAX_REPORT_BYTES, "independent-validator report", REPORT_KEYS
    )
    if len(report_raw) != report_size or digest_bytes(report_raw) != report_sha256:
        fail("independent-validator report bytes do not match context identity")

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
        or binding["evidence_kind"] != "independent-validator"
        or binding["source_identity_id"] not in SOURCE_IDS
        or binding["tcb_ids"] != list(TCB_IDS)
    ):
        fail("independent-validator binding context drifted")
    binding_payload = {
        key: value for key, value in binding.items() if key != "binding_sha256"
    }
    if binding["binding_sha256"] != canonical_digest(binding_payload):
        fail("independent-validator binding identity mismatch")

    spec, statement, property_ids = requirements_spec(
        requirements, binding["obligation_class"], binding["obligation_id"]
    )
    profiles = {
        record["id"]: record["kinds"] for record in requirements["evidence_profiles"]
    }
    if (
        binding["profile_id"] not in spec["evidence_profiles"]
        or "independent-validator" not in profiles.get(binding["profile_id"], [])
        or binding["path_id"] not in spec["path_obligations"]
        or binding["statement_sha256"] != digest_bytes(statement.encode())
    ):
        fail("independent-validator obligation, profile, path, or statement drifted")
    properties = property_bindings(requirements, property_ids)
    for record in properties:
        exact_keys(record, PROPERTY_KEYS, f"assurance property {record['name']}")

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
        fail("independent-validator path resolution drifted")

    sources = validate_sources(context["sources"], requirements)
    tcb = validate_tcb(context["tcb"])
    source_identities = {record["id"]: canonical_digest(record) for record in sources}
    source_closures = {
        record["id"]: record["source_closure_sha256"] for record in sources
    }
    tcb_identities = {record["id"]: record["identity_sha256"] for record in tcb}

    roster_relative = safe_relative(report["roster_path"], "validator roster path")
    transcript_relative = safe_relative(
        report["transcript_path"], "validator transcript path"
    )
    expected_roster = f"validator-runs/{artifact_id}.independent-validator.roster.json"
    expected_transcript = (
        f"validator-runs/{artifact_id}.independent-validator.transcript.json"
    )
    if (
        roster_relative.as_posix() != expected_roster
        or transcript_relative.as_posix() != expected_transcript
    ):
        fail("independent-validator companion path is not canonical")
    roster_path = reject_symlink_components(root, roster_relative, "validator roster")
    transcript_path = reject_symlink_components(
        root, transcript_relative, "validator transcript"
    )
    roster, roster_raw = load_canonical(
        roster_path, MAX_ROSTER_BYTES, "validator roster", ROSTER_KEYS
    )
    transcript, transcript_raw = load_canonical(
        transcript_path, MAX_TRANSCRIPT_BYTES, "validator transcript", TRANSCRIPT_KEYS
    )
    if (
        require_size(report["roster_size_bytes"], "roster size") != len(roster_raw)
        or require_sha256(report["roster_sha256"], "roster SHA-256")
        != digest_bytes(roster_raw)
        or require_size(report["transcript_size_bytes"], "transcript size")
        != len(transcript_raw)
        or require_sha256(report["transcript_sha256"], "transcript SHA-256")
        != digest_bytes(transcript_raw)
    ):
        fail("independent-validator companion identity drifted")

    checker = validate_checker(roster["checker"], sources)
    cases = validate_cases(roster["cases"])
    checker_identity = canonical_digest(checker)
    property_identity = canonical_digest(properties)
    source_roster_identity = canonical_digest(sources)
    tcb_roster_identity = canonical_digest(tcb)
    path_identity = canonical_digest(resolution)
    if (
        roster["format"] != ROSTER_FORMAT
        or roster["binding_sha256"] != binding["binding_sha256"]
        or roster["assurance_property_bindings_sha256"] != property_identity
        or roster["path_resolution_sha256"] != path_identity
        or roster["profile_id"] != binding["profile_id"]
        or roster["requirements_sha256"] != requirements_sha256
        or roster["source_roster_sha256"] != source_roster_identity
        or roster["target"] != TARGET
        or roster["tcb_roster_sha256"] != tcb_roster_identity
    ):
        fail("independent-validator roster binding drifted")

    started = transcript["started_at_utc"]
    completed = transcript["completed_at_utc"]
    if (
        not isinstance(started, str)
        or UTC_TIME.fullmatch(started) is None
        or not isinstance(completed, str)
        or UTC_TIME.fullmatch(completed) is None
        or completed < started
    ):
        fail("independent-validator transcript time is malformed")
    validate_results(transcript["results"], cases)
    if (
        transcript["format"] != TRANSCRIPT_FORMAT
        or transcript["binding_sha256"] != binding["binding_sha256"]
        or transcript["case_counts"] != CASE_COUNTS
        or transcript["checker_identity_sha256"] != checker_identity
        or transcript["roster_sha256"] != digest_bytes(roster_raw)
        or transcript["validation_status"] != "PASS"
    ):
        fail("independent-validator transcript binding or status drifted")

    if (
        report["format"] != REPORT_FORMAT
        or report["evidence_kind"] != "independent-validator"
        or report["authority"] != AUTHORITY
        or report["independence_attestation"] != INDEPENDENCE_ATTESTATION
        or report["nonclaim"] != NONCLAIM
        or report["binding_sha256"] != binding["binding_sha256"]
        or report["obligation_class"] != binding["obligation_class"]
        or report["obligation_id"] != binding["obligation_id"]
        or report["obligation_state"] != "Open"
        or report["statement_sha256"] != binding["statement_sha256"]
        or report["assurance_property_bindings"] != properties
        or report["path_id"] != binding["path_id"]
        or report["path_resolution_sha256"] != path_identity
        or report["profile_id"] != binding["profile_id"]
        or report["requirements_sha256"] != requirements_sha256
        or report["source_identity_id"] != binding["source_identity_id"]
        or report["source_identity_sha256s"] != source_identities
        or report["source_closure_sha256s"] != source_closures
        or report["source_roster_sha256"] != source_roster_identity
        or report["target"] != TARGET
        or report["tcb_identity_sha256s"] != tcb_identities
        or report["tcb_roster_sha256"] != tcb_roster_identity
        or report["checker_id"] != checker["id"]
        or report["checker_organization"] != checker["organization"]
        or report["checker_identity_sha256"] != checker_identity
        or report["case_counts"] != CASE_COUNTS
        or report["validation_status"] != "PASS"
    ):
        fail("independent-validator report content or identity drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("independent-validator protocol mismatch")
    context, context_payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(context_payload)}"
    )


if __name__ == "__main__":
    main()
