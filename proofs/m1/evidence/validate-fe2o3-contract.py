#!/usr/bin/env python3
"""Validate one canonical, identity-bound M1 fe2o3 contract report."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, BinaryIO, NoReturn


PROTOCOL = "ferric.m1-validator.fe2o3-contract.v1"
OBLIGATION_CLASSES = ("Assurance", "Roadmap")
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REPORT_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-V1"
CONTRACT_BODY_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-BODY-V1"
CONTRACT_SET_FORMAT = "FERRIC-M1-FE2O3-CONTRACT-SET-V1"
CONTRACT_SET_SCHEMA = "fe2o3-proof-contracts::ContractSetV1"
CONTRACT_SET_SOURCE_PATH = "crates/fe2o3-proof-contracts/src/model.rs"
CONTRACT_SET_VALIDATION = "ContractSetV1::validate_closed-structural-only"
PROPERTY_KIND_NAMESPACE = "harsh-nod.ferric.m1.fe2o3-contract-binding.v1"
PROPERTY_KIND_CODE = 1
CONTRACT_TARGET = "gfx942:xnack-"
AUTHORITY = "contract-declaration-structure-only"
NONCLAIM = (
    "This report authenticates an exact fe2o3 ContractSetV1 and Contracted "
    "property declaration only. A contract is not implementation or proof and "
    "grants no machine-refinement, load, launch, hardware, performance, or "
    "qualification authority."
)
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
MAX_CONTEXT_BYTES = 1_000_000
MAX_REPORT_BYTES = 128_000
MAX_CONTRACT_BODY_BYTES = 128_000
MAX_CONTRACT_SET_BYTES = 128_000
MAX_REQUIREMENTS_BYTES = 1_000_000
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
PROPERTY_KEYS = {
    "evidence",
    "identity_sha256",
    "kind",
    "statement_identity_sha256",
    "status",
}
PROPERTY_KIND_KEYS = {"code", "namespace_sha256", "variant"}
EVIDENCE_KEYS = {"binding", "contract_artifact", "variant"}
EVIDENCE_BINDING_KEYS = {
    "identity_sha256",
    "property_identity_sha256",
    "statement_identity_sha256",
}
CONTRACT_ARTIFACT_KEYS = {"bytes_sha256", "format_sha256"}
OBLIGATION_KEYS = {
    "identity_sha256",
    "property_identity_sha256",
    "required_status",
    "satisfaction",
    "statement_identity_sha256",
}
SATISFACTION_KEYS = {
    "evidence_identity_sha256",
    "property_identity_sha256",
    "statement_identity_sha256",
    "status",
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


def domain_digest(domain: str, parts: list[bytes]) -> str:
    hasher = hashlib.sha256()
    encoded_domain = domain.encode("ascii")
    hasher.update(len(encoded_domain).to_bytes(8, "big"))
    hasher.update(encoded_domain)
    for part in parts:
        hasher.update(len(part).to_bytes(8, "big"))
        hasher.update(part)
    return hasher.hexdigest()


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
        fail("fe2o3-contract context is empty or oversized")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("fe2o3-contract context must have one trailing newline")
    payload = raw[:-1]
    try:
        source = payload.decode("ascii")
        value = json.loads(source, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"fe2o3-contract context is not canonical ASCII JSON: {error}")
    canonical = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    if source != canonical:
        fail("fe2o3-contract context is not canonical JSON")
    return exact_keys(value, CONTEXT_KEYS, "fe2o3-contract context"), payload


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
        fail("fe2o3-contract report absolute path is not canonical")
    root = report_path
    for _ in report_relative.parts:
        root = root.parent
    if root.joinpath(*report_relative.parts) != report_path:
        fail("fe2o3-contract report absolute and relative paths disagree")
    reject_symlink_components(root, report_relative, "fe2o3-contract report")
    return root


def load_canonical_json(
    path: Path, limit: int, description: str, *, compact: bool
) -> tuple[dict[str, Any], bytes]:
    raw = read_bounded(path, limit, description)
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail(f"{description} must have one trailing newline")
    try:
        value = json.loads(raw, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"{description} is invalid JSON: {error}")
    if compact:
        expected = (
            json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
            + "\n"
        ).encode("ascii")
    else:
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
        fail("fe2o3-contract source roster is incomplete")
    for record, expected_id in zip(value, SOURCE_IDS, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source context {expected_id}")
        if record["id"] != expected_id:
            fail("fe2o3-contract source roster order or identity drifted")
        if record["repository"] != SOURCE_REPOSITORIES[expected_id]:
            fail(f"fe2o3-contract source repository drifted: {expected_id}")
        require_git_id(record["base_commit"], f"{expected_id} base commit")
        require_git_id(record["commit"], f"{expected_id} commit")
        require_git_id(record["tree"], f"{expected_id} tree")
        require_id(
            record["source_closure_artifact_id"], f"{expected_id} closure artifact"
        )
        require_sha256(record["source_closure_sha256"], f"{expected_id} closure")
    if value[0]["base_commit"] != requirements["m1_upstream_base_commit"]:
        fail("fe2o3-contract fe2o3 base identity drifted")
    if value[1]["base_commit"] != FERRIC_BASE_COMMIT:
        fail("fe2o3-contract Ferric base identity drifted")
    return value


def validate_tcb(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("fe2o3-contract TCB roster is incomplete")
    for record, expected_id in zip(value, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, f"TCB context {expected_id}")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("fe2o3-contract TCB order, identity, or kind drifted")
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
        fail("fe2o3-contract requires all M1 obligations and properties to remain Open")
    if obligation_class == "Roadmap":
        matches = [record for record in roadmaps if record["id"] == obligation_id]
        if len(matches) != 1:
            fail("fe2o3-contract binding names an unknown roadmap obligation")
        record = matches[0]
        return record, record["title"], record["assurance_properties"]
    if obligation_class == "Assurance":
        matches = [record for record in properties if record["name"] == obligation_id]
        if len(matches) != 1:
            fail("fe2o3-contract binding names an unknown assurance property")
        record = matches[0]
        return record, record["boundary"], [obligation_id]
    fail("fe2o3-contract obligation class drifted")


def assurance_declarations(
    requirements: dict[str, Any], assurance_property_ids: list[str]
) -> list[dict[str, Any]]:
    by_name = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    declarations: list[dict[str, Any]] = []
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


def expected_contract_body(
    binding: dict[str, Any],
    declarations: list[dict[str, Any]],
    requirements_sha256: str,
    source_roster_sha256: str,
    tcb_roster_sha256: str,
) -> dict[str, Any]:
    return {
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
        "target": CONTRACT_TARGET,
        "tcb_roster_sha256": tcb_roster_sha256,
    }


def expected_contract_set(
    binding: dict[str, Any], contract_body_sha256: str
) -> dict[str, Any]:
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
        identity_parts + [bytes.fromhex(contract_body_sha256)],
    )
    obligation_identity = domain_digest(
        "ferric.m1.fe2o3-contract.obligation-identity.v1", identity_parts
    )
    evidence_binding = {
        "identity_sha256": evidence_identity,
        "property_identity_sha256": property_identity,
        "statement_identity_sha256": binding["statement_sha256"],
    }
    satisfaction = {
        "evidence_identity_sha256": evidence_identity,
        "property_identity_sha256": property_identity,
        "statement_identity_sha256": binding["statement_sha256"],
        "status": "Contracted",
    }
    return {
        "correspondences": [],
        "format": CONTRACT_SET_FORMAT,
        "obligations": [
            {
                "identity_sha256": obligation_identity,
                "property_identity_sha256": property_identity,
                "required_status": "Contracted",
                "satisfaction": satisfaction,
                "statement_identity_sha256": binding["statement_sha256"],
            }
        ],
        "properties": [
            {
                "evidence": {
                    "binding": evidence_binding,
                    "contract_artifact": {
                        "bytes_sha256": contract_body_sha256,
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
    }


def positive_size(value: Any, description: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        fail(f"{description} is invalid")
    return value


def validate_contract_set_shape(value: dict[str, Any]) -> None:
    properties = value["properties"]
    obligations = value["obligations"]
    if not isinstance(properties, list) or len(properties) != 1:
        fail("fe2o3 ContractSet must declare exactly one property")
    if not isinstance(obligations, list) or len(obligations) != 1:
        fail("fe2o3 ContractSet must declare exactly one obligation")
    if value["trusted_computing_base"] != [] or value["correspondences"] != []:
        fail("fe2o3 contracted declaration has unexpected local authority")

    property_record = exact_keys(properties[0], PROPERTY_KEYS, "fe2o3 property")
    property_kind = exact_keys(
        property_record["kind"], PROPERTY_KIND_KEYS, "fe2o3 property kind"
    )
    evidence = exact_keys(
        property_record["evidence"], EVIDENCE_KEYS, "fe2o3 contracted evidence"
    )
    evidence_binding = exact_keys(
        evidence["binding"], EVIDENCE_BINDING_KEYS, "fe2o3 evidence binding"
    )
    contract_artifact = exact_keys(
        evidence["contract_artifact"],
        CONTRACT_ARTIFACT_KEYS,
        "fe2o3 contract artifact",
    )
    obligation = exact_keys(
        obligations[0], OBLIGATION_KEYS, "fe2o3 contract obligation"
    )
    satisfaction = exact_keys(
        obligation["satisfaction"],
        SATISFACTION_KEYS,
        "fe2o3 obligation satisfaction",
    )
    for description, record, keys in (
        (
            "fe2o3 property",
            property_record,
            ("identity_sha256", "statement_identity_sha256"),
        ),
        ("fe2o3 property kind", property_kind, ("namespace_sha256",)),
        (
            "fe2o3 evidence binding",
            evidence_binding,
            (
                "identity_sha256",
                "property_identity_sha256",
                "statement_identity_sha256",
            ),
        ),
        (
            "fe2o3 contract artifact",
            contract_artifact,
            ("bytes_sha256", "format_sha256"),
        ),
        (
            "fe2o3 contract obligation",
            obligation,
            (
                "identity_sha256",
                "property_identity_sha256",
                "statement_identity_sha256",
            ),
        ),
        (
            "fe2o3 obligation satisfaction",
            satisfaction,
            (
                "evidence_identity_sha256",
                "property_identity_sha256",
                "statement_identity_sha256",
            ),
        ),
    ):
        for key in keys:
            require_sha256(record[key], f"{description} {key}")


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("fe2o3-contract context index format drifted")
    repo = Path(__file__).resolve(strict=True).parents[3]
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements, requirements_raw = load_canonical_json(
        requirements_path,
        MAX_REQUIREMENTS_BYTES,
        "M1 requirements manifest",
        compact=False,
    )
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("fe2o3-contract context requirements identity drifted")

    artifact = exact_keys(context["artifact"], ARTIFACT_KEYS, "artifact context")
    artifact_id = require_id(artifact["id"], "artifact id")
    if artifact["kind"] != "ContractDocument":
        fail("fe2o3-contract artifact kind drifted")
    report_relative = safe_relative(artifact["path"], "report relative path")
    expected_report_path = f"artifacts/{artifact_id}.fe2o3-contract.json"
    if report_relative.as_posix() != expected_report_path:
        fail("fe2o3-contract report path is not canonical for its artifact id")
    report_sha256 = require_sha256(artifact["sha256"], "report SHA-256")
    positive_size(artifact["size_bytes"], "fe2o3-contract report size")
    if not isinstance(context["artifact_absolute_path"], str):
        fail("fe2o3-contract report absolute path is invalid")
    report_path = Path(context["artifact_absolute_path"])
    root = evidence_root(report_path, report_relative)
    report_value, report_bytes = load_canonical_json(
        report_path, MAX_REPORT_BYTES, "fe2o3-contract report", compact=False
    )
    report = exact_keys(report_value, REPORT_KEYS, "fe2o3-contract report")
    if (
        len(report_bytes) != artifact["size_bytes"]
        or digest_bytes(report_bytes) != report_sha256
    ):
        fail("fe2o3-contract report bytes do not match their context identity")

    contract_body_relative = safe_relative(
        report["contract_body_path"], "contract body relative path"
    )
    expected_contract_body_path = f"contracts/{artifact_id}.fe2o3-contract-body.json"
    if contract_body_relative.as_posix() != expected_contract_body_path:
        fail("fe2o3 contract body path is not canonical for its artifact id")
    contract_body_path = reject_symlink_components(
        root, contract_body_relative, "fe2o3 contract body"
    )
    contract_body, contract_body_bytes = load_canonical_json(
        contract_body_path,
        MAX_CONTRACT_BODY_BYTES,
        "fe2o3 contract body",
        compact=False,
    )
    exact_keys(contract_body, CONTRACT_BODY_KEYS, "fe2o3 contract body")
    contract_body_sha256 = require_sha256(
        report["contract_body_sha256"], "contract body SHA-256"
    )
    contract_body_size = positive_size(
        report["contract_body_size_bytes"], "fe2o3 contract body size"
    )
    if (
        len(contract_body_bytes) != contract_body_size
        or digest_bytes(contract_body_bytes) != contract_body_sha256
    ):
        fail("fe2o3 contract body identity mismatch")

    contract_set_relative = safe_relative(
        report["contract_set_path"], "contract-set relative path"
    )
    expected_contract_set_path = f"contract-sets/{artifact_id}.fe2o3-contract-set.json"
    if contract_set_relative.as_posix() != expected_contract_set_path:
        fail("fe2o3 ContractSet path is not canonical for its artifact id")
    contract_set_path = reject_symlink_components(
        root, contract_set_relative, "fe2o3 ContractSet declaration"
    )
    contract_set, contract_set_bytes = load_canonical_json(
        contract_set_path,
        MAX_CONTRACT_SET_BYTES,
        "fe2o3 ContractSet declaration",
        compact=False,
    )
    exact_keys(contract_set, CONTRACT_SET_KEYS, "fe2o3 ContractSet declaration")
    validate_contract_set_shape(contract_set)
    contract_set_sha256 = require_sha256(
        report["contract_set_sha256"], "ContractSet SHA-256"
    )
    contract_set_size = positive_size(
        report["contract_set_size_bytes"], "fe2o3 ContractSet size"
    )
    if (
        len(contract_set_bytes) != contract_set_size
        or digest_bytes(contract_set_bytes) != contract_set_sha256
    ):
        fail("fe2o3 ContractSet identity mismatch")

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
        or binding["evidence_kind"] != "fe2o3-contract"
        or binding["profile_id"] not in {"composition", "kernel", "runtime"}
        or binding["source_identity_id"] not in SOURCE_IDS
        or binding["tcb_ids"] != list(TCB_IDS)
    ):
        fail("fe2o3-contract binding context drifted")
    binding_payload = {
        key: value for key, value in binding.items() if key != "binding_sha256"
    }
    if binding["binding_sha256"] != canonical_digest(binding_payload):
        fail("fe2o3-contract binding identity mismatch")

    spec, statement, assurance_property_ids = requirements_spec(
        requirements, binding["obligation_class"], binding["obligation_id"]
    )
    profiles = {
        record["id"]: record["kinds"] for record in requirements["evidence_profiles"]
    }
    if (
        binding["profile_id"] not in spec["evidence_profiles"]
        or profiles.get(binding["profile_id"], []).count("fe2o3-contract") != 1
        or binding["path_id"] not in spec["path_obligations"]
        or binding["statement_sha256"] != digest_bytes(statement.encode("utf-8"))
    ):
        fail("fe2o3-contract obligation, profile, path, or statement drifted")

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
        fail("fe2o3-contract path resolution drifted")

    sources = validate_sources(context["sources"], requirements)
    tcb = validate_tcb(context["tcb"])
    source_by_id = {record["id"]: record for record in sources}
    expected_tcb_identities = {
        record["id"]: record["identity_sha256"] for record in tcb
    }
    declarations = assurance_declarations(requirements, assurance_property_ids)
    source_roster_sha256 = canonical_digest(sources)
    tcb_roster_sha256 = canonical_digest(tcb)
    expected_body = expected_contract_body(
        binding,
        declarations,
        requirements_sha256,
        source_roster_sha256,
        tcb_roster_sha256,
    )
    expected_set = expected_contract_set(binding, contract_body_sha256)
    for key in (
        "binding_sha256",
        "bound_source_identity_sha256",
        "path_resolution_sha256",
        "requirements_sha256",
        "source_roster_sha256",
        "statement_sha256",
        "tcb_roster_sha256",
    ):
        require_sha256(report[key], f"report {key}")
    if (
        report["format"] != REPORT_FORMAT
        or report["authority"] != AUTHORITY
        or report["nonclaim"] != NONCLAIM
        or report["evidence_kind"] != "fe2o3-contract"
        or report["contract_set_schema"] != CONTRACT_SET_SCHEMA
        or report["contract_set_source_path"] != CONTRACT_SET_SOURCE_PATH
        or report["contract_set_validation"] != CONTRACT_SET_VALIDATION
        or report["contract_target"] != CONTRACT_TARGET
        or report["profile_id"] != binding["profile_id"]
        or report["binding_sha256"] != binding["binding_sha256"]
        or report["obligation_class"] != binding["obligation_class"]
        or report["obligation_id"] != binding["obligation_id"]
        or report["obligation_state"] != "Open"
        or report["assurance_property_declarations"] != declarations
        or report["path_id"] != binding["path_id"]
        or report["path_resolution_sha256"] != canonical_digest(resolution)
        or report["requirements_sha256"] != requirements_sha256
        or report["source_identity_id"] != binding["source_identity_id"]
        or report["source_roster_sha256"] != source_roster_sha256
        or report["bound_source_identity_sha256"]
        != canonical_digest(source_by_id[binding["source_identity_id"]])
        or report["statement_sha256"] != binding["statement_sha256"]
        or report["tcb_identity_sha256s"] != expected_tcb_identities
        or report["tcb_roster_sha256"] != tcb_roster_sha256
        or contract_body != expected_body
        or contract_set != expected_set
    ):
        fail("fe2o3-contract report content or identity drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("fe2o3-contract validator protocol mismatch")
    context, context_payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(context_payload)}"
    )


if __name__ == "__main__":
    main()
