#!/usr/bin/env python3
"""Validate one canonical, source-bound M1 trusted-computing-base report."""

from __future__ import annotations

import ast
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, BinaryIO, NoReturn


PROTOCOL = "ferric.m1-validator.tcb-report.v1"
OBLIGATION_CLASSES = ()
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REPORT_FORMAT = "FERRIC-M1-TCB-REPORT-V1"
REPORT_TARGET = "gfx942:xnack-"
AUTHORITY = "trusted-boundary-declaration-only"
NONCLAIM = (
    "This report authenticates the declared M1 trusted boundary only. It does "
    "not establish component presence, version provenance, compiler or runtime "
    "correctness, hardware behavior, theorem truth, machine refinement, load, "
    "launch, performance, or qualification authority and closes no obligation."
)
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.:-]+\Z")
SAFE_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
MAX_CONTEXT_BYTES = 1_000_000
MAX_REPORT_BYTES = 512_000
MAX_REQUIREMENTS_BYTES = 1_000_000
MAX_VALIDATOR_BYTES = 2_000_000
MAX_CHECKER_BYTES = 2_000_000

TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {"source.fe2o3": "fe2o3", "source.ferric": "ferric"}
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
EVIDENCE_KIND_BINDING_CLASSES = (
    ("artifact-identity", ("Assurance", "Roadmap")),
    ("canonical-structure-check", ("Assurance", "Roadmap")),
    ("external-contract", ("Assurance", "Roadmap")),
    ("fe2o3-contract", ("Assurance", "Roadmap")),
    ("hardware-test", ("Assurance", "Roadmap")),
    ("independent-validator", ("Assurance", "Roadmap")),
    ("negative-mutation", ("Assurance",)),
    ("performance-gate", ("Assurance", "Roadmap")),
    ("tcb-report", ()),
    ("unsupported-rationale", ("Assurance",)),
    ("verus-theorem", ("Assurance",)),
)

# This is the complete checker-owned version-1 validator TCB vocabulary. The
# literal checker registry supplies each reviewed source pin. Parsing that
# literal rather than importing executable checker code keeps this validator's
# TCB bounded and lets independently reviewed validator slices compose.
VALIDATOR_SPECS = (
    (
        "artifact-identity",
        "proofs/m1/evidence/validate-artifact-identity.py",
        "ferric.m1-validator.artifact-identity.v1",
    ),
    (
        "canonical-structure-check",
        "proofs/m1/evidence/validate-canonical-structure.py",
        "ferric.m1-validator.canonical-structure.v1",
    ),
    (
        "external-contract",
        "proofs/m1/evidence/validate-external-contract.py",
        "ferric.m1-validator.external-contract.v1",
    ),
    (
        "fe2o3-contract",
        "proofs/m1/evidence/validate-fe2o3-contract.py",
        "ferric.m1-validator.fe2o3-contract.v1",
    ),
    (
        "hardware-test",
        "proofs/m1/evidence/validate-hardware-transcript.py",
        "ferric.m1-validator.hardware-transcript.v1",
    ),
    (
        "independent-validator",
        "proofs/m1/evidence/validate-independent-validator.py",
        "ferric.m1-validator.independent-validator.v1",
    ),
    (
        "negative-mutation",
        "proofs/m1/evidence/validate-negative-mutation.py",
        "ferric.m1-validator.negative-mutation.v1",
    ),
    (
        "performance-gate",
        "proofs/m1/evidence/validate-performance-report.py",
        "ferric.m1-validator.performance-report.v1",
    ),
    (
        "qualification-receipt",
        "proofs/m1/evidence/validate-qualification-receipt.py",
        "ferric.m1-validator.qualification-receipt.v1",
    ),
    (
        "tcb-report",
        "proofs/m1/evidence/validate-tcb-report.py",
        PROTOCOL,
    ),
    (
        "unsupported-rationale",
        "proofs/m1/evidence/validate-unsupported-rationale.py",
        "ferric.m1-validator.unsupported-rationale.v1",
    ),
    (
        "verus-theorem",
        "proofs/m1/evidence/validate-verus-theorem.py",
        "ferric.m1-validator.verus-theorem.v1",
    ),
)

