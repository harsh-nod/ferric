#!/usr/bin/env python3
"""Offline, plan-bound Qwen3-8B reference producer for Ferric M1."""

from __future__ import annotations

import ctypes
import hashlib
import json
import os
import secrets
import stat
import struct
import sys
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


PLAN_FORMAT = "FERRIC-M1-BENCHMARK-PLAN-V1"
BENCHMARK_INPUT_FORMAT = "FERRIC-M1-BENCHMARK-INPUT-V1"
ROSTER_FORMAT = "FERRIC-M1-QUALIFICATION-ROSTER-V1"
WORKLOAD_FORMAT = "FERRIC-M1-QUALIFICATION-WORKLOAD-V3"
ENVIRONMENT_FORMAT = "FERRIC-M1-QUALIFICATION-ENVIRONMENT-V1"
QUALIFICATION_CLOSURE_FORMAT = "FERRIC-M1-QUALIFICATION-CLOSURE-V1"
CAPTURE_FORMAT = "FERRIC-M1-QUALIFICATION-CAPTURE-V2"
OUTPUT_FORMAT = "FERRIC-M1-DIFFERENTIAL-OUTPUT-V1"
ACCEPTANCE_POLICY_FORMAT = "FERRIC-M1-DIFFERENTIAL-ACCEPTANCE-POLICY-V1"
IMPLEMENTATION_FORMAT = "FERRIC-M1-REFERENCE-IMPLEMENTATION-V1"
PROTOCOL_FORMAT = "FERRIC-M1-REFERENCE-PROTOCOL-V3"
TARGET = "gfx942:xnack-"
VOCABULARY_SIZE = 151_936
INPUT_VOCABULARY_SIZE = 151_643
MAX_DOCUMENT_BYTES = 8 * 1_024 * 1_024
BF16_BYTES = 2
TOKEN_BYTES = 4
COMPLETION_WAIT_POLICY = {
    "id": "ferric-m1-completion-progress-wait-v2",
    "max_consecutive_scans_without_progress": 8_192,
    "minimum_pending_scan_pause_micros": 10_000,
    "timeout_basis": "paced-completion-signal-scans",
    "total_scan_bound_rule": "(packet-count+1)*max-consecutive-scans-without-progress",
}
PINNED_MODEL_IDENTITY = (
    "6dfba0acd1c00ce13cec7b5eebb180691bdb8855a7eee89876df2a0a12a2802b"
)
PINNED_REPOSITORY = "Qwen/Qwen3-8B"
PINNED_REVISION = "b968826d9c46dd6066d109eabc6255188de91218"
REFERENCE_NONCLAIM = (
    "This producer supplies plan-bound independent Qwen3 reference bytes only. "
    "It does not establish Ferric correctness, hardware correctness, an acceptance "
    "threshold, qualification authority, or close m1.r29."
)
DIFFERENTIAL_NONCLAIM = (
    "Structural acceptance authenticates externally collected target-only differential "
    "records only. It does not validate a logit tolerance, prove token equality, "
    "establish numerical or hardware correctness, qualify performance, or close m1.r29."
)
CAPTURE_NONCLAIM = (
    "Observed bytes only; this transcript does not establish a reference comparison, "
    "tolerance, numerical correctness, hardware correctness, performance, qualification, "
    "or m1.r29 closure."
)
ACCEPTANCE_POLICY_NONCLAIM = (
    "This artifact supplies plan-admitted differential thresholds only. It does not "
    "establish independent review, numerical correctness, hardware correctness, "
    "qualification authority, or close m1.r29."
)

CASE_GEOMETRY = {
    "decode-s1-c8192": (1, 8192, "decode"),
    "decode-s32-c8192": (32, 8192, "decode"),
    "decode-s8-c8192": (8, 8192, "decode"),
    "prefill-s1-t128": (1, 128, "prefill"),
    "prefill-s1-t2048": (1, 2048, "prefill"),
    "prefill-s1-t512": (1, 512, "prefill"),
    "prefill-s8-t128": (8, 128, "prefill"),
}
CASE_KINDS = tuple(CASE_GEOMETRY)

COMMON_IDENTITIES = (
    "benchmark-executable",
    "benchmark-protocol",
    "config",
    "dispatch-graph",
    "environment",
    "fe2o3-source-closure",
    "ferric-source-closure",
    "generated-plan",
    "model",
    "schedule-catalog",
    "tokenizer",
    "weights",
    "workload-roster",
)
DIFFERENTIAL_IDENTITIES = (
    "differential-acceptance-policy",
    "reference-implementation",
    "reference-protocol",
)
DISPATCH_IDENTITIES = tuple(f"dispatch-graph-{kind}" for kind in CASE_KINDS)
PLAN_IDENTITIES = tuple(
    sorted(COMMON_IDENTITIES + DIFFERENTIAL_IDENTITIES + DISPATCH_IDENTITIES)
)

DEPENDENCY_VERSIONS = {
    "numpy": "1.26.4",
    "python": "3.12",
    "safetensors": "0.5.3",
    "tokenizers": "0.21.4",
    "torch": "2.12.1+rocm7.2",
    "transformers": "4.51.0",
    "triton-rocm": "3.7.1",
}

MODEL_FILES = {
    "config.json": (
        728,
        "f7c4eadfbbf522470667b797a3c89be2524832d2d599797248dc304fff447c30",
    ),
    "model.safetensors.index.json": (
        32_878,
        "f9fdbcb91c23971c13ec5d5f2573d2349e8f61f2f049371ec699281748fdb1bc",
    ),
    "model-00001-of-00005.safetensors": (
        3_996_250_744,
        "31d6a825ae35f11fb85b195b4c42c146c051e446433125a215336abdf95cbf5f",
    ),
    "model-00002-of-00005.safetensors": (
        3_993_160_032,
        "5991236cea6fe21f3d43cab0f0e84448734fbbe0789816202989f2ddc9d18282",
    ),
    "model-00003-of-00005.safetensors": (
        3_959_604_768,
        "c5185c4794be2d8a9784d5753c9922db38df478ce11f9ed0b415b7304d896836",
    ),
    "model-00004-of-00005.safetensors": (
        3_187_841_392,
        "b5ee7de71fbf17db3d5704e0c8f2bc7d005ca9e1d7ca2aeb19827b0cfcaa917a",
    ),
    "model-00005-of-00005.safetensors": (
        1_244_659_840,
        "20c2d6366ab85c90786ccdd829cd2b9e7d30ef3b2ebbb998280e7e4014b542ff",
    ),
    "tokenizer.json": (
        11_422_654,
        "aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4",
    ),
    "tokenizer_config.json": (
        9_732,
        "d5d09f07b48c3086c508b30d1c9114bd1189145b74e982a265350c923acd8101",
    ),
}


class ReferenceFailure(Exception):
    """A fail-closed reference contract rejection."""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require_sha256(value: Any, description: str) -> str:
    if not isinstance(value, str) or len(value) != 64:
        raise ReferenceFailure(f"{description} must be a lowercase SHA-256 identity")
    if any(character not in "0123456789abcdef" for character in value):
        raise ReferenceFailure(f"{description} must be a lowercase SHA-256 identity")
    return value


def _reject_float(_: str) -> None:
    raise ReferenceFailure("canonical JSON does not admit floating-point values")


def _reject_constant(_: str) -> None:
    raise ReferenceFailure("canonical JSON does not admit nonfinite values")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ReferenceFailure(f"canonical JSON repeats object key {key!r}")
        result[key] = value
    return result


def canonical_bytes(value: Any) -> bytes:
    try:
        encoded = json.dumps(
            value,
            allow_nan=False,
            ensure_ascii=True,
            indent=2,
            sort_keys=True,
        )
    except (TypeError, ValueError) as error:
        raise ReferenceFailure(f"cannot encode canonical JSON: {error}") from error
    return encoded.encode("ascii") + b"\n"


def parse_canonical(data: bytes, description: str) -> Any:
    if not data or not data.isascii():
        raise ReferenceFailure(f"{description} must be nonempty ASCII JSON")
    try:
        value = json.loads(
            data,
            object_pairs_hook=_unique_object,
            parse_float=_reject_float,
            parse_constant=_reject_constant,
        )
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise ReferenceFailure(f"cannot parse {description}: {error}") from error
    if canonical_bytes(value) != data:
        raise ReferenceFailure(f"{description} is not canonical JSON")
    return value


