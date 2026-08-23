#!/usr/bin/env python3
"""Validate one canonical M1 same-source negative-mutation run artifact."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from typing import Any, NoReturn


PROTOCOL = "ferric.m1-validator.negative-mutation.v1"
OBLIGATION_CLASSES = ("Assurance",)
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REGISTRY_FORMAT = "format=FERRIC-M1-NEGATIVE-FOUNDATIONS-V1"
RUN_FORMAT = "FERRIC-M1-NEGATIVE-RUN-V1"
RESULT_FORMAT = "FERRIC-M1-NEGATIVE-RESULT-V1"
MUTATION_FORMAT = "FERRIC-M1-NEGATIVE-MUTATION-V1"
COMPILE_FORMAT = "FERRIC-M1-NEGATIVE-COMPILE-V1"
VERUS_FORMAT = "FERRIC-M1-NEGATIVE-VERUS-V1"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
DECIMAL = re.compile(r"0|[1-9][0-9]*\Z")
MAX_CONTEXT_BYTES = 1_000_000
MAX_CONTROL_BYTES = 1_000_000
MAX_TRANSCRIPT_BYTES = 20_000_000
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {"source.fe2o3": "fe2o3", "source.ferric": "ferric"}
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
SOURCE_EXCLUDED_DIRECTORIES = {".git", ".ruff_cache", "__pycache__", "target"}
SOURCE_EXCLUDED_SUFFIXES = {".pyc", ".receipt"}

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
PATH_KEYS = {"availability", "id", "path", "repository", "source_identity_id"}

RUN_KEYS = (
    "FORMAT",
    "FERRIC_COMMIT",
    "FERRIC_TREE",
    "FERRIC_SOURCE_CLOSURE_SHA256",
    "VERUS_VERSION",
    "VERUS_SHA256",
    "VERUS_CLOSURE_MANIFEST_SHA256",
    "VERUS_CLOSURE_SHA256",
    "REGISTRY_SHA256",
    "RUNNER_SHA256",
    "AUTHORITY",
    "NONCLAIM",
)
RESULT_KEYS = (
    "FORMAT",
    "MUTATION",
    "RUN_IDENTITY_SHA256",
    "ACTIVE_FOUNDATIONS_SHA256",
    "SELECTED_FOUNDATIONS_SHA256",
    "VERUS_CLOSURE_TRANSCRIPT_SHA256",
    "MUTATION_RECORD",
    "MUTATION_RECORD_SHA256",
    "MUTATION_RECORD_SIZE",
    "COMPILE_TRANSCRIPT",
    "COMPILE_TRANSCRIPT_SHA256",
    "COMPILE_TRANSCRIPT_SIZE",
    "COMPILE_EXIT_STATUS",
    "VERUS_TRANSCRIPT",
    "VERUS_TRANSCRIPT_SHA256",
    "VERUS_TRANSCRIPT_SIZE",
    "VERUS_EXIT_STATUS",
    "RESULT",
)
MUTATION_KEYS = (
    "FORMAT",
    "MUTATED_SOURCE",
    "MUTATION",
    "CLAUSE",
    "ANCHOR_SHA256",
    "MUTATOR_SHA256",
    "ORIGINAL_SOURCE_SHA256",
    "MUTATED_SOURCE_SHA256",
    "FOUNDATION",
    "OPEN_ASSURANCE_PROPERTY",
    "OPEN_PATH_OBLIGATION",
    "VERUS_PACKAGE",
    "VERUS_MODULE",
    "VERUS_FUNCTION",
    "EXPECTED_FAILURE_MARKER",
    "CARGO_CHECK",
)


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
        return digest_bytes(path.read_bytes())
    except OSError as error:
        fail(f"cannot hash {path}: {error}")


def require_sha256(value: Any, description: str) -> str:
    if not isinstance(value, str) or SHA256.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_git_id(value: Any, description: str) -> str:
    if not isinstance(value, str) or GIT_ID.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_name(value: Any, description: str) -> str:
    if not isinstance(value, str) or SAFE_NAME.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_decimal(value: str, description: str) -> int:
    if DECIMAL.fullmatch(value) is None:
        fail(f"malformed {description}")
    return int(value)


def regular_file(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"{description} must be a regular non-symlink file")


def read_bounded(path: Path, limit: int, description: str) -> bytes:
    regular_file(path, description)
    try:
        if path.stat().st_size <= 0 or path.stat().st_size > limit:
            fail(f"{description} size is outside the admitted bound")
        return path.read_bytes()
    except OSError as error:
        fail(f"cannot read {description}: {error}")


def reject_duplicate_key(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def load_stdin_context() -> tuple[dict[str, Any], bytes]:
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


def load_canonical_json(path: Path, description: str) -> dict[str, Any]:
    raw = read_bounded(path, MAX_CONTROL_BYTES, description)
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail(f"{description} must have one trailing newline")
    try:
        value = json.loads(raw, object_pairs_hook=reject_duplicate_key)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"{description} is invalid JSON: {error}")
    expected = (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")
    if raw != expected or not isinstance(value, dict):
        fail(f"{description} is not canonical JSON")
    return value


def parse_kv(path: Path, keys: tuple[str, ...], description: str) -> dict[str, str]:
    raw = read_bounded(path, MAX_CONTROL_BYTES, description)
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail(f"{description} must have one trailing newline")
    try:
        lines = raw.decode("ascii").splitlines()
    except UnicodeDecodeError as error:
        fail(f"{description} is not ASCII: {error}")
    if len(lines) != len(keys):
        fail(f"{description} record count drifted")
    result: dict[str, str] = {}
    for line, expected_key in zip(lines, keys, strict=True):
        if "=" not in line:
            fail(f"{description} contains a malformed record")
        key, value = line.split("=", 1)
        if key != expected_key or not value or key in result:
            fail(f"{description} field order or identity drifted")
        result[key] = value
    return result


def canonical_digest(value: dict[str, Any]) -> str:
    payload = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    return digest_bytes(payload.encode("ascii"))


def command_output(repo: Path, arguments: list[str], description: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), *arguments],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=10,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"cannot {description}: {error}")
    if result.returncode != 0:
        fail(f"cannot {description}: {result.stderr.strip()}")
    return result.stdout


def git_identity(repo: Path) -> tuple[str, str]:
    commit = command_output(
        repo, ["rev-parse", "HEAD^{commit}"], "resolve qualified Ferric commit"
    ).strip()
    tree = command_output(
        repo, ["rev-parse", "HEAD^{tree}"], "resolve qualified Ferric tree"
    ).strip()
    return (
        require_git_id(commit, "qualified Git commit"),
        require_git_id(tree, "qualified Git tree"),
    )


def source_closure(repo: Path) -> tuple[bytes, set[str]]:
    records: list[str] = []
    members: set[str] = set()
    try:
        candidates = sorted(
            repo.rglob("*"), key=lambda path: path.relative_to(repo).as_posix()
        )
        for path in candidates:
            relative = path.relative_to(repo)
            if any(part in SOURCE_EXCLUDED_DIRECTORIES for part in relative.parts):
                continue
            if path.is_symlink():
                fail(f"qualified Ferric source closure contains a symlink: {relative}")
            if path.is_dir():
                continue
            if not path.is_file():
                fail(
                    f"qualified Ferric source closure contains a special entry: {relative}"
                )
            if path.suffix in SOURCE_EXCLUDED_SUFFIXES:
                fail(
                    f"qualified Ferric source closure contains a generated input: {relative}"
                )
            name = relative.as_posix()
            metadata = path.stat()
            mode = stat.S_IMODE(metadata.st_mode)
            records.append(f"{name}|{mode:o}|{metadata.st_size}|{digest_file(path)}")
            members.add(name)
    except (OSError, ValueError) as error:
        fail(f"cannot measure qualified Ferric source closure: {error}")
    if not records:
        fail("qualified Ferric source closure is empty")
    return ("\n".join(records) + "\n").encode("utf-8"), members


def qualified_source_identity(repo: Path) -> tuple[str, str, str]:
    status = command_output(
        repo,
        ["status", "--porcelain=v1", "--untracked-files=all"],
        "inspect qualified Ferric source",
    )
    if status:
        fail("qualified Ferric source worktree is not clean")
    commit, tree = git_identity(repo)
    closure, members = source_closure(repo)
    tracked = {
        name
        for name in command_output(
            repo, ["ls-tree", "-r", "--name-only", "HEAD"], "enumerate Ferric tree"
        ).splitlines()
        if not any(part in SOURCE_EXCLUDED_DIRECTORIES for part in Path(name).parts)
        and Path(name).suffix not in SOURCE_EXCLUDED_SUFFIXES
    }
    if members != tracked:
        fail("qualified Ferric source closure is not the exact committed tree")
    return commit, tree, digest_bytes(closure)


def validate_context(
    repo: Path, context: dict[str, Any]
) -> tuple[Path, dict[str, Any], dict[str, Any], dict[str, Any]]:
    if context["format"] != INDEX_FORMAT:
        fail("negative-mutation context index format drifted")
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = load_canonical_json(requirements_path, "M1 requirements")
    if context["requirements_sha256"] != digest_file(requirements_path):
        fail("negative-mutation context requirements identity drifted")

    artifact = exact_keys(context["artifact"], ARTIFACT_KEYS, "artifact context")
    if artifact["kind"] != "MutationTranscript":
        fail("negative-mutation artifact kind drifted")
    require_name(artifact["id"], "artifact id")
    artifact_digest = require_sha256(artifact["sha256"], "artifact SHA-256")
    if not isinstance(artifact["size_bytes"], int) or artifact["size_bytes"] <= 0:
        fail("negative-mutation artifact size is invalid")
    if not isinstance(context["artifact_absolute_path"], str):
        fail("negative-mutation artifact absolute path is invalid")
    artifact_path = Path(context["artifact_absolute_path"])
    if not artifact_path.is_absolute():
        fail("negative-mutation artifact path must be absolute")
    if (
        not isinstance(artifact["path"], str)
        or not artifact["path"]
        or Path(artifact["path"]).is_absolute()
        or ".." in Path(artifact["path"]).parts
        or Path(artifact["path"]).name != artifact_path.name
    ):
        fail("negative-mutation artifact relative path escaped or drifted")
    artifact_bytes = read_bounded(
        artifact_path, MAX_CONTROL_BYTES, "negative-mutation result artifact"
    )
    if (
        len(artifact_bytes) != artifact["size_bytes"]
        or digest_bytes(artifact_bytes) != artifact_digest
    ):
        fail("negative-mutation artifact bytes do not match their context identity")

    binding = exact_keys(context["binding"], BINDING_KEYS, "binding context")
    require_name(binding["id"], "binding id")
    require_name(binding["artifact_id"], "binding artifact id")
    require_name(binding["obligation_id"], "binding obligation id")
    require_name(binding["path_id"], "binding path id")
    require_name(binding["profile_id"], "binding profile id")
    require_name(binding["source_identity_id"], "binding source identity id")
    require_sha256(binding["binding_sha256"], "binding SHA-256")
    require_sha256(binding["statement_sha256"], "binding statement SHA-256")
    if not isinstance(binding["tcb_ids"], list):
        fail("negative-mutation binding TCB roster drifted")
    if (
        context["subject"] != f"binding:{binding['id']}"
        or binding["artifact_id"] != artifact["id"]
        or binding["evidence_kind"] != "negative-mutation"
        or binding["obligation_class"] != "Assurance"
        or binding["source_identity_id"] != "source.ferric"
        or tuple(binding["tcb_ids"]) != TCB_IDS
    ):
        fail("negative-mutation binding context drifted")
    binding_payload = {
        key: value for key, value in binding.items() if key != "binding_sha256"
    }
    if binding["binding_sha256"] != canonical_digest(binding_payload):
        fail("negative-mutation binding identity mismatch")

    properties = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    property_record = properties.get(binding["obligation_id"])
    if property_record is None or property_record["obligation_state"] != "Open":
        fail("negative-mutation binding does not name an Open assurance property")
    if binding["profile_id"] not in property_record["evidence_profiles"]:
        fail("negative-mutation binding profile is not assigned to its property")
    profiles = {
        record["id"]: record["kinds"] for record in requirements["evidence_profiles"]
    }
    if "negative-mutation" not in profiles.get(binding["profile_id"], []):
        fail("negative-mutation binding profile does not require mutation evidence")
    if binding["statement_sha256"] != digest_bytes(
        property_record["boundary"].encode("utf-8")
    ):
        fail("negative-mutation binding statement identity drifted")

    resolution = exact_keys(context["path_resolution"], PATH_KEYS, "path context")
    path_records = {record["id"]: record for record in requirements["path_obligations"]}
    expected_path = path_records.get(binding["path_id"])
    if (
        expected_path is None
        or resolution["id"] != binding["path_id"]
        or resolution["availability"] != expected_path["availability"]
        or resolution["path"] != expected_path["path"]
        or resolution["repository"] != "ferric"
        or resolution["source_identity_id"] != "source.ferric"
    ):
        fail("negative-mutation path context drifted")

    sources = context["sources"]
    if not isinstance(sources, list) or len(sources) != 2:
        fail("negative-mutation source context roster drifted")
    source_map: dict[str, dict[str, Any]] = {}
    for source in sources:
        record = exact_keys(source, SOURCE_KEYS, "source context")
        identifier = record["id"]
        if not isinstance(identifier, str) or identifier not in SOURCE_REPOSITORIES:
            fail("unknown negative-mutation source context")
        if identifier in source_map:
            fail("duplicate negative-mutation source context")
        require_git_id(record["commit"], f"{identifier} commit")
        require_git_id(record["tree"], f"{identifier} tree")
        require_sha256(record["source_closure_sha256"], f"{identifier} source closure")
        require_name(
            record["source_closure_artifact_id"], f"{identifier} closure artifact"
        )
        if record["repository"] != SOURCE_REPOSITORIES[identifier]:
            fail(f"negative-mutation source repository drifted: {identifier}")
        source_map[identifier] = record
    if tuple(source_map) != SOURCE_IDS:
        fail("negative-mutation source context order drifted")
    ferric_source = source_map["source.ferric"]
    if ferric_source["base_commit"] != FERRIC_BASE_COMMIT:
        fail("negative-mutation Ferric source base identity drifted")
    if (
        source_map["source.fe2o3"]["base_commit"]
        != requirements["m1_upstream_base_commit"]
    ):
        fail("negative-mutation fe2o3 source base identity drifted")
    actual_commit, actual_tree, actual_closure = qualified_source_identity(repo)
    if (
        ferric_source["commit"] != actual_commit
        or ferric_source["tree"] != actual_tree
        or ferric_source["source_closure_sha256"] != actual_closure
    ):
        fail("negative-mutation Ferric context is not the qualified source identity")

    tcb = context["tcb"]
    if not isinstance(tcb, list) or len(tcb) != len(TCB_IDS):
        fail("negative-mutation TCB roster drifted")
    for record, expected_id in zip(tcb, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, "TCB context")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("negative-mutation TCB order or identity drifted")
        require_name(record["artifact_id"], f"{expected_id} artifact")
        require_sha256(record["identity_sha256"], f"{expected_id} identity")
    return artifact_path, requirements, binding, ferric_source


def registry_rows(repo: Path) -> tuple[bytes, list[tuple[str, ...]]]:
    path = repo / "proofs/m1/negative/REQUIRED_FOUNDATIONS"
    raw = read_bounded(path, MAX_CONTROL_BYTES, "M1 mutation registry")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("M1 mutation registry must have one trailing newline")
    try:
        lines = raw.decode("ascii").splitlines()
    except UnicodeDecodeError as error:
        fail(f"M1 mutation registry is not ASCII: {error}")
    if not lines or lines[0] != REGISTRY_FORMAT:
        fail("M1 mutation registry format drifted")
    rows: list[tuple[str, ...]] = []
    for line in lines[1:]:
        if not line.startswith("mutation="):
            fail("M1 mutation registry contains a malformed row")
        fields = tuple(line.removeprefix("mutation=").split("|"))
        if len(fields) != 11:
            fail("M1 mutation registry row width drifted")
        rows.append(fields)
    if not rows or [row[0] for row in rows] != sorted({row[0] for row in rows}):
        fail("M1 mutation registry order or uniqueness drifted")
    active = "".join("|".join(row) + "\n" for row in rows).encode("ascii")
    return active, rows


def exact_run_files(run_dir: Path, selected: list[tuple[str, ...]]) -> None:
    expected = {
        "RUN_IDENTITY",
        "active-foundations",
        "selected-foundations",
        "verus-closure.transcript",
    }
    for row in selected:
        name = row[0]
        expected.update(
            {
                f"{name}.compile.transcript",
                f"{name}.mutation",
                f"{name}.result",
                f"{name}.verus.transcript",
            }
        )
    try:
        entries = list(run_dir.iterdir())
    except OSError as error:
        fail(f"cannot enumerate negative-mutation run directory: {error}")
    observed = {entry.name for entry in entries}
    if observed != expected or len(entries) != len(expected):
        fail("negative-mutation run file roster is incomplete or contains extras")
    for entry in entries:
        regular_file(entry, f"negative-mutation run member {entry.name}")


def validate_run_identity(
    repo: Path,
    run_dir: Path,
    ferric_source: dict[str, Any],
    active_bytes: bytes,
) -> tuple[dict[str, str], bytes]:
    path = run_dir / "RUN_IDENTITY"
    values = parse_kv(path, RUN_KEYS, "negative-mutation run identity")
    if values["FORMAT"] != RUN_FORMAT:
        fail("negative-mutation run format drifted")
    if (
        values["FERRIC_COMMIT"] != ferric_source["commit"]
        or values["FERRIC_TREE"] != ferric_source["tree"]
        or values["FERRIC_SOURCE_CLOSURE_SHA256"]
        != ferric_source["source_closure_sha256"]
    ):
        fail("negative-mutation run source commit, tree, or closure drifted")
    require_git_id(values["FERRIC_COMMIT"], "run Ferric commit")
    require_git_id(values["FERRIC_TREE"], "run Ferric tree")
    for key in (
        "FERRIC_SOURCE_CLOSURE_SHA256",
        "VERUS_SHA256",
        "VERUS_CLOSURE_MANIFEST_SHA256",
        "VERUS_CLOSURE_SHA256",
        "REGISTRY_SHA256",
        "RUNNER_SHA256",
    ):
        require_sha256(values[key], f"run {key}")

    version = (repo / "proofs/verus/VERUS_VERSION").read_text(encoding="ascii")
    verus_sha = (repo / "proofs/verus/VERUS_SHA256").read_text(encoding="ascii")
    if (
        version != values["VERUS_VERSION"] + "\n"
        or verus_sha != values["VERUS_SHA256"] + "\n"
    ):
        fail("negative-mutation run Verus release identity drifted")
    manifest_path = repo / "proofs/verus/VERUS_CLOSURE_MANIFEST"
    manifest = read_bounded(manifest_path, MAX_CONTROL_BYTES, "Verus closure manifest")
    if values["VERUS_CLOSURE_MANIFEST_SHA256"] != digest_bytes(manifest):
        fail("negative-mutation Verus closure-manifest identity drifted")
    closure_values = [
        line.removeprefix(b"closure-sha256=").decode("ascii")
        for line in manifest.splitlines()
        if line.startswith(b"closure-sha256=")
    ]
    if closure_values != [values["VERUS_CLOSURE_SHA256"]]:
        fail("negative-mutation Verus closure identity drifted")
    if values["REGISTRY_SHA256"] != digest_file(
        repo / "proofs/m1/negative/REQUIRED_FOUNDATIONS"
    ) or values["RUNNER_SHA256"] != digest_file(
        repo / "proofs/m1/negative/run-same-source.sh"
    ):
        fail("negative-mutation registry or runner identity drifted")
    if (
        values["AUTHORITY"] != "hostile-foundation-proof-rejection-only"
        or values["NONCLAIM"] != "no-m1-property-or-roadmap-closure"
    ):
        fail("negative-mutation authority boundary drifted")

    closure_transcript = read_bounded(
        run_dir / "verus-closure.transcript",
        MAX_CONTROL_BYTES,
        "Verus closure transcript",
    )
    file_count = [
        line.removeprefix(b"file-count=").decode("ascii")
        for line in manifest.splitlines()
        if line.startswith(b"file-count=")
    ]
    total_bytes = [
        line.removeprefix(b"total-bytes=").decode("ascii")
        for line in manifest.splitlines()
        if line.startswith(b"total-bytes=")
    ]
    if len(file_count) != 1 or len(total_bytes) != 1:
        fail("Verus closure manifest count records drifted")
    expected_closure = (
        f"PASS: pinned Verus release closure matched "
        f"({file_count[0]} files, {total_bytes[0]} bytes)\n"
    ).encode("ascii")
    if closure_transcript != expected_closure or active_bytes == b"":
        fail("negative-mutation Verus closure transcript drifted")
    return values, closure_transcript


def companion(
    run_dir: Path,
    expected_name: str,
    recorded_name: str,
    recorded_size: str,
    recorded_digest: str,
    description: str,
    limit: int,
) -> tuple[Path, bytes]:
    if recorded_name != expected_name or Path(recorded_name).name != recorded_name:
        fail(f"{description} path escaped or drifted")
    size = require_decimal(recorded_size, f"{description} size")
    if size <= 0:
        fail(f"{description} size must be positive")
    digest = require_sha256(recorded_digest, f"{description} SHA-256")
    path = run_dir / recorded_name
    raw = read_bounded(path, limit, description)
    if len(raw) != size or digest_bytes(raw) != digest:
        fail(f"{description} byte identity mismatch")
    return path, raw


def validate_mutator(
    repo: Path,
    row: tuple[str, ...],
    record: dict[str, str],
) -> None:
    (
        name,
        foundation,
        property_name,
        path_id,
        package,
        source,
        mutator,
        marker,
        module,
        function,
        clause,
    ) = row
    expected = {
        "FORMAT": MUTATION_FORMAT,
        "MUTATED_SOURCE": source,
        "MUTATION": name,
        "CLAUSE": clause,
        "FOUNDATION": foundation,
        "OPEN_ASSURANCE_PROPERTY": property_name,
        "OPEN_PATH_OBLIGATION": path_id,
        "VERUS_PACKAGE": package,
        "VERUS_MODULE": module,
        "VERUS_FUNCTION": function,
        "EXPECTED_FAILURE_MARKER": marker,
        "CARGO_CHECK": "passed",
    }
    for key, value in expected.items():
        if record[key] != value:
            fail(f"negative-mutation marker binding drifted: {name}/{key}")
    anchor = require_sha256(record["ANCHOR_SHA256"], f"{name} anchor")
    source_path = repo / source
    mutator_path = repo / "proofs/m1/negative/components" / mutator
    regular_file(source_path, f"{name} original source")
    regular_file(mutator_path, f"{name} mutator")
    if record["ORIGINAL_SOURCE_SHA256"] != digest_file(source_path):
        fail(f"negative-mutation original source identity drifted: {name}")
    if record["MUTATOR_SHA256"] != digest_file(mutator_path):
        fail(f"negative-mutation mutator identity drifted: {name}")
    require_sha256(record["MUTATED_SOURCE_SHA256"], f"{name} mutated source")

    with tempfile.TemporaryDirectory(prefix="ferric-m1-validator-mutation.") as scratch:
        root = Path(scratch)
        copied = root / source
        copied.parent.mkdir(parents=True)
        shutil.copy2(source_path, copied)
        try:
            result = subprocess.run(
                [sys.executable, "-I", str(mutator_path), str(root)],
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                timeout=10,
                env={"PATH": os.environ.get("PATH", "")},
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            fail(f"cannot reconstruct negative mutation {name}: {error}")
        expected_stdout = (
            f"MUTATED_SOURCE={source}\nMUTATION={name}\nCLAUSE={clause}\n"
            f"ANCHOR_SHA256={anchor}\n"
        )
        if result.returncode != 0 or result.stdout != expected_stdout:
            fail(f"negative-mutation current anchor does not reconstruct: {name}")
        if digest_file(copied) != record["MUTATED_SOURCE_SHA256"]:
            fail(f"negative-mutation transformed source identity drifted: {name}")


def transcript_parts(raw: bytes, headers: tuple[str, ...], description: str) -> str:
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        fail(f"{description} is not UTF-8: {error}")
    lines = text.splitlines(keepends=True)
    if len(lines) <= len(headers):
        fail(f"{description} has no compiler output")
    actual = tuple(line.removesuffix("\n") for line in lines[: len(headers)])
    if actual != headers or any(not line.endswith("\n") for line in lines):
        fail(f"{description} header, order, or trailing newline drifted")
    return "".join(lines[len(headers) :])


def validate_compile_transcript(raw: bytes, row: tuple[str, ...]) -> None:
    name, _, _, _, package, _, _, _, _, _, _ = row
    body = transcript_parts(
        raw,
        (
            f"FORMAT={COMPILE_FORMAT}",
            f"MUTATION={name}",
            f"CARGO_PACKAGE={package}",
            "COMMAND=cargo-check-locked-all-targets",
        ),
        f"{name} compile transcript",
    )
    if len(body.encode("utf-8")) < 200:
        fail(f"negative-mutation compile transcript is implausibly short: {name}")
    if re.search(rf"\b(?:Checking|Compiling) {re.escape(package)} v", body) is None:
        fail(
            f"negative-mutation compile transcript did not compile its package: {name}"
        )
    if "Finished `dev` profile" not in body:
        fail(
            f"negative-mutation compile transcript has no successful terminal result: {name}"
        )
    prohibited = ("error:", "could not compile", "timed out", "timeout", "FAIL:")
    if any(value.lower() in body.lower() for value in prohibited):
        fail(f"negative-mutation ordinary compilation was not clean: {name}")


def validate_verus_transcript(raw: bytes, row: tuple[str, ...]) -> None:
    name, _, _, _, package, source, _, marker, module, function, _ = row
    body = transcript_parts(
        raw,
        (
            f"FORMAT={VERUS_FORMAT}",
            f"MUTATION={name}",
            f"VERUS_PACKAGE={package}",
            f"VERUS_MODULE={module}",
            f"VERUS_FUNCTION={function}",
            "COMMAND=cargo-verus-build-lib-locked-release-no-cheating-exact-function",
            f"EXPECTED_FAILURE_MARKER={marker}",
        ),
        f"{name} Verus transcript",
    )
    if len(body.encode("utf-8")) < 500:
        fail(f"negative-mutation Verus transcript is implausibly short: {name}")
    package_compile = re.compile(
        rf"^\s+Compiling {re.escape(package)} v[^\n]+"
        rf"\([^\n]*/copy-{re.escape(name)}/crates/{re.escape(package)}\)\s*$",
        re.MULTILINE,
    )
    if len(package_compile.findall(body)) != 1:
        fail(f"negative-mutation Verus did not compile its selected package: {name}")
    module_note = f"note: verifying module {module} (selected functions)"
    if body.count(module_note) != 1:
        fail(f"negative-mutation Verus selected-module diagnostic drifted: {name}")
    expected_error = (
        f"error: {marker} failed"
        if marker == "assertion"
        else "error: postcondition not satisfied"
    )
    source_diagnostic = re.compile(
        rf"^\s*--> {re.escape(source)}:[1-9][0-9]*:[1-9][0-9]*\s*$",
        re.MULTILINE,
    )
    if expected_error not in body:
        fail(
            f"negative-mutation Verus transcript lacks its exact proof diagnostic: {name}"
        )
    body_lines = body.splitlines()
    proof_diagnostics = [
        index for index, line in enumerate(body_lines) if line == expected_error
    ]
    for index in proof_diagnostics:
        next_error = next(
            (
                position
                for position in range(index + 1, len(body_lines))
                if body_lines[position].startswith("error:")
            ),
            len(body_lines),
        )
        if not any(
            source_diagnostic.fullmatch(line)
            for line in body_lines[index + 1 : next_error]
        ):
            fail(
                f"negative-mutation proof diagnostic is not bound to its selected source: {name}"
            )
    proof_diagnostic_count = len(proof_diagnostics)
    other_error = (
        "error: postcondition not satisfied"
        if marker == "assertion"
        else "error: assertion failed"
    )
    if other_error in body:
        fail(f"negative-mutation Verus transcript has the wrong proof marker: {name}")
    results = re.findall(
        r"verification results:: ([0-9]+) verified, ([0-9]+) errors", body
    )
    if (
        not results
        or results[-1] != ("0", "1")
        or any(errors != "0" for _, errors in results[:-1])
    ):
        fail(f"negative-mutation Verus result count drifted: {name}")
    terminal_suffix = "error" if proof_diagnostic_count == 1 else "errors"
    terminal = re.compile(
        rf"error: could not compile `{re.escape(package)}` \(lib\) "
        rf"due to {proof_diagnostic_count} previous {terminal_suffix}\n\Z"
    )
    if terminal.search(body) is None:
        fail(
            f"negative-mutation Verus transcript has no exact rejected terminal result: {name}"
        )
    for line in body.splitlines():
        if (
            line.startswith("error:")
            and line != expected_error
            and terminal.fullmatch(line + "\n") is None
        ):
            fail(
                f"negative-mutation Verus transcript contains a non-proof error: {name}"
            )
    prohibited = (
        "timed out",
        "timeout",
        "could not find module",
        "could not find function",
        "available modules are",
        "assume/admit not allowed",
        "no space left",
        "permission denied",
        "failed to get",
        "connection refused",
        "panicked at",
        "unknown option",
    )
    if any(value in body.lower() for value in prohibited):
        fail(
            f"negative-mutation Verus transcript contains an infrastructure error: {name}"
        )


def validate_result(
    repo: Path,
    run_dir: Path,
    row: tuple[str, ...],
    run_identity: bytes,
    active: bytes,
    selected: bytes,
    closure: bytes,
) -> dict[str, str]:
    name = row[0]
    result = parse_kv(run_dir / f"{name}.result", RESULT_KEYS, f"{name} result record")
    expected_scalars = {
        "FORMAT": RESULT_FORMAT,
        "MUTATION": name,
        "RUN_IDENTITY_SHA256": digest_bytes(run_identity),
        "ACTIVE_FOUNDATIONS_SHA256": digest_bytes(active),
        "SELECTED_FOUNDATIONS_SHA256": digest_bytes(selected),
        "VERUS_CLOSURE_TRANSCRIPT_SHA256": digest_bytes(closure),
        "COMPILE_EXIT_STATUS": "0",
        "VERUS_EXIT_STATUS": "101",
        "RESULT": "proof-rejected",
    }
    for key, value in expected_scalars.items():
        if result[key] != value:
            fail(f"negative-mutation result binding drifted: {name}/{key}")

    mutation_path, _ = companion(
        run_dir,
        f"{name}.mutation",
        result["MUTATION_RECORD"],
        result["MUTATION_RECORD_SIZE"],
        result["MUTATION_RECORD_SHA256"],
        f"{name} mutation record",
        MAX_CONTROL_BYTES,
    )
    compile_path, compile_raw = companion(
        run_dir,
        f"{name}.compile.transcript",
        result["COMPILE_TRANSCRIPT"],
        result["COMPILE_TRANSCRIPT_SIZE"],
        result["COMPILE_TRANSCRIPT_SHA256"],
        f"{name} compile transcript",
        MAX_TRANSCRIPT_BYTES,
    )
    _, verus_raw = companion(
        run_dir,
        f"{name}.verus.transcript",
        result["VERUS_TRANSCRIPT"],
        result["VERUS_TRANSCRIPT_SIZE"],
        result["VERUS_TRANSCRIPT_SHA256"],
        f"{name} Verus transcript",
        MAX_TRANSCRIPT_BYTES,
    )
    mutation = parse_kv(mutation_path, MUTATION_KEYS, f"{name} mutation record")
    validate_mutator(repo, row, mutation)
    validate_compile_transcript(compile_raw, row)
    validate_verus_transcript(verus_raw, row)
    regular_file(compile_path, f"{name} compile transcript")
    return result


def validate_run(
    repo: Path,
    artifact_path: Path,
    binding: dict[str, Any],
    ferric_source: dict[str, Any],
) -> None:
    run_dir = artifact_path.parent
    regular_file(artifact_path, "negative-mutation result artifact")
    active_expected, rows = registry_rows(repo)
    active_path = run_dir / "active-foundations"
    active = read_bounded(active_path, MAX_CONTROL_BYTES, "active mutation roster")
    if active != active_expected:
        fail("negative-mutation active registry does not match qualified source")
    selected = read_bounded(
        run_dir / "selected-foundations", MAX_CONTROL_BYTES, "selected mutation roster"
    )
    try:
        selected_lines = selected.decode("ascii").splitlines()
    except UnicodeDecodeError as error:
        fail(f"selected mutation roster is not ASCII: {error}")
    if not selected.endswith(b"\n") or selected.endswith(b"\n\n") or not selected_lines:
        fail("selected mutation roster is empty or has trailing data")
    row_by_line = {"|".join(row): row for row in rows}
    if len(selected_lines) != len(set(selected_lines)) or any(
        line not in row_by_line for line in selected_lines
    ):
        fail("selected mutation roster has duplicate or unknown rows")
    selected_names = {row_by_line[line][0] for line in selected_lines}
    expected_lines = ["|".join(row) for row in rows if row[0] in selected_names]
    if selected_lines != expected_lines:
        fail("selected mutation roster is reordered or noncanonical")
    selected_rows = [row_by_line[line] for line in selected_lines]
    binding_rows = [
        row
        for row in rows
        if row[2] == binding["obligation_id"] and row[3] == binding["path_id"]
    ]
    if not binding_rows or selected_rows != binding_rows:
        fail("selected mutation roster is incomplete for its bound property and path")
    exact_run_files(run_dir, selected_rows)

    run_values, closure = validate_run_identity(
        repo, run_dir, ferric_source, active_expected
    )
    run_identity = read_bounded(
        run_dir / "RUN_IDENTITY", MAX_CONTROL_BYTES, "negative-mutation run identity"
    )
    if run_values["FERRIC_COMMIT"] != ferric_source["commit"]:
        fail("negative-mutation run was replayed across source identities")

    results = {
        row[0]: validate_result(
            repo, run_dir, row, run_identity, active, selected, closure
        )
        for row in selected_rows
    }
    primary_name = artifact_path.name.removesuffix(".result")
    if artifact_path.name != f"{primary_name}.result" or primary_name not in results:
        fail("negative-mutation artifact does not name one selected result")
    primary_row = next(row for row in selected_rows if row[0] == primary_name)
    if (
        primary_row[2] != binding["obligation_id"]
        or primary_row[3] != binding["path_id"]
    ):
        fail("negative-mutation artifact was substituted across registry rows")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail(f"usage: {sys.argv[0]} {PROTOCOL}")
    context, payload = load_stdin_context()
    repo = Path.cwd().resolve(strict=True)
    artifact_path, _, binding, ferric_source = validate_context(repo, context)
    validate_run(repo, artifact_path, binding, ferric_source)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(payload)}"
    )


if __name__ == "__main__":
    main()