CONTEXT_KEYS = {
    "artifact",
    "artifact_absolute_path",
    "format",
    "requirements_sha256",
    "sources",
    "subject",
    "tcb",
    "tcb_record",
}
ARTIFACT_KEYS = {"id", "kind", "path", "sha256", "size_bytes"}
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
COMPONENT_KEYS = {"authority", "id", "identity_sha256", "kind", "status", "version"}
OBLIGATION_KEYS = {
    "class",
    "id",
    "path_ids",
    "profile_ids",
    "statement_sha256",
    "status",
}
PATH_KEYS = {
    "availability",
    "id",
    "path",
    "repository",
    "source_identity_id",
    "status",
}
PROFILE_KEYS = {"evidence_kinds", "id"}
TCB_STRUCTURE_KEYS = {"artifact_id", "id", "kind"}
VALIDATOR_KEYS = {"availability", "evidence_kind", "path", "protocol", "source_sha256"}
REPORT_KEYS = {
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
REQUIREMENTS_KEYS = {
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
        fail("TCB-report context is empty or oversized")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("TCB-report context must have one trailing newline")
    payload = raw[:-1]
    try:
        source = payload.decode("ascii")
        value = json.loads(source, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"TCB-report context is not canonical ASCII JSON: {error}")
    canonical = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    if source != canonical:
        fail("TCB-report context is not canonical JSON")
    return exact_keys(value, CONTEXT_KEYS, "TCB-report context"), payload


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
        fail("TCB report absolute path is not canonical")
    root = report_path
    for _ in report_relative.parts:
        root = root.parent
    if root.joinpath(*report_relative.parts) != report_path:
        fail("TCB report absolute and relative paths disagree")
    reject_symlink_components(root, report_relative, "TCB report")
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


def validate_requirements(requirements: dict[str, Any]) -> None:
    exact_keys(requirements, REQUIREMENTS_KEYS, "M1 requirements manifest")
    if (
        requirements["format"] != "ferric.m1-requirements.v1"
        or requirements["milestone"] != "M1"
        or tuple(requirements["evidence_kinds"]) != EVIDENCE_KINDS
        or len(requirements["roadmap_requirements"]) != 33
        or len(requirements["assurance_properties"]) != 17
        or len(requirements["path_obligations"]) != 39
        or len(requirements["evidence_profiles"]) != len(PROFILE_IDS)
    ):
        fail("M1 requirements cardinality, format, or evidence vocabulary drifted")
    for key in (
        "m0_contracts_commit",
        "m1_upstream_base_commit",
        "m1_upstream_base_tree",
    ):
        require_git_id(requirements[key], f"requirements {key}")
    applicability = requirements["evidence_kind_binding_classes"]
    if not isinstance(applicability, list):
        fail("M1 evidence-kind binding-class roster is invalid")
    observed_applicability: list[tuple[str, tuple[str, ...]]] = []
    for record in applicability:
        if not isinstance(record, dict) or set(record) != {"classes", "kind"}:
            fail("M1 evidence-kind binding-class record drifted")
        kind = record["kind"]
        classes = record["classes"]
        if (
            not isinstance(kind, str)
            or not isinstance(classes, list)
            or not all(isinstance(item, str) for item in classes)
        ):
            fail("M1 evidence-kind binding-class roster is malformed")
        observed_applicability.append((kind, tuple(classes)))
    if tuple(observed_applicability) != EVIDENCE_KIND_BINDING_CLASSES:
        fail("M1 evidence-kind binding-class roster drifted")
    if any(
        record.get("obligation_state") != "Open"
        for group in (
            requirements["roadmap_requirements"],
            requirements["assurance_properties"],
            requirements["path_obligations"],
        )
        for record in group
    ):
        fail("TCB reporting requires every M1 obligation to remain Open")
    profile_ids = [record.get("id") for record in requirements["evidence_profiles"]]
    if tuple(profile_ids) != PROFILE_IDS or len(profile_ids) != len(set(profile_ids)):
        fail("M1 evidence profile roster drifted")
    for record in requirements["evidence_profiles"]:
        kinds = record.get("kinds")
        if (
            not isinstance(kinds, list)
            or kinds.count("tcb-report") != 1
            or len(kinds) != len(set(kinds))
            or any(kind not in EVIDENCE_KINDS for kind in kinds)
        ):
            fail(f"M1 profile does not require one TCB report: {record.get('id')}")


def expected_obligations(requirements: dict[str, Any]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()
    for record in requirements["roadmap_requirements"]:
        item = {
            "class": "Roadmap",
            "id": record["id"],
            "path_ids": record["path_obligations"],
            "profile_ids": record["evidence_profiles"],
            "statement_sha256": digest_bytes(record["title"].encode("utf-8")),
            "status": record["obligation_state"],
        }
        exact_keys(item, OBLIGATION_KEYS, "projected roadmap obligation")
        result.append(item)
    for record in requirements["assurance_properties"]:
        item = {
            "class": "Assurance",
            "id": record["name"],
            "path_ids": record["path_obligations"],
            "profile_ids": record["evidence_profiles"],
            "statement_sha256": digest_bytes(record["boundary"].encode("utf-8")),
            "status": record["obligation_state"],
        }
        exact_keys(item, OBLIGATION_KEYS, "projected assurance obligation")
        result.append(item)
    for item in result:
        key = (item["class"], item["id"])
        if key in seen:
            fail("M1 requirements contain a duplicate obligation")
        seen.add(key)
    return result


def expected_paths(requirements: dict[str, Any]) -> list[dict[str, Any]]:
    result = [
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
    if len({record["id"] for record in result}) != len(result):
        fail("M1 requirements contain a duplicate path obligation")
    for record in result:
        exact_keys(record, PATH_KEYS, "projected M1 path")
        safe_relative(record["path"], f"path obligation {record['id']}")
        if record["repository"] not in SOURCE_REPOSITORIES.values():
            fail(f"M1 path repository drifted: {record['id']}")
    return result


def expected_profiles(requirements: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {"evidence_kinds": record["kinds"], "id": record["id"]}
        for record in requirements["evidence_profiles"]
    ]


def validate_sources(value: Any, requirements: dict[str, Any]) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail("TCB source roster is incomplete")
    expected_bases = {
        "source.fe2o3": requirements["m1_upstream_base_commit"],
        "source.ferric": FERRIC_BASE_COMMIT,
    }
    identities: set[str] = set()
    closures: set[str] = set()
    for record, expected_id in zip(value, SOURCE_IDS, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source context {expected_id}")
        if (
            record["id"] != expected_id
            or record["repository"] != SOURCE_REPOSITORIES[expected_id]
            or record["base_commit"] != expected_bases[expected_id]
        ):
            fail("TCB source order, repository, or base identity drifted")
        require_git_id(record["base_commit"], f"{expected_id} base commit")
        require_git_id(record["commit"], f"{expected_id} commit")
        require_git_id(record["tree"], f"{expected_id} tree")
        require_id(
            record["source_closure_artifact_id"], f"{expected_id} closure artifact"
        )
        closure = require_sha256(
            record["source_closure_sha256"], f"{expected_id} source closure"
        )
        if record["source_closure_artifact_id"] in identities or closure in closures:
            fail("TCB source roster contains a duplicate artifact or closure identity")
        identities.add(record["source_closure_artifact_id"])
        closures.add(closure)
    return value


def validate_tcb(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("TCB context roster is incomplete")
    artifact_ids: set[str] = set()
    identities: set[str] = set()
    for record, expected_id in zip(value, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, f"TCB context {expected_id}")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("TCB context order, identity, or kind drifted")
        artifact_id = require_id(record["artifact_id"], f"{expected_id} artifact")
        identity = require_sha256(record["identity_sha256"], f"{expected_id} identity")
        if artifact_id in artifact_ids or identity in identities:
            fail("TCB context contains a duplicate artifact or identity")
        artifact_ids.add(artifact_id)
        identities.add(identity)
    return value


def file_digest(path: Path, description: str) -> str:
    return digest_bytes(read_bounded(path, MAX_VALIDATOR_BYTES, description))


def checker_registry(repo: Path) -> dict[str, tuple[str, str, str | None]]:
    checker_path = repo / "proofs/check-m1-evidence-index.py"
    raw = read_bounded(checker_path, MAX_CHECKER_BYTES, "M1 evidence-index checker")
    try:
        tree = ast.parse(raw.decode("ascii"), filename=str(checker_path))
    except (UnicodeDecodeError, SyntaxError) as error:
        fail(f"cannot parse checker-owned validator registry: {error}")
    assignments = [
        node.value
        for node in tree.body
        if isinstance(node, (ast.Assign, ast.AnnAssign))
        and (
            (
                isinstance(node, ast.Assign)
                and any(
                    isinstance(target, ast.Name) and target.id == "TRUSTED_VALIDATORS"
                    for target in node.targets
                )
            )
            or (
                isinstance(node, ast.AnnAssign)
                and isinstance(node.target, ast.Name)
                and node.target.id == "TRUSTED_VALIDATORS"
            )
        )
    ]
    if len(assignments) != 1:
        fail("checker must contain exactly one literal trusted-validator registry")
    try:
        value = ast.literal_eval(assignments[0])
    except (ValueError, TypeError, SyntaxError) as error:
        fail(f"checker trusted-validator registry is not literal data: {error}")
    if not isinstance(value, dict):
        fail("checker trusted-validator registry is not an object")
    expected_ids = tuple(spec[0] for spec in VALIDATOR_SPECS)
    if tuple(value) != expected_ids:
        fail("checker trusted-validator registry is incomplete or reordered")
    result: dict[str, tuple[str, str, str | None]] = {}
    for evidence_kind, record in value.items():
        if (
            not isinstance(evidence_kind, str)
            or not isinstance(record, tuple)
            or len(record) != 3
            or not isinstance(record[0], str)
            or not isinstance(record[1], str)
            or (record[2] is not None and not isinstance(record[2], str))
        ):
            fail(f"checker validator registry entry drifted: {evidence_kind!r}")
        result[evidence_kind] = record
    return result


def expected_validators(repo: Path) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    registry = checker_registry(repo)
    for evidence_kind, relative_name, protocol in VALIDATOR_SPECS:
        registered_path, registered_protocol, registered_sha256 = registry[
            evidence_kind
        ]
        if registered_path != relative_name or registered_protocol != protocol:
            fail(f"checker-owned validator path or protocol drifted: {evidence_kind}")
        relative = safe_relative(relative_name, f"{evidence_kind} validator path")
        path = repo.joinpath(*relative.parts)
        if registered_sha256 is not None:
            source_sha256 = require_sha256(
                registered_sha256, f"checker {evidence_kind} validator source"
            )
            if file_digest(path, f"trusted {evidence_kind} validator") != source_sha256:
                fail(
                    f"checker-owned validator source identity drifted: {evidence_kind}"
                )
            availability = "ExistingFoundation"
        else:
            try:
                path.lstat()
            except FileNotFoundError:
                pass
            except OSError as error:
                fail(
                    f"cannot inspect RequiredFuture validator {relative_name}: {error}"
                )
            else:
                fail(f"RequiredFuture validator unexpectedly exists: {relative_name}")
            source_sha256 = None
            availability = "RequiredFuture"
        result.append(
            {
                "availability": availability,
                "evidence_kind": evidence_kind,
                "path": relative_name,
                "protocol": protocol,
                "source_sha256": source_sha256,
            }
        )
    return result


def declared_component(
    identifier: str,
    kind: str,
    version: str,
    status: str,
    authority: str,
    identity_payload: Any,
) -> dict[str, Any]:
    return {
        "authority": authority,
        "id": identifier,
        "identity_sha256": canonical_digest(identity_payload),
        "kind": kind,
        "status": status,
        "version": version,
    }


def expected_components(
    repo: Path, sources: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    source_by_id = {record["id"]: record for record in sources}
    rust_toolchain = file_digest(repo / "rust-toolchain.toml", "Rust toolchain pin")
    verus_version_raw = read_bounded(
        repo / "proofs/verus/VERUS_VERSION", 4096, "Verus version pin"
    )
    try:
        verus_version = verus_version_raw.decode("ascii").removesuffix("\n")
    except UnicodeDecodeError as error:
        fail(f"Verus version pin is not ASCII: {error}")
    if not verus_version or "\n" in verus_version:
        fail("Verus version pin is not one canonical line")
    verus_closure = file_digest(
        repo / "proofs/verus/VERUS_CLOSURE_MANIFEST", "Verus closure manifest"
    )
    records = [
        declared_component(
            "compiler.amdgpu-linker",
            "Compiler",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["compiler.amdgpu-linker", "qualification-bound-external", REPORT_TARGET],
        ),
        declared_component(
            "compiler.llvm-amdgpu",
            "Compiler",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["compiler.llvm-amdgpu", "qualification-bound-external", REPORT_TARGET],
        ),
        declared_component(
            "compiler.rust",
            "Compiler",
            "1.97.1",
            "Pinned",
            "source-configuration-only",
            ["compiler.rust", "1.97.1", rust_toolchain],
        ),
        declared_component(
            "compiler.verus",
            "Compiler",
            verus_version,
            "Pinned",
            "proof-tool-source-closure",
            ["compiler.verus", verus_version, verus_closure],
        ),
        declared_component(
            "hardware.gfx942",
            "Hardware",
            REPORT_TARGET,
            "Contracted",
            "single-device-target-only",
            ["hardware.gfx942", REPORT_TARGET, "one-physical-device"],
        ),
        declared_component(
            "runtime.amdgpu-firmware",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.amdgpu-firmware", "qualification-bound-external", REPORT_TARGET],
        ),
        declared_component(
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
        declared_component(
            "runtime.fe2o3",
            "Runtime",
            source_by_id["source.fe2o3"]["commit"],
            "SourceBound",
            "exact-source-identity",
            source_by_id["source.fe2o3"],
        ),
        declared_component(
            "runtime.ferric",
            "Runtime",
            source_by_id["source.ferric"]["commit"],
            "SourceBound",
            "exact-source-identity",
            source_by_id["source.ferric"],
        ),
        declared_component(
            "runtime.hsa",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.hsa", "qualification-bound-external", REPORT_TARGET],
        ),
        declared_component(
            "runtime.posix-host",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "os-filesystem-process-supervision",
            ["runtime.posix-host", "qualification-bound-external"],
        ),
        declared_component(
            "runtime.python",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "validator-interpreter-and-stdlib",
            ["runtime.python", "qualification-bound-external"],
        ),
    ]
    if [record["id"] for record in records] != sorted(
        record["id"] for record in records
    ):
        fail("internal TCB component roster is not canonical")
    if len({record["identity_sha256"] for record in records}) != len(records):
        fail("internal TCB component identities are not unique")
    return records


def validate_report_rosters(report: dict[str, Any]) -> None:
    for record in report["component_roster"]:
        exact_keys(record, COMPONENT_KEYS, "TCB component")
        require_id(record["id"], "TCB component id")
        require_sha256(record["identity_sha256"], "TCB component identity")
    for record in report["obligation_roster"]:
        exact_keys(record, OBLIGATION_KEYS, "TCB obligation")
        require_sha256(record["statement_sha256"], "TCB obligation statement")
    for record in report["path_roster"]:
        exact_keys(record, PATH_KEYS, "TCB path")
    for record in report["profile_roster"]:
        exact_keys(record, PROFILE_KEYS, "TCB profile")
    for record in report["source_roster"]:
        exact_keys(record, SOURCE_KEYS, "TCB report source")
    for record in report["tcb_structure_roster"]:
        exact_keys(record, TCB_STRUCTURE_KEYS, "TCB structure")
    for record in report["validator_roster"]:
        exact_keys(record, VALIDATOR_KEYS, "TCB validator")
        if record["source_sha256"] is not None:
            require_sha256(record["source_sha256"], "TCB validator source")


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("TCB-report context index format drifted")
    validator_path = Path(__file__).absolute()
    repo = validator_path.parents[3]
    if validator_path.resolve(strict=True) != validator_path:
        fail("TCB validator source path contains a symlink")

    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements, requirements_raw = load_canonical_json(
        requirements_path,
        MAX_REQUIREMENTS_BYTES,
        "M1 requirements manifest",
        compact=False,
    )
    validate_requirements(requirements)
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("TCB-report context requirements identity drifted")

    artifact = exact_keys(context["artifact"], ARTIFACT_KEYS, "artifact context")
    artifact_id = require_id(artifact["id"], "artifact id")
    if artifact["kind"] != "TcbReport":
        fail("TCB-report artifact kind drifted")
    report_relative = safe_relative(artifact["path"], "report relative path")
    expected_report_path = f"artifacts/{artifact_id}.tcb-report.json"
    if report_relative.as_posix() != expected_report_path:
        fail("TCB report path is not canonical for its artifact id")
    report_sha256 = require_sha256(artifact["sha256"], "TCB report SHA-256")
    if (
        not isinstance(artifact["size_bytes"], int)
        or isinstance(artifact["size_bytes"], bool)
        or artifact["size_bytes"] <= 0
    ):
        fail("TCB report size is invalid")
    if not isinstance(context["artifact_absolute_path"], str):
        fail("TCB report absolute path is invalid")
    report_path = Path(context["artifact_absolute_path"])
    root = evidence_root(report_path, report_relative)
    report_value, report_bytes = load_canonical_json(
        report_path, MAX_REPORT_BYTES, "TCB report", compact=False
    )
    reject_symlink_components(root, report_relative, "TCB report after read")
    report = exact_keys(report_value, REPORT_KEYS, "TCB report")
    if (
        len(report_bytes) != artifact["size_bytes"]
        or digest_bytes(report_bytes) != report_sha256
    ):
        fail("TCB report bytes do not match their context identity")

    sources = validate_sources(context["sources"], requirements)
    tcb = validate_tcb(context["tcb"])
    tcb_record = exact_keys(context["tcb_record"], TCB_KEYS, "subject TCB context")
    subject_id = tcb_record["id"]
    if (
        subject_id not in TCB_IDS
        or tcb_record != tcb[TCB_IDS.index(subject_id)]
        or context["subject"] != f"tcb:{subject_id}"
        or tcb_record["artifact_id"] != artifact_id
        or tcb_record["identity_sha256"] != report_sha256
        or tcb_record["kind"] != TCB_KINDS[subject_id]
    ):
        fail("TCB report subject, kind, artifact, or identity drifted")

    expected_tcb_structure = [
        {
            "artifact_id": record["artifact_id"],
            "id": record["id"],
            "kind": record["kind"],
        }
        for record in tcb
    ]
    expected_obligation_roster = expected_obligations(requirements)
    expected_path_roster = expected_paths(requirements)
    expected_profile_roster = expected_profiles(requirements)
    expected_component_roster = expected_components(repo, sources)
    expected_validator_roster = expected_validators(repo)
    validate_report_rosters(report)
    if (
        report["format"] != REPORT_FORMAT
        or report["milestone"] != "M1"
        or report["authority"] != AUTHORITY
        or report["nonclaim"] != NONCLAIM
        or report["evidence_kind"] != "tcb-report"
        or report["obligation_state"] != "Open"
        or report["target"] != REPORT_TARGET
        or report["requirements_sha256"] != requirements_sha256
        or report["subject_tcb_id"] != subject_id
        or report["subject_tcb_kind"] != tcb_record["kind"]
        or report["source_roster"] != sources
        or report["tcb_structure_roster"] != expected_tcb_structure
        or report["obligation_roster"] != expected_obligation_roster
        or report["path_roster"] != expected_path_roster
        or report["profile_roster"] != expected_profile_roster
        or report["component_roster"] != expected_component_roster
        or report["validator_roster"] != expected_validator_roster
    ):
        fail("TCB report content, completeness, order, version, or authority drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("TCB-report validator protocol mismatch")
    context, context_payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(context_payload)}"
    )


if __name__ == "__main__":
    main()
