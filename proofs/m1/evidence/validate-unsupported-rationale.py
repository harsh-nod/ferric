#!/usr/bin/env python3
"""Validate a canonical, scope-limited M1 Unsupported rationale."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import stat
import sys
from typing import Any, NoReturn


PROTOCOL = "ferric.m1-validator.unsupported-rationale.v1"
OBLIGATION_CLASSES = ("Assurance",)
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
ARTIFACT_FORMAT = "FERRIC-M1-UNSUPPORTED-RATIONALE-V1"
AUTHORITY = "nonclaim-only"
NONCLAIM = (
    "This artifact grants no theorem, validation, artifact, load, launch, "
    "hardware, performance, or qualification authority."
)
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
MAX_CONTEXT_BYTES = 1_000_000
MAX_ARTIFACT_BYTES = 64_000
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
SOURCE_IDS = ("source.fe2o3", "source.ferric")
PATHS = {
    "identity-closure": (
        "RequiredFuture",
        "crates/ferric-build/src/identity_closure.rs",
    ),
    "m1-tcb": ("RequiredFuture", "docs/M1_TCB.md"),
    "speculation-proof": ("RequiredFuture", "proofs/m1/speculative_graph.rs"),
}

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
ARTIFACT_RECORD_KEYS = {"id", "kind", "path", "sha256", "size_bytes"}
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
RATIONALE_KEYS = {
    "authority",
    "binding_sha256",
    "excluded_claims",
    "format",
    "nonclaim",
    "obligation_class",
    "obligation_id",
    "path_id",
    "rationale",
    "reason_code",
    "required_closure_status",
    "requirements_sha256",
    "source_identity_id",
    "source_roster_sha256",
    "statement_sha256",
    "tcb_identity_sha256s",
    "tcb_roster_sha256",
}

RATIONALES: dict[str, dict[str, Any]] = {
    "distribution_preserved": {
        "reason_code": "outside-m1-deterministic-greedy-scope",
        "rationale": (
            "Stochastic sampling and stochastic speculative distribution "
            "preservation are outside the deterministic greedy M1 envelope."
        ),
        "excluded_claims": [
            "stochastic-sampling",
            "stochastic-speculative-distribution-preservation",
        ],
        "paths": {"m1-tcb", "speculation-proof"},
    },
    "machine_refined": {
        "reason_code": "unresolved-machine-correspondence",
        "rationale": (
            "The five independent translation validators required by the "
            "assurance policy do not exist; source proofs do not establish "
            "machine semantics."
        ),
        "excluded_claims": [
            "mir-to-structured-algorithm-correspondence",
            "algorithm-to-schedule-refinement",
            "gpu-subset-to-llvm-correspondence",
            "llvm-optimization-validation",
            "object-to-amdgpu-isa-correspondence",
        ],
        "paths": {"identity-closure", "m1-tcb"},
    },
    "multi_device_refined": {
        "reason_code": "outside-m1-single-device-scope",
        "rationale": (
            "M1 admits exactly one physical gfx942 device and makes no "
            "collective or multi-device refinement claim."
        ),
        "excluded_claims": [
            "collective-execution",
            "multi-device-execution",
            "multi-device-refinement",
        ],
        "paths": {"m1-tcb"},
    },
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


def digest_file(path: Path) -> str:
    try:
        hasher = hashlib.sha256()
        with path.open("rb") as source:
            for block in iter(lambda: source.read(64 * 1024), b""):
                hasher.update(block)
        return hasher.hexdigest()
    except OSError as error:
        fail(f"cannot hash artifact: {error}")


def canonical_digest(value: Any) -> str:
    payload = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    return digest_bytes(payload.encode("ascii"))


def require_sha256(value: Any, description: str) -> str:
    if not isinstance(value, str) or SHA256.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_git_id(value: Any, description: str) -> str:
    if not isinstance(value, str) or GIT_ID.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def reject_duplicate_key(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def load_context() -> tuple[dict[str, Any], bytes]:
    raw = sys.stdin.buffer.read(MAX_CONTEXT_BYTES + 1)
    if not raw or len(raw) > MAX_CONTEXT_BYTES:
        fail("validator context is empty or oversized")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("validator context must have one trailing newline")
    payload = raw[:-1]
    try:
        source = payload.decode("ascii")
        value = json.loads(source, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"validator context is not canonical ASCII JSON: {error}")
    canonical = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    if source != canonical:
        fail("validator context is not canonical JSON")
    return exact_keys(value, CONTEXT_KEYS, "validator context"), payload


def regular_file(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"{description} must be a regular non-symlink file")


def load_artifact(path: Path) -> dict[str, Any]:
    regular_file(path, "unsupported rationale artifact")
    try:
        size = path.stat().st_size
        if size <= 0 or size > MAX_ARTIFACT_BYTES:
            fail("unsupported rationale artifact size is outside the admitted bound")
        raw = path.read_bytes()
    except OSError as error:
        fail(f"cannot read unsupported rationale artifact: {error}")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("unsupported rationale artifact must have one trailing newline")
    try:
        value = json.loads(raw, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"unsupported rationale artifact is invalid JSON: {error}")
    expected = (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")
    if raw != expected:
        fail("unsupported rationale artifact is not canonical JSON")
    return exact_keys(value, RATIONALE_KEYS, "unsupported rationale artifact")


def validate_sources(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail("source roster is incomplete")
    for expected_id, record in zip(SOURCE_IDS, value, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source record {expected_id}")
        if record["id"] != expected_id:
            fail("source roster order or identity drifted")
        expected_repository = expected_id.removeprefix("source.")
        if record["repository"] != expected_repository:
            fail(f"source repository drifted: {expected_id}")
        require_git_id(record["base_commit"], f"source base commit: {expected_id}")
        require_git_id(record["commit"], f"source commit: {expected_id}")
        require_git_id(record["tree"], f"source tree: {expected_id}")
        require_sha256(
            record["source_closure_sha256"], f"source closure: {expected_id}"
        )
    return value


def validate_tcb(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("TCB roster is incomplete")
    expected_kinds = ("Compiler", "Hardware", "Runtime")
    for expected_id, expected_kind, record in zip(
        TCB_IDS, expected_kinds, value, strict=True
    ):
        exact_keys(record, TCB_KEYS, f"TCB record {expected_id}")
        if record["id"] != expected_id or record["kind"] != expected_kind:
            fail("TCB roster order, identity, or kind drifted")
        require_sha256(record["identity_sha256"], f"TCB identity: {expected_id}")
    return value


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("evidence-index format drifted")
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    sources = validate_sources(context["sources"])
    tcb = validate_tcb(context["tcb"])

    artifact_record = exact_keys(context["artifact"], ARTIFACT_RECORD_KEYS, "artifact")
    if artifact_record["kind"] != "UnsupportedRationale":
        fail("artifact kind cannot carry an unsupported rationale")
    if (
        not isinstance(artifact_record["size_bytes"], int)
        or artifact_record["size_bytes"] <= 0
    ):
        fail("artifact size is invalid")
    expected_artifact_sha256 = require_sha256(
        artifact_record["sha256"], "artifact SHA-256"
    )
    artifact_path_value = context["artifact_absolute_path"]
    if (
        not isinstance(artifact_path_value, str)
        or not Path(artifact_path_value).is_absolute()
    ):
        fail("artifact path must be absolute")
    artifact_path = Path(artifact_path_value)
    regular_file(artifact_path, "unsupported rationale artifact")
    if (
        artifact_path.stat().st_size != artifact_record["size_bytes"]
        or digest_file(artifact_path) != expected_artifact_sha256
    ):
        fail("unsupported rationale artifact identity mismatch")

    binding = exact_keys(context["binding"], BINDING_KEYS, "evidence binding")
    binding_payload = {
        key: value for key, value in binding.items() if key != "binding_sha256"
    }
    if binding["binding_sha256"] != canonical_digest(binding_payload):
        fail("evidence binding identity mismatch")
    obligation_id = binding["obligation_id"]
    expected = RATIONALES.get(obligation_id)
    if expected is None:
        fail("unsupported rationale names an unsupported property outside M1")
    statement_sha256 = digest_bytes(expected["rationale"].encode("utf-8"))
    if (
        binding["artifact_id"] != artifact_record["id"]
        or binding["obligation_class"] != "Assurance"
        or binding["evidence_kind"] != "unsupported-rationale"
        or binding["profile_id"] != "nonclaim"
        or binding["source_identity_id"] != "source.ferric"
        or binding["path_id"] not in expected["paths"]
        or binding["statement_sha256"] != statement_sha256
        or binding["tcb_ids"] != list(TCB_IDS)
    ):
        fail("unsupported rationale binding drifted")
    if context["subject"] != f"binding:{binding['id']}":
        fail("validator subject does not bind the evidence record")

    path = exact_keys(context["path_resolution"], PATH_KEYS, "path resolution")
    expected_path = PATHS[binding["path_id"]]
    if (
        path["id"] != binding["path_id"]
        or path["repository"] != "ferric"
        or path["source_identity_id"] != "source.ferric"
        or (path["availability"], path["path"]) != expected_path
    ):
        fail("unsupported rationale path resolution drifted")

    rationale = load_artifact(artifact_path)
    expected_tcb_identities = {
        record["id"]: record["identity_sha256"] for record in tcb
    }
    if (
        rationale["format"] != ARTIFACT_FORMAT
        or rationale["authority"] != AUTHORITY
        or rationale["nonclaim"] != NONCLAIM
        or rationale["obligation_class"] != "Assurance"
        or rationale["obligation_id"] != obligation_id
        or rationale["required_closure_status"] != "Unsupported"
        or rationale["requirements_sha256"] != requirements_sha256
        or rationale["binding_sha256"] != binding["binding_sha256"]
        or rationale["source_identity_id"] != binding["source_identity_id"]
        or rationale["path_id"] != binding["path_id"]
        or rationale["statement_sha256"] != statement_sha256
        or rationale["rationale"] != expected["rationale"]
        or rationale["reason_code"] != expected["reason_code"]
        or rationale["excluded_claims"] != expected["excluded_claims"]
        or rationale["source_roster_sha256"] != canonical_digest(sources)
        or rationale["tcb_roster_sha256"] != canonical_digest(tcb)
        or rationale["tcb_identity_sha256s"] != expected_tcb_identities
    ):
        fail("unsupported rationale content or identity drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("unsupported-rationale validator protocol mismatch")
    context, payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(payload)}"
    )


if __name__ == "__main__":
    main()