def exact_object(value: Any, keys: Iterable[str], description: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ReferenceFailure(f"{description} must be an object")
    expected = set(keys)
    if set(value) != expected:
        raise ReferenceFailure(f"{description} field roster drifted")
    return value


def exact_array(value: Any, length: int, description: str) -> list[Any]:
    if not isinstance(value, list) or len(value) != length:
        raise ReferenceFailure(f"{description} must contain exactly {length} entries")
    return value


def string_field(value: dict[str, Any], key: str, description: str) -> str:
    field = value.get(key)
    if not isinstance(field, str):
        raise ReferenceFailure(f"{description} field {key} must be a string")
    return field


def integer_field(value: dict[str, Any], key: str, description: str) -> int:
    field = value.get(key)
    if (
        isinstance(field, bool)
        or not isinstance(field, int)
        or field < 0
        or field > 0xFFFF_FFFF_FFFF_FFFF
    ):
        raise ReferenceFailure(f"{description} field {key} must be an unsigned integer")
    return field


def expect_string(
    value: dict[str, Any], key: str, expected: str, description: str
) -> None:
    if string_field(value, key, description) != expected:
        raise ReferenceFailure(f"{description} field {key} drifted")


def expect_integer(
    value: dict[str, Any], key: str, expected: int, description: str
) -> None:
    if integer_field(value, key, description) != expected:
        raise ReferenceFailure(f"{description} field {key} drifted")


@dataclass(frozen=True)
class FileIdentity:
    device: int
    inode: int


def _same_snapshot(left: os.stat_result, right: os.stat_result) -> bool:
    return (
        left.st_dev == right.st_dev
        and left.st_ino == right.st_ino
        and left.st_mode == right.st_mode
        and left.st_nlink == right.st_nlink
        and left.st_size == right.st_size
        and left.st_mtime_ns == right.st_mtime_ns
        and left.st_ctime_ns == right.st_ctime_ns
    )


class SecureFile:
    def __init__(self, fd: int, initial: os.stat_result, description: str) -> None:
        self.fd = fd
        self.initial = initial
        self.description = description

    @property
    def identity(self) -> FileIdentity:
        return FileIdentity(self.initial.st_dev, self.initial.st_ino)

    def validate(self) -> None:
        current = os.fstat(self.fd)
        if not _same_snapshot(self.initial, current):
            raise ReferenceFailure(f"{self.description} changed during use")

    def read(self, maximum: int | None = None, exact: int | None = None) -> bytes:
        if exact is not None and self.initial.st_size != exact:
            raise ReferenceFailure(f"{self.description} length drifted")
        if maximum is not None and not 0 < self.initial.st_size <= maximum:
            raise ReferenceFailure(
                f"{self.description} size is outside the admitted bound"
            )
        os.lseek(self.fd, 0, os.SEEK_SET)
        remaining = self.initial.st_size
        chunks: list[bytes] = []
        while remaining:
            chunk = os.read(self.fd, min(1024 * 1024, remaining))
            if not chunk:
                raise ReferenceFailure(
                    f"{self.description} ended during its exact read"
                )
            chunks.append(chunk)
            remaining -= len(chunk)
        if os.read(self.fd, 1):
            raise ReferenceFailure(f"{self.description} grew during its exact read")
        self.validate()
        return b"".join(chunks)

    def digest(self, expected_bytes: int, expected_sha256: str) -> None:
        if self.initial.st_size != expected_bytes:
            raise ReferenceFailure(f"{self.description} length drifted")
        os.lseek(self.fd, 0, os.SEEK_SET)
        digest = hashlib.sha256()
        remaining = expected_bytes
        while remaining:
            chunk = os.read(self.fd, min(8 * 1024 * 1024, remaining))
            if not chunk:
                raise ReferenceFailure(
                    f"{self.description} ended during authentication"
                )
            digest.update(chunk)
            remaining -= len(chunk)
        if os.read(self.fd, 1):
            raise ReferenceFailure(f"{self.description} grew during authentication")
        self.validate()
        if digest.hexdigest() != expected_sha256:
            raise ReferenceFailure(f"{self.description} SHA-256 drifted")

    def close(self) -> None:
        if self.fd >= 0:
            os.close(self.fd)
            self.fd = -1

    def __enter__(self) -> SecureFile:
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()


def _path_parts(path: Path) -> tuple[bool, tuple[str, ...]]:
    raw = PurePosixPath(os.fspath(path))
    absolute = raw.is_absolute()
    parts: list[str] = []
    for part in raw.parts:
        if part in ("", "/", "."):
            continue
        if part == "..":
            raise ReferenceFailure("secure paths must not contain parent traversal")
        parts.append(part)
    return absolute, tuple(parts)


def _open_directory_path(path: Path, description: str) -> int:
    absolute, parts = _path_parts(path)
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    fd = os.open("/" if absolute else ".", flags)
    try:
        for part in parts:
            next_fd = os.open(part, flags, dir_fd=fd)
            os.close(fd)
            fd = next_fd
        if not stat.S_ISDIR(os.fstat(fd).st_mode):
            raise ReferenceFailure(f"{description} must be a directory")
        return fd
    except OSError as error:
        os.close(fd)
        raise ReferenceFailure(
            f"cannot open {description} without symlinks: {error}"
        ) from error
    except Exception:
        os.close(fd)
        raise


class SecureDirectory:
    def __init__(self, fd: int, description: str) -> None:
        self.fd = fd
        self.description = description

    @classmethod
    def open(cls, path: Path, description: str) -> SecureDirectory:
        return cls(_open_directory_path(path, description), description)

    @property
    def identity(self) -> FileIdentity:
        current = os.fstat(self.fd)
        return FileIdentity(current.st_dev, current.st_ino)

    def entries(self) -> set[str]:
        try:
            return set(os.listdir(self.fd))
        except OSError as error:
            raise ReferenceFailure(
                f"cannot list {self.description}: {error}"
            ) from error

    def child(self, name: str, description: str) -> SecureDirectory:
        require_relative_name(name, description)
        flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
        try:
            fd = os.open(name, flags, dir_fd=self.fd)
        except OSError as error:
            raise ReferenceFailure(f"cannot open {description}: {error}") from error
        return SecureDirectory(fd, description)

    def open_file(self, relative: str, description: str) -> SecureFile:
        parts = relative_parts(relative, description)
        current = os.dup(self.fd)
        flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC
        try:
            for part in parts[:-1]:
                next_fd = os.open(
                    part,
                    os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                    dir_fd=current,
                )
                os.close(current)
                current = next_fd
            fd = os.open(parts[-1], flags, dir_fd=current)
        except OSError as error:
            raise ReferenceFailure(f"cannot open {description}: {error}") from error
        finally:
            os.close(current)
        initial = os.fstat(fd)
        if not stat.S_ISREG(initial.st_mode) or initial.st_nlink != 1:
            os.close(fd)
            raise ReferenceFailure(f"{description} must be a one-link regular file")
        return SecureFile(fd, initial, description)

    def read(
        self, relative: str, description: str, maximum: int = MAX_DOCUMENT_BYTES
    ) -> bytes:
        with self.open_file(relative, description) as source:
            return source.read(maximum=maximum)

    def read_canonical(
        self, relative: str, description: str
    ) -> tuple[Any, bytes, FileIdentity]:
        with self.open_file(relative, description) as source:
            data = source.read(maximum=MAX_DOCUMENT_BYTES)
            return parse_canonical(data, description), data, source.identity

    def close(self) -> None:
        if self.fd >= 0:
            os.close(self.fd)
            self.fd = -1

    def __enter__(self) -> SecureDirectory:
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()


def require_relative_name(name: str, description: str) -> None:
    if not name or "/" in name or name in (".", ".."):
        raise ReferenceFailure(f"{description} must be one safe path component")


def relative_parts(value: str, description: str) -> tuple[str, ...]:
    path = PurePosixPath(value)
    if path.is_absolute() or not path.parts:
        raise ReferenceFailure(f"{description} must be a relative path")
    parts = tuple(path.parts)
    if any(part in ("", ".", "..") for part in parts):
        raise ReferenceFailure(f"{description} contains unsafe traversal")
    return parts


def open_parent(path: Path, description: str) -> tuple[SecureDirectory, str]:
    name = path.name
    require_relative_name(name, description)
    parent = path.parent if os.fspath(path.parent) else Path(".")
    return SecureDirectory.open(parent, f"{description} parent"), name


def read_canonical_path(
    path: Path, description: str
) -> tuple[Any, bytes, FileIdentity]:
    with open_parent(path, description)[0] as parent:
        return parent.read_canonical(path.name, description)


@dataclass(frozen=True)
class PlanCase:
    case_id: str
    input_sha256: str
    kind: str
    workload_sha256: str


@dataclass(frozen=True)
class Plan:
    data: bytes
    cases: tuple[PlanCase, ...]
    identities: dict[str, str]
    input_sha256: str

    @property
    def sha256(self) -> str:
        return sha256_bytes(self.data)


@dataclass(frozen=True)
class Lane:
    active_length: int
    context_length: int


@dataclass(frozen=True)
class Workload:
    data: bytes
    case: PlanCase
    lanes: tuple[Lane, ...]
    tokens: tuple[tuple[int, ...], ...]


@dataclass(frozen=True)
class CaptureTranscript:
    data: bytes
    sha256: str
    case: PlanCase


@dataclass(frozen=True)
class ReferenceBundle:
    case: PlanCase
    logits: bytes
    tokens: bytes
    runner: bytes
    manifest: bytes


def parse_implementation_manifest(
    value: Any,
    data: bytes,
    base: SecureDirectory,
    running_script: Path,
) -> str:
    document = exact_object(
        value,
        ("authority", "files", "format", "python"),
        "reference implementation manifest",
    )
    expect_string(
        document,
        "authority",
        "source-reviewed-reference-implementation-closure-only",
        "reference implementation manifest",
    )
    expect_string(
        document, "format", IMPLEMENTATION_FORMAT, "reference implementation manifest"
    )
    expect_string(document, "python", "3.12", "reference implementation manifest")
    files = exact_array(document["files"], 3, "reference implementation files")
    expected_names = ("pyproject.toml", "run.py", "uv.lock")
    seen: list[str] = []
    run_identity: FileIdentity | None = None
    for item in files:
        entry = exact_object(item, ("bytes", "path", "sha256"), "implementation file")
        path = string_field(entry, "path", "implementation file")
        require_relative_name(path, "implementation file path")
        if path not in expected_names:
            raise ReferenceFailure(
                f"implementation manifest names unexpected file {path!r}"
            )
        expected_bytes = integer_field(entry, "bytes", "implementation file")
        expected_sha256 = require_sha256(
            entry["sha256"], "implementation file identity"
        )
        with base.open_file(path, f"implementation file {path}") as source:
            source.digest(expected_bytes, expected_sha256)
            if path == "run.py":
                run_identity = source.identity
        seen.append(path)
    if tuple(seen) != expected_names:
        raise ReferenceFailure("implementation files must be uniquely sorted by path")
    _, _, actual_script_identity = read_canonical_or_bytes_path(
        running_script,
        "running reference implementation",
        canonical=False,
    )
    if run_identity != actual_script_identity:
        raise ReferenceFailure(
            "running script is not the implementation-manifest run.py"
        )
    return sha256_bytes(data)


def read_canonical_or_bytes_path(
    path: Path,
    description: str,
    *,
    canonical: bool,
) -> tuple[Any | None, bytes, FileIdentity]:
    parent, name = open_parent(path, description)
    with parent:
        with parent.open_file(name, description) as source:
            data = source.read(maximum=MAX_DOCUMENT_BYTES)
            value = parse_canonical(data, description) if canonical else None
            return value, data, source.identity


def validate_protocol(value: Any) -> None:
    document = exact_object(
        value,
        (
            "authority",
            "cases",
            "dependencies",
            "execution",
            "format",
            "invocation",
            "model",
            "nonclaim",
            "output",
            "target",
        ),
        "reference protocol",
    )
    expect_string(
        document,
        "authority",
        "source-reviewed-independent-qwen3-reference-only",
        "reference protocol",
    )
    if (
        tuple(exact_array(document["cases"], 7, "reference protocol cases"))
        != CASE_KINDS
    ):
        raise ReferenceFailure("reference protocol case roster drifted")
    dependencies = exact_object(
        document["dependencies"], DEPENDENCY_VERSIONS, "reference protocol dependencies"
    )
    if dependencies != DEPENDENCY_VERSIONS:
        raise ReferenceFailure("reference protocol dependency pins drifted")
    execution = exact_object(
        document["execution"],
        (
            "attention_implementation",
            "completion_wait_policy",
            "determinism",
            "input_encoding",
            "lane_execution",
            "model_forward",
            "network",
            "package_provenance",
            "projection",
            "python_isolation",
            "remote_code",
            "row_order",
        ),
        "reference protocol execution",
    )
    completion_wait_policy = exact_object(
        execution["completion_wait_policy"],
        COMPLETION_WAIT_POLICY,
        "reference protocol completion wait policy",
    )
    if completion_wait_policy != COMPLETION_WAIT_POLICY:
        raise ReferenceFailure("reference protocol completion wait policy drifted")
    expected_execution = {
        "attention_implementation": "sdpa",
        "completion_wait_policy": COMPLETION_WAIT_POLICY,
        "determinism": "two-byte-identical-executions-per-case",
        "input_encoding": "lane-major-u32-le",
        "lane_execution": "sequential-full-context-per-lane-twice",
        "model_forward": "model.model(use_cache=false)",
        "network": "offline",
        "package_provenance": "active-non-base-virtualenv-only",
        "projection": "model.lm_head(last_hidden_state[:, -1:, :])",
        "python_isolation": "isolated-ignore-environment-no-user-site-safe-path",
        "remote_code": False,
        "row_order": "declared-lane-order",
    }
    if execution != expected_execution:
        raise ReferenceFailure("reference protocol execution contract drifted")
    expect_string(document, "format", PROTOCOL_FORMAT, "reference protocol")
    invocation = exact_object(
        document["invocation"],
        ("arguments", "command", "mode"),
        "reference protocol invocation",
    )
    if invocation != {
        "arguments": [
            "IMPLEMENTATION-MANIFEST",
            "PROTOCOL",
            "PLAN",
            "INPUT-BUNDLE",
            "MODEL-SOURCE",
            "FERRIC-CAPTURE-ROOT",
            "OUTPUT-ROOT",
        ],
        "command": "VENV/bin/python -I run.py",
        "mode": "all-seven-single-model-load",
    }:
        raise ReferenceFailure("reference protocol invocation contract drifted")
    model = exact_object(
        document["model"],
        (
            "deployment_bundle_identity",
            "dtype",
            "repository",
            "revision",
            "visible_gpu",
            "vocabulary_size",
        ),
        "reference protocol model",
    )
    expected_model = {
        "deployment_bundle_identity": PINNED_MODEL_IDENTITY,
        "dtype": "bfloat16",
        "repository": PINNED_REPOSITORY,
        "revision": PINNED_REVISION,
        "visible_gpu": "gcnArchName-gfx942-with-xnack-minus",
        "vocabulary_size": VOCABULARY_SIZE,
    }
    if model != expected_model:
        raise ReferenceFailure("reference protocol model contract drifted")
    expect_string(document, "nonclaim", REFERENCE_NONCLAIM, "reference protocol")
    output = exact_object(
        document["output"],
        ("bundle_naming", "logits_encoding", "publication", "token_encoding"),
        "reference protocol output",
    )
    if output != {
        "bundle_naming": "KIND.reference.bundle",
        "logits_encoding": "bf16-le",
        "publication": "single-root-rename-noreplace",
        "token_encoding": "lowest-token-id-bf16-argmax-u32-le",
    }:
        raise ReferenceFailure("reference protocol output contract drifted")
    expect_string(document, "target", TARGET, "reference protocol")


def parse_plan(value: Any, data: bytes) -> Plan:
    document = exact_object(
        value,
        (
            "authority",
            "cases",
            "format",
            "identities",
            "input_sha256",
            "milestone",
            "nonclaim",
            "obligation_id",
            "path_id",
            "source_path",
            "suite",
            "target",
        ),
        "benchmark plan",
    )
    expect_string(document, "authority", "benchmark-run-plan-only", "benchmark plan")
    expect_string(document, "format", PLAN_FORMAT, "benchmark plan")
    input_sha256 = require_sha256(document["input_sha256"], "benchmark input identity")
    expect_string(document, "milestone", "M1", "benchmark plan")
    expect_string(document, "nonclaim", DIFFERENTIAL_NONCLAIM, "benchmark plan")
    expect_string(document, "obligation_id", "m1.r29", "benchmark plan")
    expect_string(document, "path_id", "differential-bench", "benchmark plan")
    expect_string(
        document, "source_path", "benches/m1/differential.rs", "benchmark plan"
    )
    expect_string(document, "suite", "differential", "benchmark plan")
    expect_string(document, "target", TARGET, "benchmark plan")
    identities = exact_object(
        document["identities"], PLAN_IDENTITIES, "benchmark identities"
    )
    parsed_identities = {
        name: require_sha256(identities[name], f"benchmark identity {name}")
        for name in PLAN_IDENTITIES
    }
    if parsed_identities["model"] != PINNED_MODEL_IDENTITY:
        raise ReferenceFailure(
            "benchmark plan does not bind the pinned M1 deployment bundle"
        )
    case_values = exact_array(document["cases"], 7, "benchmark plan cases")
    cases: list[PlanCase] = []
    for item in case_values:
        case = exact_object(
            item, ("id", "input_sha256", "kind", "workload_sha256"), "benchmark case"
        )
        kind = string_field(case, "kind", "benchmark case")
        if kind not in CASE_GEOMETRY:
            raise ReferenceFailure(f"benchmark plan has unsupported case kind {kind!r}")
        case_id = string_field(case, "id", "benchmark case")
        if case_id != f"{kind}.001":
            raise ReferenceFailure(
                "benchmark case ID drifted from its canonical kind.001 value"
            )
        cases.append(
            PlanCase(
                case_id=case_id,
                input_sha256=require_sha256(
                    case["input_sha256"], "benchmark case input"
                ),
                kind=kind,
                workload_sha256=require_sha256(
                    case["workload_sha256"], "benchmark case workload"
                ),
            )
        )
    if tuple(case.case_id for case in cases) != tuple(
        sorted(case.case_id for case in cases)
    ):
        raise ReferenceFailure(
            "benchmark plan cases are not uniquely sorted by case ID"
        )
    if {case.kind for case in cases} != set(CASE_KINDS):
        raise ReferenceFailure("benchmark plan case-kind roster drifted")
    return Plan(
        data=data,
        cases=tuple(cases),
        identities=parsed_identities,
        input_sha256=input_sha256,
    )


def parse_workload(
    value: Any,
    data: bytes,
    case: PlanCase,
    input_root: SecureDirectory,
) -> Workload:
    if sha256_bytes(data) != case.workload_sha256:
        raise ReferenceFailure(f"workload identity drifted for {case.case_id}")
    document = exact_object(
        value,
        (
            "case_id",
            "completion_wait_policy",
            "format",
            "input",
            "kind",
            "lanes",
            "selection",
        ),
        "qualification workload",
    )
    expect_string(document, "case_id", case.case_id, "qualification workload")
    expect_string(document, "format", WORKLOAD_FORMAT, "qualification workload")
    expect_string(document, "kind", case.kind, "qualification workload")
    completion_wait_policy = exact_object(
        document["completion_wait_policy"],
        COMPLETION_WAIT_POLICY,
        "qualification completion wait policy",
    )
    if completion_wait_policy != COMPLETION_WAIT_POLICY:
        raise ReferenceFailure("qualification completion wait policy drifted")
    rows, width, mode = CASE_GEOMETRY[case.kind]
    selection = exact_object(
        document["selection"], ("bucket", "mode", "role"), "qualification selection"
    )
    if selection != {"bucket": case.kind, "mode": mode, "role": "target-8b"}:
        raise ReferenceFailure(f"qualification selection drifted for {case.case_id}")
    lane_values = exact_array(document["lanes"], rows, "qualification workload lanes")
    lanes: list[Lane] = []
    for ordinal, item in enumerate(lane_values):
        lane = exact_object(
            item, ("active_length", "context_length"), "qualification lane"
        )
        active = integer_field(lane, "active_length", "qualification lane")
        context = integer_field(lane, "context_length", "qualification lane")
        expected = (1, 8191) if mode == "decode" else (width, 0)
        if (active, context) != expected:
            raise ReferenceFailure(
                f"qualification lane {ordinal} geometry drifted for {case.case_id}"
            )
        lanes.append(Lane(active_length=active, context_length=context))
    payload = exact_object(
        document["input"],
        ("bytes", "encoding", "path", "sha256"),
        "qualification input",
    )
    expect_integer(payload, "bytes", rows * width * TOKEN_BYTES, "qualification input")
    expect_string(payload, "encoding", "u32-le", "qualification input")
    expected_path = f"{case.case_id}.tokens.u32le"
    expect_string(payload, "path", expected_path, "qualification input")
    input_sha256 = require_sha256(payload["sha256"], "qualification input identity")
    if input_sha256 != case.input_sha256:
        raise ReferenceFailure(
            f"qualification input identity drifted for {case.case_id}"
        )
    with input_root.open_file(
        expected_path, f"qualification input {case.case_id}"
    ) as source:
        token_bytes = source.read(exact=rows * width * TOKEN_BYTES)
    if sha256_bytes(token_bytes) != case.input_sha256:
        raise ReferenceFailure(
            f"qualification input payload drifted for {case.case_id}"
        )
    unpacked = struct.unpack(f"<{rows * width}I", token_bytes)
    if any(token >= INPUT_VOCABULARY_SIZE for token in unpacked):
        raise ReferenceFailure(
            "qualification input token is outside the base vocabulary"
        )
    tokens = tuple(
        tuple(unpacked[row * width : (row + 1) * width]) for row in range(rows)
    )
    return Workload(data=data, case=case, lanes=tuple(lanes), tokens=tokens)


def _validate_capture_execution(
    value: Any, plan: Plan, workload: Workload, dispatch_generation: int
) -> None:
    rows, _, mode = CASE_GEOMETRY[workload.case.kind]
    if mode == "prefill":
        execution = exact_object(
            value,
            ("dispatch_generation", "epoch", "mode", "round_count"),
            "capture execution",
        )
        expect_integer(
            execution, "dispatch_generation", dispatch_generation, "capture execution"
        )
        integer_field(execution, "epoch", "capture execution")
        expect_string(execution, "mode", "one-shot-prefill", "capture execution")
        expect_integer(execution, "round_count", 1, "capture execution")
        return
    execution = exact_object(
        value,
        (
            "context_plan_sha256",
            "declared_workload_binding_sha256",
            "first_dispatch_generation",
            "first_epoch",
            "mode",
            "ordered_lane_bindings",
            "round_count",
            "round_history_sha256",
            "terminal_dispatch_generation",
            "terminal_epoch",
            "terminal_ordinal",
        ),
        "capture execution",
    )
    for key in (
        "context_plan_sha256",
        "declared_workload_binding_sha256",
        "round_history_sha256",
    ):
        require_sha256(execution[key], f"capture execution {key}")
    expected_plan = plan.identities[f"dispatch-graph-{workload.case.kind}"]
    if execution["context_plan_sha256"] != expected_plan:
        raise ReferenceFailure(
            "capture context plan differs from the selected plan identity"
        )
    integer_field(execution, "first_dispatch_generation", "capture execution")
    integer_field(execution, "first_epoch", "capture execution")
    expect_string(execution, "mode", "teacher-forced-c8192", "capture execution")
    expect_integer(execution, "round_count", 8192, "capture execution")
    expect_integer(
        execution,
        "terminal_dispatch_generation",
        dispatch_generation,
        "capture execution",
    )
    integer_field(execution, "terminal_epoch", "capture execution")
    expect_integer(execution, "terminal_ordinal", 8191, "capture execution")
    bindings = exact_array(
        execution["ordered_lane_bindings"], rows, "capture lane bindings"
    )
    for ordinal, item in enumerate(bindings):
        binding = exact_object(
            item,
            ("lane_identity_sha256", "lane_ordinal", "token_sequence_identity_sha256"),
            "capture lane binding",
        )
        require_sha256(binding["lane_identity_sha256"], "capture lane identity")
        require_sha256(
            binding["token_sequence_identity_sha256"], "capture token sequence"
        )
        expect_integer(binding, "lane_ordinal", ordinal, "capture lane binding")


def parse_capture_transcript(
    value: Any,
    data: bytes,
    plan: Plan,
    workload: Workload,
    gpu_unique_id: int,
) -> CaptureTranscript:
    document = exact_object(
        value,
        (
            "authority",
            "benchmark_executable_sha256",
            "benchmark_protocol_sha256",
            "case_id",
            "compact_sha256",
            "device_identity_sha256",
            "dispatch_generation",
            "environment_sha256",
            "execution",
            "format",
            "gpu_unique_id",
            "input_sha256",
            "kernel_artifact_manifest_sha256",
            "kind",
            "logits_row_sha256",
            "logits_sha256",
            "nonclaim",
            "plan_sha256",
            "program_catalog_sha256",
            "runner_declaration_sha256",
            "selection",
            "status",
            "target",
            "tokens_sha256",
            "workload_sha256",
        ),
        "Ferric capture transcript",
    )
    expect_string(
        document,
        "authority",
        "observed-target-only-qualification-capture",
        "Ferric capture transcript",
    )
    bindings = {
        "benchmark_executable_sha256": plan.identities["benchmark-executable"],
        "benchmark_protocol_sha256": plan.identities["benchmark-protocol"],
        "case_id": workload.case.case_id,
        "environment_sha256": plan.identities["environment"],
        "format": CAPTURE_FORMAT,
        "input_sha256": workload.case.input_sha256,
        "kind": workload.case.kind,
        "nonclaim": CAPTURE_NONCLAIM,
        "plan_sha256": plan.sha256,
        "runner_declaration_sha256": plan.identities["generated-plan"],
        "status": "OBSERVED",
        "target": TARGET,
        "workload_sha256": workload.case.workload_sha256,
    }
    for key, expected in bindings.items():
        expect_string(document, key, expected, "Ferric capture transcript")
    for key in (
        "compact_sha256",
        "device_identity_sha256",
        "kernel_artifact_manifest_sha256",
        "logits_sha256",
        "program_catalog_sha256",
        "runner_declaration_sha256",
        "tokens_sha256",
    ):
        require_sha256(document[key], f"Ferric capture {key}")
    expect_integer(
        document, "gpu_unique_id", gpu_unique_id, "Ferric capture transcript"
    )
    dispatch_generation = integer_field(
        document, "dispatch_generation", "Ferric capture transcript"
    )
    rows, _, mode = CASE_GEOMETRY[workload.case.kind]
    selection = exact_object(
        document["selection"], ("bucket", "mode", "role"), "capture selection"
    )
    if selection != {"bucket": workload.case.kind, "mode": mode, "role": "target-8b"}:
        raise ReferenceFailure("Ferric capture selection drifted")
    row_hashes = exact_array(
        document["logits_row_sha256"], rows, "capture logit row hashes"
    )
    for row_hash in row_hashes:
        require_sha256(row_hash, "capture logit row identity")
    _validate_capture_execution(
        document["execution"], plan, workload, dispatch_generation
    )
    return CaptureTranscript(
        data=data,
        sha256=sha256_bytes(data),
        case=workload.case,
    )


def _validate_payload_manifest(
    value: Any,
    *,
    expected_bytes: int,
    expected_encoding: str,
    expected_path: str,
    description: str,
) -> str:
    payload = exact_object(value, ("bytes", "encoding", "path", "sha256"), description)
    expect_integer(payload, "bytes", expected_bytes, description)
    expect_string(payload, "encoding", expected_encoding, description)
    expect_string(payload, "path", expected_path, description)
    return require_sha256(payload["sha256"], f"{description} identity")


def validate_ferric_output(
    value: Any,
    plan: Plan,
    workload: Workload,
    transcript: CaptureTranscript,
    capture_root: SecureDirectory,
) -> None:
    document = exact_object(
        value,
        (
            "authority",
            "case_id",
            "environment_sha256",
            "format",
            "input_sha256",
            "kind",
            "logits",
            "plan_sha256",
            "producer",
            "producer_sha256",
            "protocol_sha256",
            "runner_transcript_sha256",
            "shape",
            "tokens",
            "workload_sha256",
        ),
        "Ferric output manifest",
    )
    expected_fields = {
        "authority": "externally-collected-model-output-only",
        "case_id": workload.case.case_id,
        "environment_sha256": plan.identities["environment"],
        "format": OUTPUT_FORMAT,
        "input_sha256": workload.case.input_sha256,
        "kind": workload.case.kind,
        "plan_sha256": plan.sha256,
        "producer": "ferric",
        "producer_sha256": plan.identities["benchmark-executable"],
        "protocol_sha256": plan.identities["benchmark-protocol"],
        "runner_transcript_sha256": transcript.sha256,
        "workload_sha256": workload.case.workload_sha256,
    }
    for key, expected in expected_fields.items():
        expect_string(document, key, expected, "Ferric output manifest")
    rows, _, _ = CASE_GEOMETRY[workload.case.kind]
    shape = exact_object(
        document["shape"], ("rows", "vocabulary_size"), "Ferric output shape"
    )
    expect_integer(shape, "rows", rows, "Ferric output shape")
    expect_integer(shape, "vocabulary_size", VOCABULARY_SIZE, "Ferric output shape")
    logits_bytes = rows * VOCABULARY_SIZE * BF16_BYTES
    tokens_bytes = rows * TOKEN_BYTES
    logits_sha = _validate_payload_manifest(
        document["logits"],
        expected_bytes=logits_bytes,
        expected_encoding="bf16-le",
        expected_path="logits.bf16le",
        description="Ferric logits payload",
    )
    tokens_sha = _validate_payload_manifest(
        document["tokens"],
        expected_bytes=tokens_bytes,
        expected_encoding="u32-le",
        expected_path="tokens.u32le",
        description="Ferric tokens payload",
    )
    with capture_root.open_file("logits.bf16le", "Ferric logits payload") as source:
        logits_data = source.read(exact=logits_bytes)
    if sha256_bytes(logits_data) != logits_sha:
        raise ReferenceFailure("Ferric logits payload SHA-256 drifted")
    with capture_root.open_file("tokens.u32le", "Ferric tokens payload") as source:
        source.digest(tokens_bytes, tokens_sha)
    runner_value = parse_canonical(transcript.data, "Ferric capture transcript")
    if (
        runner_value["logits_sha256"] != logits_sha
        or runner_value["tokens_sha256"] != tokens_sha
    ):
        raise ReferenceFailure(
            "Ferric output payload identities differ from its capture transcript"
        )
    row_bytes = VOCABULARY_SIZE * BF16_BYTES
    actual_row_hashes = [
        sha256_bytes(logits_data[offset : offset + row_bytes])
        for offset in range(0, len(logits_data), row_bytes)
    ]
    if runner_value["logits_row_sha256"] != actual_row_hashes:
        raise ReferenceFailure(
            "Ferric capture row identities differ from its logits payload"
        )


def load_capture(
    captures: SecureDirectory,
    plan: Plan,
    workload: Workload,
    gpu_unique_id: int,
) -> CaptureTranscript:
    bundle_name = f"{workload.case.kind}.capture.bundle"
    with captures.child(
        bundle_name, f"capture bundle {workload.case.case_id}"
    ) as bundle:
        if bundle.entries() != {
            "logits.bf16le",
            "output.json",
            "runner.json",
            "tokens.u32le",
        }:
            raise ReferenceFailure(
                f"capture bundle roster drifted for {workload.case.case_id}"
            )
        runner_value, runner_data, _ = bundle.read_canonical(
            "runner.json", f"Ferric runner transcript {workload.case.case_id}"
        )
        transcript = parse_capture_transcript(
            runner_value, runner_data, plan, workload, gpu_unique_id
        )
        output_value, _, _ = bundle.read_canonical(
            "output.json", f"Ferric output manifest {workload.case.case_id}"
        )
        validate_ferric_output(output_value, plan, workload, transcript, bundle)
        return transcript


def validate_bound_files(
    directory: SecureDirectory,
    files: dict[str, SecureFile],
    description: str,
) -> None:
    if directory.entries() != set(files):
        raise ReferenceFailure(f"{description} file roster changed during use")
    for name, held in files.items():
        held.validate()
        with directory.open_file(name, f"reopened {description} {name}") as reopened:
            reopened.validate()
            if reopened.identity != held.identity:
                raise ReferenceFailure(
                    f"{description} filename {name} changed inode during use"
                )


class AuthenticatedModelSource:
    def __init__(self, target: SecureDirectory, files: dict[str, SecureFile]) -> None:
        self.target = target
        self.files = files

    @property
    def transformers_path(self) -> str:
        return f"/proc/self/fd/{self.target.fd}"

    def validate(self) -> None:
        validate_bound_files(self.target, self.files, "pinned target model")

    def close(self) -> None:
        for source in self.files.values():
            source.close()
        self.files = {}
        self.target.close()

    def __enter__(self) -> AuthenticatedModelSource:
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()


def authenticate_model_source(source_root: Path) -> AuthenticatedModelSource:
    root = SecureDirectory.open(source_root, "model source root")
    try:
        if root.entries() != {"draft", "target"}:
            raise ReferenceFailure(
                "model source root must contain exactly target and draft"
            )
        target = root.child("target", "pinned target model root")
    finally:
        root.close()
    files: dict[str, SecureFile] = {}
    try:
        if target.entries() != set(MODEL_FILES):
            raise ReferenceFailure("pinned target model file roster drifted")
        for name, (expected_bytes, expected_sha256) in MODEL_FILES.items():
            source = target.open_file(name, f"pinned target model {name}")
            try:
                source.digest(expected_bytes, expected_sha256)
            except Exception:
                source.close()
                raise
            files[name] = source
        config_source = files["config.json"]
        config_data = config_source.read(exact=MODEL_FILES["config.json"][0])
        try:
            config = json.loads(config_data)
        except json.JSONDecodeError as error:
            raise ReferenceFailure(
                f"cannot parse pinned target config: {error}"
            ) from error
        required_config = {
            "architectures": ["Qwen3ForCausalLM"],
            "hidden_size": 4096,
            "model_type": "qwen3",
            "num_hidden_layers": 36,
            "torch_dtype": "bfloat16",
            "transformers_version": "4.51.0",
            "vocab_size": VOCABULARY_SIZE,
        }
        if any(
            config.get(key) != expected for key, expected in required_config.items()
        ):
            raise ReferenceFailure(
                "pinned target config is incompatible with M1 reference execution"
            )
        return AuthenticatedModelSource(target, files)
    except Exception:
        for source in files.values():
            source.close()
        target.close()
        raise


@dataclass(frozen=True)
class ModelDependencies:
    numpy: Any
    safetensors: Any
    tokenizers: Any
    torch: Any
    transformers: Any


def require_isolated_python(flags: Any = sys.flags) -> None:
    required = {
        "ignore_environment": 1,
        "isolated": 1,
        "no_user_site": 1,
        "safe_path": True,
    }
    for name, expected in required.items():
        if getattr(flags, name, None) != expected:
            raise ReferenceFailure(
                "reference execution requires VENV/bin/python -I with isolated, "
                "ignore-environment, no-user-site, and safe-path modes"
            )


def require_under_prefix(path: Path, prefix: Path, description: str) -> None:
    try:
        path.resolve(strict=True).relative_to(prefix.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise ReferenceFailure(
            f"{description} is outside the active virtual environment"
        ) from error


def require_virtual_environment(
    prefix: Path | None = None,
    base_prefix: Path | None = None,
    executable: Path | None = None,
) -> None:
    prefix = Path(sys.prefix) if prefix is None else prefix
    base_prefix = Path(sys.base_prefix) if base_prefix is None else base_prefix
    executable = Path(sys.executable) if executable is None else executable
    if prefix == base_prefix:
        raise ReferenceFailure(
            "reference execution requires a non-base virtual environment"
        )
    try:
        executable.absolute().relative_to(prefix.absolute())
    except ValueError as error:
        raise ReferenceFailure(
            "reference Python executable is outside the active virtual environment"
        ) from error


def validate_dependency_provenance(modules: dict[str, Any]) -> None:
    require_virtual_environment()
    prefix = Path(sys.prefix)
    from importlib.metadata import distribution

    distribution_names = {
        "numpy": "numpy",
        "safetensors": "safetensors",
        "tokenizers": "tokenizers",
        "torch": "torch",
        "transformers": "transformers",
        "triton-rocm": "triton-rocm",
    }
    for name, distribution_name in distribution_names.items():
        module_path = getattr(modules[name], "__file__", None)
        if not isinstance(module_path, str):
            raise ReferenceFailure(
                f"reference dependency {name} has no module provenance"
            )
        require_under_prefix(
            Path(module_path), prefix, f"reference dependency module {name}"
        )
        try:
            distribution_root = Path(distribution(distribution_name).locate_file(""))
        except Exception as error:
            raise ReferenceFailure(
                f"cannot inspect reference dependency distribution {name}: {error}"
            ) from error
        require_under_prefix(
            distribution_root, prefix, f"reference dependency distribution {name}"
        )


def validate_gpu_target(torch: Any) -> None:
    if not torch.cuda.is_available() or torch.cuda.device_count() != 1:
        raise ReferenceFailure(
            "reference execution requires exactly one visible ROCm GPU"
        )
    properties = torch.cuda.get_device_properties(0)
    architecture = getattr(properties, "gcnArchName", None)
    if not isinstance(architecture, str):
        raise ReferenceFailure("visible ROCm GPU does not report gcnArchName")
    components = architecture.split(":")
    if components[0] != "gfx942" or "xnack-" not in components[1:]:
        raise ReferenceFailure(
            "reference execution requires visible gcnArchName gfx942 with xnack-"
        )


def load_dependencies() -> ModelDependencies:
    require_isolated_python()
    if sys.version_info[:2] != (3, 12):
        raise ReferenceFailure("reference execution requires Python 3.12 exactly")
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    os.environ["HF_DATASETS_OFFLINE"] = "1"
    try:
        import numpy
        import safetensors
        import tokenizers
        import torch
        import transformers
        import triton
    except ImportError as error:
        raise ReferenceFailure(
            f"cannot import pinned reference dependency: {error}"
        ) from error
    actual = {
        "numpy": numpy.__version__,
        "safetensors": safetensors.__version__,
        "tokenizers": tokenizers.__version__,
        "torch": torch.__version__,
        "transformers": transformers.__version__,
    }
    for name, expected in DEPENDENCY_VERSIONS.items():
        if name in ("python", "triton-rocm"):
            continue
        if actual[name] != expected:
            raise ReferenceFailure(
                f"reference dependency {name} drifted: expected {expected}, found {actual[name]}"
            )
    try:
        from importlib.metadata import version

        triton_version = version("triton-rocm")
    except Exception as error:
        raise ReferenceFailure(
            f"cannot inspect pinned triton-rocm dependency: {error}"
        ) from error
    if triton_version != DEPENDENCY_VERSIONS["triton-rocm"]:
        raise ReferenceFailure(
            "reference dependency triton-rocm drifted: expected "
            f"{DEPENDENCY_VERSIONS['triton-rocm']}, found {triton_version}"
        )
    validate_dependency_provenance(
        {
            "numpy": numpy,
            "safetensors": safetensors,
            "tokenizers": tokenizers,
            "torch": torch,
            "transformers": transformers,
            "triton-rocm": triton,
        }
    )
    hip_version = getattr(torch.version, "hip", None)
    if not isinstance(hip_version, str) or not hip_version.startswith("7.2"):
        raise ReferenceFailure("pinned torch build does not report ROCm 7.2")
    validate_gpu_target(torch)
    return ModelDependencies(numpy, safetensors, tokenizers, torch, transformers)


def load_model(
    dependencies: ModelDependencies, source: AuthenticatedModelSource
) -> Any:
    torch = dependencies.torch
    model = dependencies.transformers.AutoModelForCausalLM.from_pretrained(
        source.transformers_path,
        attn_implementation="sdpa",
        local_files_only=True,
        torch_dtype=torch.bfloat16,
        trust_remote_code=False,
        use_safetensors=True,
    )
    source.validate()
    if model.__class__.__name__ != "Qwen3ForCausalLM":
        raise ReferenceFailure("transformers loaded an unexpected model class")
    required_config = {
        "hidden_size": 4096,
        "model_type": "qwen3",
        "num_hidden_layers": 36,
        "torch_dtype": torch.bfloat16,
        "vocab_size": VOCABULARY_SIZE,
    }
    for name, expected in required_config.items():
        if getattr(model.config, name, None) != expected:
            raise ReferenceFailure(f"loaded model config field {name} drifted")
    model.to(device="cuda", dtype=torch.bfloat16)
    model.eval()
    source.validate()
    return model


def bf16_argmax(row: bytes) -> int:
    if len(row) != VOCABULARY_SIZE * BF16_BYTES:
        raise ReferenceFailure("BF16 row has an invalid vocabulary extent")
    best_key: int | None = None
    best_token = 0
    for token, (bits,) in enumerate(struct.iter_unpack("<H", row)):
        if bits & 0x7F80 == 0x7F80:
            raise ReferenceFailure(
                f"BF16 row contains a nonfinite value at token {token}"
            )
        magnitude = bits & 0x7FFF
        key = 0x8000 - magnitude if bits & 0x8000 else 0x8000 + magnitude
        if best_key is None or key > best_key:
            best_key = key
            best_token = token
    return best_token


def serialize_bf16_tensor(row: Any, torch: Any) -> bytes:
    if row.dtype != torch.bfloat16:
        raise ReferenceFailure("projected reference logits are not BF16")
    if row.ndim != 1 or row.numel() != VOCABULARY_SIZE:
        raise ReferenceFailure("projected reference logit shape drifted")
    if not bool(torch.isfinite(row).all().item()):
        raise ReferenceFailure("projected reference logits contain a nonfinite value")
    if sys.byteorder != "little":
        raise ReferenceFailure("raw BF16 publication requires a little-endian host")
    cpu_row = row.detach().contiguous().cpu()
    data = bytes(cpu_row.view(torch.uint8).tolist())
    if len(data) != VOCABULARY_SIZE * BF16_BYTES:
        raise ReferenceFailure("serialized reference logit extent drifted")
    # Revalidate the exact bytes that will be published, not a wider tensor view.
    bf16_argmax(data)
    return data


def execute_workload(model: Any, torch: Any, workload: Workload) -> tuple[bytes, bytes]:
    logits_rows: list[bytes] = []
    selected_tokens: list[int] = []
    parameter = next(model.parameters(), None)
    if (
        parameter is None
        or parameter.device.type != "cuda"
        or parameter.dtype != torch.bfloat16
    ):
        raise ReferenceFailure(
            "reference model is not resident as BF16 on the visible GPU"
        )
    with torch.inference_mode():
        for lane_ordinal, lane_tokens in enumerate(workload.tokens):
            input_ids = torch.tensor(
                [lane_tokens], dtype=torch.long, device=parameter.device
            )
            attention_mask = torch.ones_like(input_ids)
            result = model.model(
                input_ids=input_ids,
                attention_mask=attention_mask,
                return_dict=True,
                use_cache=False,
            )
            hidden = result.last_hidden_state
            if (
                hidden.dtype != torch.bfloat16
                or hidden.ndim != 3
                or tuple(hidden.shape[:2]) != (1, len(lane_tokens))
                or hidden.shape[2] != 4096
            ):
                raise ReferenceFailure(
                    f"base-model hidden-state contract drifted at lane {lane_ordinal}"
                )
            final_hidden = hidden[:, -1:, :]
            projected = model.lm_head(final_hidden).to(dtype=torch.bfloat16)
            if tuple(projected.shape) != (1, 1, VOCABULARY_SIZE):
                raise ReferenceFailure(
                    f"final-position projection shape drifted at lane {lane_ordinal}"
                )
            row = serialize_bf16_tensor(projected.reshape(VOCABULARY_SIZE), torch)
            logits_rows.append(row)
            selected_tokens.append(bf16_argmax(row))
            del attention_mask, final_hidden, hidden, input_ids, projected, result
    torch.cuda.synchronize(parameter.device)
    logits = b"".join(logits_rows)
    tokens = struct.pack(f"<{len(selected_tokens)}I", *selected_tokens)
    rows, _, _ = CASE_GEOMETRY[workload.case.kind]
    if len(logits) != rows * VOCABULARY_SIZE * BF16_BYTES:
        raise ReferenceFailure("reference logits payload extent drifted")
    if len(tokens) != rows * TOKEN_BYTES:
        raise ReferenceFailure("reference token payload extent drifted")
    return logits, tokens


def reference_manifest(
    plan: Plan,
    workload: Workload,
    transcript: CaptureTranscript,
    logits: bytes,
    tokens: bytes,
) -> bytes:
    rows, _, _ = CASE_GEOMETRY[workload.case.kind]
    if len(logits) != rows * VOCABULARY_SIZE * BF16_BYTES:
        raise ReferenceFailure("reference manifest received invalid logit extent")
    if len(tokens) != rows * TOKEN_BYTES:
        raise ReferenceFailure("reference manifest received invalid token extent")
    value = {
        "authority": "externally-collected-model-output-only",
        "case_id": workload.case.case_id,
        "environment_sha256": plan.identities["environment"],
        "format": OUTPUT_FORMAT,
        "input_sha256": workload.case.input_sha256,
        "kind": workload.case.kind,
        "logits": {
            "bytes": len(logits),
            "encoding": "bf16-le",
            "path": "logits.bf16le",
            "sha256": sha256_bytes(logits),
        },
        "plan_sha256": plan.sha256,
        "producer": "reference",
        "producer_sha256": plan.identities["reference-implementation"],
        "protocol_sha256": plan.identities["reference-protocol"],
        "runner_transcript_sha256": transcript.sha256,
        "shape": {"rows": rows, "vocabulary_size": VOCABULARY_SIZE},
        "tokens": {
            "bytes": len(tokens),
            "encoding": "u32-le",
            "path": "tokens.u32le",
            "sha256": sha256_bytes(tokens),
        },
        "workload_sha256": workload.case.workload_sha256,
    }
    return canonical_bytes(value)


_LIBC = ctypes.CDLL(None, use_errno=True)
_RENAME_NOREPLACE = 1
try:
    _RENAMEAT2 = _LIBC.renameat2
except AttributeError as error:
    raise RuntimeError(
        "Ferric M1 reference publication requires Linux renameat2"
    ) from error
_RENAMEAT2.argtypes = [
    ctypes.c_int,
    ctypes.c_char_p,
    ctypes.c_int,
    ctypes.c_char_p,
    ctypes.c_uint,
]
_RENAMEAT2.restype = ctypes.c_int


def rename_noreplace(parent_fd: int, source: str, destination: str) -> None:
    result = _RENAMEAT2(
        parent_fd,
        os.fsencode(source),
        parent_fd,
        os.fsencode(destination),
        _RENAME_NOREPLACE,
    )
    if result != 0:
        error_number = ctypes.get_errno()
        raise OSError(error_number, os.strerror(error_number), destination)


def write_new(parent_fd: int, name: str, data: bytes, description: str) -> None:
    require_relative_name(name, description)
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC
    fd = os.open(name, flags, 0o600, dir_fd=parent_fd)
    try:
        view = memoryview(data)
        written = 0
        while written < len(view):
            count = os.write(fd, view[written:])
            if count == 0:
                raise ReferenceFailure(f"staged {description} write made no progress")
            written += count
        os.fsync(fd)
    finally:
        os.close(fd)


class OutputPublisher:
    def __init__(self, output: Path) -> None:
        self.output_name = output.name
        require_relative_name(self.output_name, "reference output root")
        parent_path = output.parent if os.fspath(output.parent) else Path(".")
        self.parent = SecureDirectory.open(parent_path, "reference output parent")
        self.staging_name = (
            f".{self.output_name}.staging.{os.getpid()}.{secrets.token_hex(8)}"
        )
        os.mkdir(self.staging_name, 0o700, dir_fd=self.parent.fd)
        try:
            self.staging = self.parent.child(
                self.staging_name, "reference staging root"
            )
        except Exception:
            os.rmdir(self.staging_name, dir_fd=self.parent.fd)
            self.parent.close()
            raise
        self.bundle_names: list[str] = []
        self.expected_files: dict[str, dict[str, tuple[int, str]]] = {}
        self.armed = True

    def add(self, bundle: ReferenceBundle) -> None:
        bundle_name = f"{bundle.case.kind}.reference.bundle"
        if bundle_name in self.bundle_names:
            raise ReferenceFailure("reference publisher received a duplicate case")
        os.mkdir(bundle_name, 0o700, dir_fd=self.staging.fd)
        self.bundle_names.append(bundle_name)
        self.expected_files[bundle_name] = {
            "logits.bf16le": (len(bundle.logits), sha256_bytes(bundle.logits)),
            "tokens.u32le": (len(bundle.tokens), sha256_bytes(bundle.tokens)),
            "runner.json": (len(bundle.runner), sha256_bytes(bundle.runner)),
            "output.json": (len(bundle.manifest), sha256_bytes(bundle.manifest)),
        }
        with self.staging.child(bundle_name, "staged reference bundle") as case_root:
            write_new(case_root.fd, "logits.bf16le", bundle.logits, "reference logits")
            write_new(case_root.fd, "tokens.u32le", bundle.tokens, "reference tokens")
            write_new(
                case_root.fd, "runner.json", bundle.runner, "Ferric runner transcript"
            )
            write_new(
                case_root.fd,
                "output.json",
                bundle.manifest,
                "reference output manifest",
            )
            os.fsync(case_root.fd)

    def validate_staging(self) -> None:
        if self.staging.entries() != set(self.bundle_names):
            raise ReferenceFailure("staged reference bundle roster drifted")
        identities: set[FileIdentity] = set()
        for bundle_name in self.bundle_names:
            expected = self.expected_files.get(bundle_name)
            if expected is None:
                raise ReferenceFailure(
                    f"staged reference bundle {bundle_name} is incomplete"
                )
            with self.staging.child(
                bundle_name, "staged reference bundle"
            ) as case_root:
                if case_root.entries() != set(expected):
                    raise ReferenceFailure(
                        f"staged reference payload roster drifted for {bundle_name}"
                    )
                for name, (expected_bytes, expected_sha256) in expected.items():
                    with case_root.open_file(
                        name, f"staged reference {bundle_name}/{name}"
                    ) as source:
                        source.digest(expected_bytes, expected_sha256)
                        if source.identity in identities:
                            raise ReferenceFailure(
                                "staged reference payload inode was reused"
                            )
                        identities.add(source.identity)

    def publish(self) -> None:
        if tuple(self.bundle_names) != tuple(
            sorted(f"{kind}.reference.bundle" for kind in CASE_KINDS)
        ):
            raise ReferenceFailure(
                "reference publisher does not contain the exact sorted case roster"
            )
        self.validate_staging()
        os.fsync(self.staging.fd)
        with self.parent.child(
            self.staging_name, "rebound reference staging root"
        ) as rebound:
            if rebound.identity != self.staging.identity:
                raise ReferenceFailure(
                    "reference staging name no longer resolves to the held inode"
                )
            if rebound.entries() != set(self.bundle_names):
                raise ReferenceFailure("rebound reference staging roster drifted")
            rename_noreplace(self.parent.fd, self.staging_name, self.output_name)
        self.armed = False
        os.fsync(self.parent.fd)

    def close(self) -> None:
        if self.armed:
            for bundle_name in reversed(self.bundle_names):
                try:
                    case_fd = os.open(
                        bundle_name,
                        os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                        dir_fd=self.staging.fd,
                    )
                except OSError:
                    continue
                try:
                    # This path is only reached for a staging tree created by this process.
                    for name in (
                        "logits.bf16le",
                        "tokens.u32le",
                        "runner.json",
                        "output.json",
                    ):
                        try:
                            os.unlink(name, dir_fd=case_fd)
                        except FileNotFoundError:
                            pass
                finally:
                    os.close(case_fd)
                try:
                    os.rmdir(bundle_name, dir_fd=self.staging.fd)
                except OSError:
                    pass
        self.staging.close()
        if self.armed:
            try:
                os.rmdir(self.staging_name, dir_fd=self.parent.fd)
            except OSError:
                pass
        self.parent.close()

    def __enter__(self) -> OutputPublisher:
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()


def authenticate_reference_artifacts(
    implementation_path: Path,
    protocol_path: Path,
) -> tuple[str, str]:
    implementation_parent, implementation_name = open_parent(
        implementation_path, "reference implementation manifest"
    )
    with implementation_parent:
        implementation_value, implementation_data, _ = (
            implementation_parent.read_canonical(
                implementation_name, "reference implementation manifest"
            )
        )
        implementation_sha256 = parse_implementation_manifest(
            implementation_value,
            implementation_data,
            implementation_parent,
            Path(__file__),
        )
    protocol_value, protocol_data, _ = read_canonical_path(
        protocol_path, "reference protocol"
    )
    validate_protocol(protocol_value)
    return implementation_sha256, sha256_bytes(protocol_data)


def expected_input_entries(plan: Plan) -> set[str]:
    entries = {
        "acceptance-policy.json",
        "benchmark-input.json",
        "closure.json",
        "environment.json",
        "plan.json",
        "roster.json",
    }
    for case in plan.cases:
        entries.add(f"{case.case_id}.tokens.u32le")
        entries.add(f"{case.case_id}.workload.json")
    return entries


def plan_case_values(plan: Plan) -> list[dict[str, str]]:
    return [
        {
            "id": case.case_id,
            "input_sha256": case.input_sha256,
            "kind": case.kind,
            "workload_sha256": case.workload_sha256,
        }
        for case in plan.cases
    ]


def validate_acceptance_policy(value: Any) -> None:
    policy = exact_object(
        value,
        (
            "authority",
            "cases",
            "finite_logits_required",
            "format",
            "logit_metric",
            "nonclaim",
            "obligation_id",
            "path_id",
            "suite",
            "target",
            "token_metric",
            "token_selection",
        ),
        "differential acceptance policy",
    )
    expected = {
        "authority": "externally-admitted-differential-threshold-policy-only",
        "format": ACCEPTANCE_POLICY_FORMAT,
        "logit_metric": "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
        "nonclaim": ACCEPTANCE_POLICY_NONCLAIM,
        "obligation_id": "m1.r29",
        "path_id": "differential-bench",
        "suite": "differential",
        "target": TARGET,
        "token_metric": "ferric-reference-greedy-token-mismatch-count",
        "token_selection": "lowest-token-id-bf16-argmax",
    }
    for key, expected_value in expected.items():
        expect_string(policy, key, expected_value, "differential acceptance policy")
    if policy["finite_logits_required"] is not True:
        raise ReferenceFailure(
            "differential acceptance policy must require finite logits"
        )
    cases = exact_array(policy["cases"], len(CASE_KINDS), "acceptance policy cases")
    observed: list[str] = []
    for item in cases:
        case = exact_object(
            item,
            ("kind", "maximum_logit_ulp_error", "maximum_token_mismatches"),
            "acceptance policy case",
        )
        kind = string_field(case, "kind", "acceptance policy case")
        if kind not in CASE_GEOMETRY:
            raise ReferenceFailure(f"acceptance policy names unsupported kind {kind!r}")
        integer_field(case, "maximum_logit_ulp_error", "acceptance policy case")
        mismatch = integer_field(
            case, "maximum_token_mismatches", "acceptance policy case"
        )
        if mismatch > CASE_GEOMETRY[kind][0]:
            raise ReferenceFailure(
                "acceptance policy token threshold exceeds row count"
            )
        observed.append(kind)
    if tuple(observed) != CASE_KINDS:
        raise ReferenceFailure(
            "acceptance policy cases are not the exact sorted kind roster"
        )


def validate_common_documents(inputs: SecureDirectory, plan: Plan) -> int:
    benchmark, benchmark_data, _ = inputs.read_canonical(
        "benchmark-input.json", "benchmark input"
    )
    if sha256_bytes(benchmark_data) != plan.input_sha256:
        raise ReferenceFailure("benchmark input identity differs from the plan")
    benchmark = exact_object(
        benchmark,
        ("cases", "format", "identities", "suite", "target"),
        "benchmark input",
    )
    expect_string(benchmark, "format", BENCHMARK_INPUT_FORMAT, "benchmark input")
    expect_string(benchmark, "suite", "differential", "benchmark input")
    expect_string(benchmark, "target", TARGET, "benchmark input")
    if benchmark["cases"] != plan_case_values(plan):
        raise ReferenceFailure("benchmark input cases differ from the plan")
    identities = exact_object(
        benchmark["identities"], PLAN_IDENTITIES, "benchmark identities"
    )
    if identities != plan.identities:
        raise ReferenceFailure("benchmark input identities differ from the plan")

    roster, roster_data, _ = inputs.read_canonical("roster.json", "workload roster")
    if sha256_bytes(roster_data) != plan.identities["workload-roster"]:
        raise ReferenceFailure("workload roster identity differs from the plan")
    roster = exact_object(roster, ("cases", "format", "suite"), "workload roster")
    expect_string(roster, "format", ROSTER_FORMAT, "workload roster")
    expect_string(roster, "suite", "differential", "workload roster")
    if roster["cases"] != plan_case_values(plan):
        raise ReferenceFailure("workload roster cases differ from the plan")

    environment, environment_data, _ = inputs.read_canonical(
        "environment.json", "qualification environment"
    )
    if sha256_bytes(environment_data) != plan.identities["environment"]:
        raise ReferenceFailure(
            "qualification environment identity differs from the plan"
        )
    environment = exact_object(
        environment, ("format", "gpu_unique_id", "target"), "qualification environment"
    )
    expect_string(
        environment, "format", ENVIRONMENT_FORMAT, "qualification environment"
    )
    expect_string(environment, "target", TARGET, "qualification environment")
    gpu_unique_id = integer_field(
        environment, "gpu_unique_id", "qualification environment"
    )
    if gpu_unique_id == 0:
        raise ReferenceFailure(
            "qualification environment GPU unique ID must be nonzero"
        )

    policy, policy_data, _ = inputs.read_canonical(
        "acceptance-policy.json", "differential acceptance policy"
    )
    if sha256_bytes(policy_data) != plan.identities["differential-acceptance-policy"]:
        raise ReferenceFailure("acceptance policy identity differs from the plan")
    validate_acceptance_policy(policy)

    closure, _, _ = inputs.read_canonical("closure.json", "qualification closure")
    closure_keys = (
        "compiler",
        "compiler_configuration",
        "fe2o3_source",
        "ferric_source",
        "format",
        "kernel_abi_catalog",
        "kernel_proof_set",
        "qualification_protocol",
        "runtime_abi",
        "runtime_contract",
        "target_contract",
        "tcb_report",
        "validator_registry",
    )
    closure = exact_object(closure, closure_keys, "qualification closure")
    expect_string(
        closure, "format", QUALIFICATION_CLOSURE_FORMAT, "qualification closure"
    )
    for key in closure_keys:
        if key != "format":
            require_sha256(closure[key], f"qualification closure {key}")
    for key, expected_identity in {
        "fe2o3_source": plan.identities["fe2o3-source-closure"],
        "ferric_source": plan.identities["ferric-source-closure"],
        "qualification_protocol": plan.identities["benchmark-protocol"],
    }.items():
        if closure[key] != expected_identity:
            raise ReferenceFailure(f"qualification closure {key} differs from the plan")
    return gpu_unique_id


def load_workloads(input_path: Path, plan: Plan) -> tuple[tuple[Workload, ...], int]:
    with SecureDirectory.open(input_path, "qualification input bundle") as inputs:
        if inputs.entries() != expected_input_entries(plan):
            raise ReferenceFailure("qualification input bundle file roster drifted")
        _, bundled_plan, _ = inputs.read_canonical(
            "plan.json", "bundled benchmark plan"
        )
        if bundled_plan != plan.data:
            raise ReferenceFailure(
                "bundled benchmark plan differs from the supplied plan"
            )
        gpu_unique_id = validate_common_documents(inputs, plan)
        workloads: list[Workload] = []
        for case in plan.cases:
            value, data, _ = inputs.read_canonical(
                f"{case.case_id}.workload.json",
                f"qualification workload {case.case_id}",
            )
            workloads.append(parse_workload(value, data, case, inputs))
        return tuple(workloads), gpu_unique_id


def run(arguments: list[str]) -> None:
    require_isolated_python()
    require_virtual_environment()
    if len(arguments) != 7:
        raise ReferenceFailure(
            "usage: run.py IMPLEMENTATION-MANIFEST PROTOCOL PLAN INPUT-BUNDLE "
            "MODEL-SOURCE FERRIC-CAPTURE-ROOT OUTPUT-ROOT"
        )
    (
        implementation_path,
        protocol_path,
        plan_path,
        input_path,
        model_path,
        capture_path,
        output,
    ) = map(Path, arguments)
    implementation_sha256, protocol_sha256 = authenticate_reference_artifacts(
        implementation_path, protocol_path
    )
    plan_value, plan_data, _ = read_canonical_path(plan_path, "benchmark plan")
    plan = parse_plan(plan_value, plan_data)
    if plan.identities["reference-implementation"] != implementation_sha256:
        raise ReferenceFailure(
            "benchmark plan reference implementation identity drifted"
        )
    if plan.identities["reference-protocol"] != protocol_sha256:
        raise ReferenceFailure("benchmark plan reference protocol identity drifted")
    workloads, gpu_unique_id = load_workloads(input_path, plan)
    with SecureDirectory.open(capture_path, "Ferric capture root") as captures:
        if captures.entries() != {f"{case.kind}.capture.bundle" for case in plan.cases}:
            raise ReferenceFailure("Ferric capture root case roster drifted")
        transcripts = tuple(
            load_capture(captures, plan, workload, gpu_unique_id)
            for workload in workloads
        )

    bundles: list[ReferenceBundle] = []
    with authenticate_model_source(model_path) as model_source:
        dependencies = load_dependencies()
        model = load_model(dependencies, model_source)
        for workload, transcript in zip(workloads, transcripts, strict=True):
            logits, tokens = execute_workload(model, dependencies.torch, workload)
            repeated_logits, repeated_tokens = execute_workload(
                model, dependencies.torch, workload
            )
            if repeated_logits != logits or repeated_tokens != tokens:
                raise ReferenceFailure(
                    f"reference execution was not byte-deterministic for {workload.case.case_id}"
                )
            bundles.append(
                ReferenceBundle(
                    case=workload.case,
                    logits=logits,
                    tokens=tokens,
                    runner=transcript.data,
                    manifest=reference_manifest(
                        plan, workload, transcript, logits, tokens
                    ),
                )
            )
        model_source.validate()

    repeated_implementation, repeated_protocol = authenticate_reference_artifacts(
        implementation_path, protocol_path
    )
    if (repeated_implementation, repeated_protocol) != (
        implementation_sha256,
        protocol_sha256,
    ):
        raise ReferenceFailure(
            "reference implementation or protocol changed during execution"
        )
    with OutputPublisher(output) as publisher:
        for bundle in bundles:
            publisher.add(bundle)
        publisher.publish()
    print(f"output={output}")
    print(f"plan_sha256={plan.sha256}")
    print("status=REFERENCE_OUTPUTS_PUBLISHED")


def main() -> int:
    try:
        run(sys.argv[1:])
    except (ReferenceFailure, OSError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
