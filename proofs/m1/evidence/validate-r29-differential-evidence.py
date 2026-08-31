#!/usr/bin/env python3
"""Authenticate one exact M1 r29 differential intake without closing r29."""

from __future__ import annotations

import ctypes
import errno
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, NoReturn


TARGET = "gfx942:xnack-"
OBLIGATION_ID = "m1.r29"
PATH_ID = "differential-bench"
SUITE = "differential"
VOCABULARY_SIZE = 151_936
INTAKE_FORMAT = "FERRIC-M1-R29-DIFFERENTIAL-EVIDENCE-INTAKE-V1"
REVIEW_FORMAT = "FERRIC-M1-R29-DIFFERENTIAL-POLICY-REVIEW-V1"
ROSTER_FORMAT = "FERRIC-M1-R29-DIFFERENTIAL-EVIDENCE-ROSTER-V1"
REPORT_FORMAT = "FERRIC-M1-R29-DIFFERENTIAL-EVIDENCE-REPORT-V1"
PLAN_FORMAT = "FERRIC-M1-BENCHMARK-PLAN-V1"
POLICY_FORMAT = "FERRIC-M1-DIFFERENTIAL-ACCEPTANCE-POLICY-V1"
PAIRS_FORMAT = "FERRIC-M1-DIFFERENTIAL-PAIRS-V2"
OUTPUT_FORMAT = "FERRIC-M1-DIFFERENTIAL-OUTPUT-V1"
CAPTURE_FORMAT = "FERRIC-M1-QUALIFICATION-CAPTURE-V2"
RECORDS_FORMAT = "FERRIC-M1-BENCHMARK-RECORDS-V1"
RAW_FORMAT = "FERRIC-M1-DIFFERENTIAL-RAW-RECORD-V1"
ACCEPTANCE_FORMAT = "FERRIC-M1-DIFFERENTIAL-ACCEPTANCE-RESULT-V1"
INTAKE_AUTHORITY = "externally-assembled-r29-differential-intake-only"
REVIEW_AUTHORITY = "externally-declared-policy-review-only"
ROSTER_AUTHORITY = "authenticated-r29-differential-artifact-roster-only"
REPORT_AUTHORITY = "r29-differential-intake-authentication-only"
NONCLAIM = (
    "This report authenticates the exact identity-bound r29 differential intake "
    "and its declared external policy review only. It does not independently "
    "recompute logits, tokens, or comparisons; attest observation truth, hardware "
    "behavior, reviewer independence, source provenance, compiler or runtime "
    "correctness; create qualification evidence; or close m1.r29 or M1."
)
INTAKE_NONCLAIM = (
    "External r29 differential intake declaration only. Ferric authenticates its "
    "bound bytes but does not attest observation truth, reviewer independence, "
    "hardware behavior, qualification, or m1.r29 closure."
)
REVIEW_NONCLAIM = (
    "External policy-review declaration only. Ferric binds this declaration but "
    "does not establish reviewer identity, independence, authority, or correctness."
)
DIFFERENTIAL_NONCLAIM = (
    "Structural acceptance authenticates externally collected target-only "
    "differential records only. It does not validate a logit tolerance, prove "
    "token equality, establish numerical or hardware correctness, qualify "
    "performance, or close m1.r29."
)
POLICY_NONCLAIM = (
    "This artifact supplies plan-admitted differential thresholds only. It does "
    "not establish independent review, numerical correctness, hardware correctness, "
    "qualification authority, or close m1.r29."
)
ACCEPTANCE_NONCLAIM = (
    "This result authenticates exact target-only differential comparisons against "
    "one plan-admitted threshold policy only. It does not establish an independently "
    "reviewed threshold, prove operator or graph refinement, establish hardware "
    "correctness, grant qualification authority, or close m1.r29."
)
CAPTURE_NONCLAIM = (
    "Observed bytes only; this transcript does not establish a reference comparison, "
    "tolerance, numerical correctness, hardware correctness, performance, "
    "qualification, or m1.r29 closure."
)

CASE_KINDS = (
    "decode-s1-c8192",
    "decode-s32-c8192",
    "decode-s8-c8192",
    "prefill-s1-t128",
    "prefill-s1-t2048",
    "prefill-s1-t512",
    "prefill-s8-t128",
)
ROWS = {
    "decode-s1-c8192": 1,
    "decode-s32-c8192": 32,
    "decode-s8-c8192": 8,
    "prefill-s1-t128": 1,
    "prefill-s1-t2048": 1,
    "prefill-s1-t512": 1,
    "prefill-s8-t128": 8,
}
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
PLAN_IDENTITIES = tuple(
    sorted(
        COMMON_IDENTITIES
        + tuple(f"dispatch-graph-{kind}" for kind in CASE_KINDS)
        + (
            "differential-acceptance-policy",
            "reference-implementation",
            "reference-protocol",
        )
    )
)
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {"source.fe2o3": "fe2o3", "source.ferric": "ferric"}
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
TOOLCHAIN_KEYS = {
    "benchmark_executable_sha256",
    "benchmark_protocol_sha256",
    "compiler_configuration_sha256",
    "compiler_sha256",
    "qualification_protocol_sha256",
    "reference_implementation_sha256",
    "reference_protocol_sha256",
    "runtime_abi_sha256",
    "runtime_contract_sha256",
    "target_contract_sha256",
}
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_TEXT = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:/@+ -]{0,255}\Z")
MAX_JSON_BYTES = 16 * 1024 * 1024
MAX_PAYLOAD_BYTES = 16 * 1024 * 1024
MAX_U64 = (1 << 64) - 1
RENAME_NOREPLACE = 1


def fail(message: str) -> NoReturn:
    raise SystemExit(f"r29 differential evidence: {message}")


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    try:
        return (
            json.dumps(
                value, allow_nan=False, ensure_ascii=True, indent=2, sort_keys=True
            )
            + "\n"
        ).encode("ascii")
    except (TypeError, ValueError, UnicodeEncodeError) as error:
        fail(f"cannot encode canonical JSON: {error}")


def canonical_digest(value: Any) -> str:
    return digest_bytes(canonical_bytes(value))


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_canonical(raw: bytes, description: str) -> Any:
    if not raw.isascii():
        fail(f"{description} must be ASCII JSON")
    try:
        value = json.loads(raw, object_pairs_hook=_reject_duplicate_pairs)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        fail(f"cannot parse {description}: {error}")
    if canonical_bytes(value) != raw:
        fail(f"{description} must be canonical JSON")
    return value


def exact_object(value: Any, keys: set[str], description: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        fail(f"{description} field roster drifted")
    return value


def exact_array(value: Any, count: int, description: str) -> list[Any]:
    if not isinstance(value, list) or len(value) != count:
        fail(f"{description} roster drifted")
    return value


def require_string(value: Any, description: str) -> str:
    if not isinstance(value, str) or not value:
        fail(f"{description} must be a nonempty string")
    return value


def require_safe_text(value: Any, description: str) -> str:
    result = require_string(value, description)
    if SAFE_TEXT.fullmatch(result) is None:
        fail(f"{description} is not a safe declaration")
    return result


def require_sha256(value: Any, description: str) -> str:
    result = require_string(value, description)
    if SHA256.fullmatch(result) is None or result == "0" * 64:
        fail(f"{description} must be a non-placeholder lowercase SHA-256")
    return result


def require_git_id(value: Any, description: str) -> str:
    result = require_string(value, description)
    if GIT_ID.fullmatch(result) is None or result == "0" * 40:
        fail(f"{description} must be a non-placeholder git identity")
    return result


def require_uint(value: Any, description: str) -> int:
    if (
        not isinstance(value, int)
        or isinstance(value, bool)
        or value < 0
        or value > MAX_U64
    ):
        fail(f"{description} must be an unsigned integer")
    return value


def safe_relative(value: str, description: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if (
        not value
        or not value.isascii()
        or path.is_absolute()
        or any(part in ("", ".", "..") for part in path.parts)
    ):
        fail(f"{description} must be a safe relative path")
    return path


def file_snapshot(metadata: os.stat_result) -> tuple[int, ...]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_nlink,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def directory_binding(metadata: os.stat_result) -> tuple[int, ...]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
    )


def directory_snapshot(metadata: os.stat_result) -> tuple[int, ...]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_nlink,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


class HeldDirectory:
    def __init__(
        self,
        parent_fd: int,
        name: str,
        fd: int,
        identity: tuple[int, ...],
        description: str,
    ) -> None:
        self.parent_fd = parent_fd
        self.name = name
        self.fd = fd
        self.identity = identity
        self.description = description
        self.names: set[str] | None = None


class HeldFile:
    def __init__(
        self,
        parent_fd: int,
        name: str,
        fd: int,
        identity: tuple[int, ...],
        raw: bytes,
        description: str,
    ) -> None:
        self.parent_fd = parent_fd
        self.name = name
        self.fd = fd
        self.identity = identity
        self.raw = raw
        self.description = description


class SecureRoot:
    def __init__(self, path: Path, description: str, *, private: bool) -> None:
        absolute = Path(os.path.abspath(os.fspath(path)))
        if not absolute.is_absolute() or any(
            part in ("", ".", "..") for part in absolute.parts[1:]
        ):
            fail(f"{description} path must be canonical and absolute")
        self.path = absolute
        self.description = description
        self.private = private
        self._root_fd = -1
        self._root_identity: tuple[int, ...] = ()
        self._root_names: set[str] | None = None
        self._absolute_chain: list[HeldDirectory] = []
        self._directories: dict[str, HeldDirectory] = {}
        self._files: dict[str, HeldFile] = {}
        try:
            self._root_fd = os.open(
                "/", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
            )
            root_metadata = os.fstat(self._root_fd)
            if not stat.S_ISDIR(root_metadata.st_mode):
                fail(f"filesystem root for {description} is not a directory")
            self._root_identity = directory_binding(root_metadata)
            current = self._root_fd
            for ordinal, part in enumerate(absolute.parts[1:], 1):
                before = os.stat(part, dir_fd=current, follow_symlinks=False)
                child = os.open(
                    part,
                    os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                    dir_fd=current,
                )
                opened = os.fstat(child)
                if (
                    stat.S_ISLNK(before.st_mode)
                    or not stat.S_ISDIR(before.st_mode)
                    or directory_binding(before) != directory_binding(opened)
                ):
                    os.close(child)
                    fail(f"{description} component {ordinal} was substituted")
                held = HeldDirectory(
                    current,
                    part,
                    child,
                    directory_binding(opened),
                    f"{description} component {ordinal}",
                )
                self._absolute_chain.append(held)
                current = child
            self.fd = current
            metadata = os.fstat(self.fd)
            if not stat.S_ISDIR(metadata.st_mode):
                fail(f"{description} must be a nonsymlink directory")
            if private and (
                stat.S_IMODE(metadata.st_mode) & 0o077
                or metadata.st_uid != os.geteuid()
            ):
                fail(f"{description} must be owner-private")
        except OSError as error:
            self._close_descriptors()
            fail(f"cannot open {description}: {error}")
        except BaseException:
            self._close_descriptors()
            raise

    def close(self, *, revalidate: bool = True) -> None:
        error: BaseException | None = None
        try:
            if revalidate:
                self.revalidate()
        except BaseException as caught:
            error = caught
        finally:
            self._close_descriptors()
        if error is not None:
            raise error

    def _close_descriptors(self) -> None:
        for held in reversed(list(self._files.values())):
            try:
                os.close(held.fd)
            except OSError:
                pass
        self._files.clear()
        for held in reversed(list(self._directories.values())):
            try:
                os.close(held.fd)
            except OSError:
                pass
        self._directories.clear()
        for held in reversed(self._absolute_chain):
            try:
                os.close(held.fd)
            except OSError:
                pass
        self._absolute_chain.clear()
        if self._root_fd >= 0:
            try:
                os.close(self._root_fd)
            except OSError:
                pass
            self._root_fd = -1

    def _hold_directory(self, relative: PurePosixPath) -> HeldDirectory | None:
        if relative.as_posix() == ".":
            return None
        key = relative.as_posix()
        existing = self._directories.get(key)
        if existing is not None:
            return existing
        parent_relative = relative.parent
        parent = self._hold_directory(parent_relative)
        parent_fd = self.fd if parent is None else parent.fd
        name = relative.name
        try:
            before = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
            fd = os.open(
                name,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                dir_fd=parent_fd,
            )
            opened = os.fstat(fd)
        except OSError as error:
            fail(f"cannot open directory {key}: {error}")
        if (
            stat.S_ISLNK(before.st_mode)
            or not stat.S_ISDIR(before.st_mode)
            or directory_binding(before) != directory_binding(opened)
        ):
            os.close(fd)
            fail(f"directory was substituted during validation: {key}")
        held = HeldDirectory(
            parent_fd,
            name,
            fd,
            directory_binding(opened),
            f"{self.description} directory {key}",
        )
        self._directories[key] = held
        return held

    def _open_parent(self, relative: PurePosixPath) -> tuple[int, str]:
        parent = self._hold_directory(relative.parent)
        return (self.fd if parent is None else parent.fd), relative.name

    @staticmethod
    def _read_fd(fd: int, size: int, description: str) -> bytes:
        os.lseek(fd, 0, os.SEEK_SET)
        chunks: list[bytes] = []
        remaining = size
        while remaining:
            chunk = os.read(fd, min(remaining, 1024 * 1024))
            if not chunk:
                fail(f"{description} was truncated during validation")
            chunks.append(chunk)
            remaining -= len(chunk)
        if os.read(fd, 1):
            fail(f"{description} has trailing bytes beyond its held size")
        return b"".join(chunks)

    def read(self, name: str, description: str, maximum: int) -> bytes:
        relative = safe_relative(name, description)
        key = relative.as_posix()
        existing = self._files.get(key)
        if existing is not None:
            current = os.fstat(existing.fd)
            raw = self._read_fd(existing.fd, len(existing.raw), description)
            if file_snapshot(current) != existing.identity or raw != existing.raw:
                fail(f"{description} changed during validation")
            return raw
        parent, leaf = self._open_parent(relative)
        try:
            before = os.stat(leaf, dir_fd=parent, follow_symlinks=False)
            fd = os.open(
                leaf, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent
            )
            initial = os.fstat(fd)
        except OSError as error:
            fail(f"cannot open {description}: {error}")
        try:
            if (
                stat.S_ISLNK(before.st_mode)
                or file_snapshot(before) != file_snapshot(initial)
                or not stat.S_ISREG(initial.st_mode)
                or initial.st_nlink != 1
                or initial.st_size <= 0
                or initial.st_size > maximum
            ):
                fail(f"{description} is not an admitted single-link regular file")
            raw = self._read_fd(fd, initial.st_size, description)
            final = os.fstat(fd)
            named = os.stat(leaf, dir_fd=parent, follow_symlinks=False)
            if file_snapshot(final) != file_snapshot(initial) or file_snapshot(
                named
            ) != file_snapshot(initial):
                fail(f"{description} changed during validation")
            self._files[key] = HeldFile(
                parent, leaf, fd, file_snapshot(final), raw, description
            )
            return raw
        except BaseException:
            os.close(fd)
            raise

    def read_json(self, name: str, description: str) -> tuple[Any, bytes]:
        raw = self.read(name, description, MAX_JSON_BYTES)
        return parse_canonical(raw, description), raw

    def names(self, name: str = ".") -> set[str]:
        if name == ".":
            held = None
            fd = self.fd
        else:
            relative = safe_relative(name, "directory")
            held = self._hold_directory(relative)
            if held is None:
                fail("internal directory custody failure")
            fd = held.fd
        initial = os.fstat(fd)
        result = set(os.listdir(fd))
        final = os.fstat(fd)
        if directory_snapshot(final) != directory_snapshot(initial):
            fail(f"directory changed during validation: {name}")
        if any(not item.isascii() for item in result):
            fail(f"directory contains a non-ASCII name: {name}")
        if held is None:
            if self._root_names is not None and self._root_names != result:
                fail(f"directory roster changed during validation: {name}")
            self._root_names = result
        else:
            if held.names is not None and held.names != result:
                fail(f"directory roster changed during validation: {name}")
            held.names = result
        return result

    def revalidate(self) -> None:
        root_metadata = os.fstat(self._root_fd)
        if directory_binding(root_metadata) != self._root_identity:
            fail(f"filesystem root changed for {self.description}")
        for held in self._absolute_chain:
            try:
                named = os.stat(held.name, dir_fd=held.parent_fd, follow_symlinks=False)
                opened = os.fstat(held.fd)
            except OSError as error:
                fail(f"cannot revalidate {held.description}: {error}")
            if (
                stat.S_ISLNK(named.st_mode)
                or directory_binding(named) != held.identity
                or directory_binding(opened) != held.identity
            ):
                fail(f"{held.description} changed during validation")
        if self.private:
            metadata = os.fstat(self.fd)
            if (
                stat.S_IMODE(metadata.st_mode) & 0o077
                or metadata.st_uid != os.geteuid()
            ):
                fail(f"{self.description} is no longer owner-private")
        if self._root_names is not None:
            before_names = os.fstat(self.fd)
            names = set(os.listdir(self.fd))
            after_names = os.fstat(self.fd)
            if names != self._root_names or directory_snapshot(
                before_names
            ) != directory_snapshot(after_names):
                fail(f"{self.description} root roster changed during validation")
        for held in self._directories.values():
            try:
                named = os.stat(held.name, dir_fd=held.parent_fd, follow_symlinks=False)
                opened = os.fstat(held.fd)
                before_names = os.fstat(held.fd)
                names = set(os.listdir(held.fd)) if held.names is not None else None
                after_names = os.fstat(held.fd)
            except OSError as error:
                fail(f"cannot revalidate {held.description}: {error}")
            if (
                stat.S_ISLNK(named.st_mode)
                or directory_binding(named) != held.identity
                or directory_binding(opened) != held.identity
                or (
                    held.names is not None
                    and (
                        names != held.names
                        or directory_snapshot(before_names)
                        != directory_snapshot(after_names)
                    )
                )
            ):
                fail(f"{held.description} changed during validation")
        for held in self._files.values():
            try:
                current = os.fstat(held.fd)
                named = os.stat(held.name, dir_fd=held.parent_fd, follow_symlinks=False)
                raw = self._read_fd(held.fd, len(held.raw), held.description)
            except OSError as error:
                fail(f"cannot revalidate {held.description}: {error}")
            if (
                stat.S_ISLNK(named.st_mode)
                or file_snapshot(current) != held.identity
                or file_snapshot(named) != held.identity
                or raw != held.raw
            ):
                fail(f"{held.description} changed during validation")


def descriptor(path: str, raw: bytes) -> dict[str, Any]:
    return {"bytes": len(raw), "path": path, "sha256": digest_bytes(raw)}


def validate_intake(
    value: Any,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, str]]:
    intake = exact_object(
        value,
        {
            "authority",
            "format",
            "milestone",
            "nonclaim",
            "obligation_id",
            "path_id",
            "policy_review_sha256",
            "sources",
            "status",
            "target",
            "tcb",
            "toolchain",
        },
        "r29 intake",
    )
    expected = {
        "authority": INTAKE_AUTHORITY,
        "format": INTAKE_FORMAT,
        "milestone": "M1",
        "nonclaim": INTAKE_NONCLAIM,
        "obligation_id": OBLIGATION_ID,
        "path_id": PATH_ID,
        "status": "external-input-not-independent-evidence",
        "target": TARGET,
    }
    for key, expected_value in expected.items():
        if intake[key] != expected_value:
            fail(f"r29 intake {key} drifted")
    require_sha256(intake["policy_review_sha256"], "policy review identity")
    sources = exact_array(intake["sources"], len(SOURCE_IDS), "source")
    normalized_sources: list[dict[str, Any]] = []
    for source, identifier in zip(sources, SOURCE_IDS):
        row = exact_object(
            source,
            {
                "base_commit",
                "commit",
                "id",
                "repository",
                "source_closure_sha256",
                "tree",
            },
            "source identity",
        )
        if (
            row["id"] != identifier
            or row["repository"] != SOURCE_REPOSITORIES[identifier]
        ):
            fail("source identity roster drifted")
        require_git_id(row["base_commit"], f"{identifier} base commit")
        require_git_id(row["commit"], f"{identifier} commit")
        require_git_id(row["tree"], f"{identifier} tree")
        require_sha256(row["source_closure_sha256"], f"{identifier} source closure")
        normalized_sources.append(row)
    tcb = exact_array(intake["tcb"], len(TCB_IDS), "TCB")
    normalized_tcb: list[dict[str, Any]] = []
    for row_value, identifier in zip(tcb, TCB_IDS):
        row = exact_object(row_value, {"id", "identity_sha256", "kind"}, "TCB identity")
        if row["id"] != identifier or row["kind"] != TCB_KINDS[identifier]:
            fail("TCB identity roster drifted")
        require_sha256(row["identity_sha256"], f"{identifier} identity")
        normalized_tcb.append(row)
    toolchain_value = exact_object(intake["toolchain"], TOOLCHAIN_KEYS, "toolchain")
    toolchain = {
        key: require_sha256(toolchain_value[key], f"toolchain {key}")
        for key in sorted(TOOLCHAIN_KEYS)
    }
    return normalized_sources, normalized_tcb, toolchain


def validate_review(value: Any, policy_sha256: str) -> None:
    review = exact_object(
        value,
        {
            "authority",
            "format",
            "independence",
            "nonclaim",
            "organization",
            "policy_sha256",
            "review_identity_sha256",
            "reviewer",
            "status",
            "target",
        },
        "policy review",
    )
    expected = {
        "authority": REVIEW_AUTHORITY,
        "format": REVIEW_FORMAT,
        "independence": "not-validated-by-ferric",
        "nonclaim": REVIEW_NONCLAIM,
        "policy_sha256": policy_sha256,
        "status": "reviewed-declared",
        "target": TARGET,
    }
    for key, expected_value in expected.items():
        if review[key] != expected_value:
            fail(f"policy review {key} drifted")
    require_safe_text(review["organization"], "review organization")
    require_safe_text(review["reviewer"], "reviewer")
    require_sha256(review["review_identity_sha256"], "review identity")


def validate_plan(value: Any) -> tuple[list[dict[str, Any]], dict[str, str]]:
    plan = exact_object(
        value,
        {
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
        },
        "differential plan",
    )
    expected = {
        "authority": "benchmark-run-plan-only",
        "format": PLAN_FORMAT,
        "milestone": "M1",
        "nonclaim": DIFFERENTIAL_NONCLAIM,
        "obligation_id": OBLIGATION_ID,
        "path_id": PATH_ID,
        "source_path": "benches/m1/differential.rs",
        "suite": SUITE,
        "target": TARGET,
    }
    for key, expected_value in expected.items():
        if plan[key] != expected_value:
            fail(f"differential plan {key} drifted")
    require_sha256(plan["input_sha256"], "plan input identity")
    identities_value = exact_object(
        plan["identities"], set(PLAN_IDENTITIES), "plan identities"
    )
    identities = {
        key: require_sha256(identities_value[key], f"plan identity {key}")
        for key in PLAN_IDENTITIES
    }
    cases = exact_array(plan["cases"], len(CASE_KINDS), "plan case")
    normalized: list[dict[str, Any]] = []
    for case_value, kind in zip(cases, CASE_KINDS):
        case = exact_object(
            case_value,
            {"id", "input_sha256", "kind", "workload_sha256"},
            "plan case",
        )
        if case["id"] != f"{kind}.001" or case["kind"] != kind:
            fail("plan case roster drifted")
        require_sha256(case["input_sha256"], f"{kind} input identity")
        require_sha256(case["workload_sha256"], f"{kind} workload identity")
        normalized.append(case)
    return normalized, identities


def validate_policy(value: Any) -> dict[str, dict[str, int]]:
    policy = exact_object(
        value,
        {
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
        },
        "acceptance policy",
    )
    expected = {
        "authority": "externally-admitted-differential-threshold-policy-only",
        "finite_logits_required": True,
        "format": POLICY_FORMAT,
        "logit_metric": "maximum-monotonic-bf16-ulp-distance-signed-zero-equal",
        "nonclaim": POLICY_NONCLAIM,
        "obligation_id": OBLIGATION_ID,
        "path_id": PATH_ID,
        "suite": SUITE,
        "target": TARGET,
        "token_metric": "ferric-reference-greedy-token-mismatch-count",
        "token_selection": "lowest-token-id-bf16-argmax",
    }
    for key, expected_value in expected.items():
        if policy[key] != expected_value:
            fail(f"acceptance policy {key} drifted")
    result: dict[str, dict[str, int]] = {}
    for row_value, kind in zip(
        exact_array(policy["cases"], 7, "policy case"), CASE_KINDS
    ):
        row = exact_object(
            row_value,
            {"kind", "maximum_logit_ulp_error", "maximum_token_mismatches"},
            "policy case",
        )
        if row["kind"] != kind:
            fail("acceptance policy case roster drifted")
        logit = require_uint(row["maximum_logit_ulp_error"], f"{kind} logit threshold")
        tokens = require_uint(
            row["maximum_token_mismatches"], f"{kind} token threshold"
        )
        if tokens > ROWS[kind]:
            fail(f"{kind} token threshold exceeds its row count")
        result[kind] = {
            "maximum_logit_ulp_error": logit,
            "maximum_token_mismatches": tokens,
        }
    return result


def output_record(manifest: dict[str, Any], manifest_raw: bytes) -> dict[str, Any]:
    return {
        "logits_bytes": manifest["logits"]["bytes"],
        "logits_sha256": manifest["logits"]["sha256"],
        "manifest_sha256": digest_bytes(manifest_raw),
        "tokens_bytes": manifest["tokens"]["bytes"],
        "tokens_sha256": manifest["tokens"]["sha256"],
    }


def validate_runner(
    value: Any,
    case: dict[str, Any],
    identities: dict[str, str],
    plan_sha256: str,
) -> dict[str, Any]:
    runner = exact_object(
        value,
        {
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
        },
        "Ferric runner transcript",
    )
    expected = {
        "authority": "observed-target-only-qualification-capture",
        "benchmark_executable_sha256": identities["benchmark-executable"],
        "benchmark_protocol_sha256": identities["benchmark-protocol"],
        "case_id": case["id"],
        "environment_sha256": identities["environment"],
        "format": CAPTURE_FORMAT,
        "input_sha256": case["input_sha256"],
        "kind": case["kind"],
        "nonclaim": CAPTURE_NONCLAIM,
        "plan_sha256": plan_sha256,
        "runner_declaration_sha256": identities["generated-plan"],
        "status": "OBSERVED",
        "target": TARGET,
        "workload_sha256": case["workload_sha256"],
    }
    for key, expected_value in expected.items():
        if runner[key] != expected_value:
            fail(f"runner transcript {key} drifted for {case['id']}")
    for key in (
        "compact_sha256",
        "device_identity_sha256",
        "kernel_artifact_manifest_sha256",
        "logits_sha256",
        "program_catalog_sha256",
        "tokens_sha256",
    ):
        require_sha256(runner[key], f"runner {key}")
    dispatch_generation = require_uint(
        runner["dispatch_generation"], "runner dispatch generation"
    )
    if require_uint(runner["gpu_unique_id"], "runner GPU unique ID") == 0:
        fail("runner GPU unique ID must be positive")
    row_hashes = exact_array(
        runner["logits_row_sha256"], ROWS[case["kind"]], "runner logit row identity"
    )
    for index, identity in enumerate(row_hashes):
        require_sha256(identity, f"runner logit row {index} identity")
    mode = "decode" if case["kind"].startswith("decode-") else "prefill"
    selection = exact_object(
        runner["selection"], {"bucket", "mode", "role"}, "runner selection"
    )
    if selection != {
        "bucket": case["kind"],
        "mode": mode,
        "role": "target-8b",
    }:
        fail(f"runner selection drifted for {case['id']}")
    validate_runner_execution(
        runner["execution"], case, identities, mode, dispatch_generation
    )
    return runner


def validate_runner_execution(
    value: Any,
    case: dict[str, Any],
    identities: dict[str, str],
    mode: str,
    dispatch_generation: int,
) -> None:
    if mode == "prefill":
        execution = exact_object(
            value,
            {"dispatch_generation", "epoch", "mode", "round_count"},
            "prefill runner execution",
        )
        if (
            require_uint(
                execution["dispatch_generation"], "prefill dispatch generation"
            )
            != dispatch_generation
            or execution["mode"] != "one-shot-prefill"
            or require_uint(execution["round_count"], "prefill round count") != 1
        ):
            fail(f"prefill runner execution drifted for {case['id']}")
        require_uint(execution["epoch"], "prefill runner epoch")
        return

    execution = exact_object(
        value,
        {
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
        },
        "decode runner execution",
    )
    if execution["context_plan_sha256"] != identities[f"dispatch-graph-{case['kind']}"]:
        fail(f"decode context plan drifted for {case['id']}")
    for key in ("declared_workload_binding_sha256", "round_history_sha256"):
        require_sha256(execution[key], f"decode runner {key}")
    require_uint(execution["first_dispatch_generation"], "decode first generation")
    require_uint(execution["first_epoch"], "decode first epoch")
    require_uint(execution["terminal_epoch"], "decode terminal epoch")
    if (
        execution["mode"] != "teacher-forced-c8192"
        or require_uint(execution["round_count"], "decode round count") != 8192
        or require_uint(
            execution["terminal_dispatch_generation"],
            "decode terminal dispatch generation",
        )
        != dispatch_generation
        or require_uint(execution["terminal_ordinal"], "decode terminal ordinal")
        != 8191
    ):
        fail(f"decode runner execution drifted for {case['id']}")
    lanes = exact_array(
        execution["ordered_lane_bindings"],
        ROWS[case["kind"]],
        "decode lane binding",
    )
    for ordinal, lane_value in enumerate(lanes):
        lane = exact_object(
            lane_value,
            {
                "lane_identity_sha256",
                "lane_ordinal",
                "token_sequence_identity_sha256",
            },
            "decode lane binding",
        )
        require_sha256(lane["lane_identity_sha256"], "decode lane identity")
        require_sha256(
            lane["token_sequence_identity_sha256"],
            "decode token-sequence identity",
        )
        if require_uint(lane["lane_ordinal"], "decode lane ordinal") != ordinal:
            fail(f"decode lane ordinal drifted for {case['id']}")


def validate_output(
    root: SecureRoot,
    path: str,
    producer: str,
    case: dict[str, Any],
    identities: dict[str, str],
    plan_sha256: str,
    runner_sha256: str,
) -> tuple[dict[str, Any], bytes, list[dict[str, Any]], list[str]]:
    value, raw = root.read_json(path, f"{producer} output manifest")
    output = exact_object(
        value,
        {
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
        },
        f"{producer} output manifest",
    )
    identity_names = (
        ("benchmark-executable", "benchmark-protocol")
        if producer == "ferric"
        else ("reference-implementation", "reference-protocol")
    )
    expected = {
        "authority": "externally-collected-model-output-only",
        "case_id": case["id"],
        "environment_sha256": identities["environment"],
        "format": OUTPUT_FORMAT,
        "input_sha256": case["input_sha256"],
        "kind": case["kind"],
        "plan_sha256": plan_sha256,
        "producer": producer,
        "producer_sha256": identities[identity_names[0]],
        "protocol_sha256": identities[identity_names[1]],
        "runner_transcript_sha256": runner_sha256,
        "workload_sha256": case["workload_sha256"],
    }
    for key, expected_value in expected.items():
        if output[key] != expected_value:
            fail(f"{producer} output {key} drifted for {case['id']}")
    shape = exact_object(output["shape"], {"rows", "vocabulary_size"}, "output shape")
    rows = ROWS[case["kind"]]
    if shape != {"rows": rows, "vocabulary_size": VOCABULARY_SIZE}:
        fail(f"{producer} output shape drifted for {case['id']}")
    base = PurePosixPath(path).parent
    artifacts = [descriptor(path, raw)]
    logits_row_sha256: list[str] = []
    for field, encoding, expected_bytes in (
        ("logits", "bf16-le", rows * VOCABULARY_SIZE * 2),
        ("tokens", "u32-le", rows * 4),
    ):
        payload = exact_object(
            output[field],
            {"bytes", "encoding", "path", "sha256"},
            f"{producer} {field}",
        )
        if payload["bytes"] != expected_bytes or payload["encoding"] != encoding:
            fail(f"{producer} {field} contract drifted for {case['id']}")
        relative = safe_relative(payload["path"], f"{producer} {field} path")
        if len(relative.parts) != 1:
            fail(f"{producer} payload must remain inside its bundle")
        payload_path = (base / relative).as_posix()
        payload_raw = root.read(
            payload_path, f"{producer} {field} payload", MAX_PAYLOAD_BYTES
        )
        if (
            len(payload_raw) != expected_bytes
            or digest_bytes(payload_raw) != payload["sha256"]
        ):
            fail(f"{producer} {field} identity drifted for {case['id']}")
        require_sha256(payload["sha256"], f"{producer} {field} identity")
        if field == "logits":
            row_bytes = VOCABULARY_SIZE * 2
            logits_row_sha256 = [
                digest_bytes(payload_raw[offset : offset + row_bytes])
                for offset in range(0, len(payload_raw), row_bytes)
            ]
            if len(logits_row_sha256) != rows:
                fail(f"{producer} logit row roster drifted for {case['id']}")
        artifacts.append(descriptor(payload_path, payload_raw))
    return output, raw, artifacts, logits_row_sha256


def validate_runner_payload_bindings(
    runner: dict[str, Any],
    ferric: dict[str, Any],
    logits_row_sha256: list[str],
    case: dict[str, Any],
) -> None:
    if (
        runner["logits_sha256"] != ferric["logits"]["sha256"]
        or runner["tokens_sha256"] != ferric["tokens"]["sha256"]
        or runner["logits_row_sha256"] != logits_row_sha256
    ):
        fail(f"runner payload identities differ from Ferric output for {case['id']}")


def validate_companion(value: Any, path: str, raw: bytes, description: str) -> None:
    companion = exact_object(value, {"bytes", "path", "sha256"}, description)
    if companion != descriptor(path, raw):
        fail(f"{description} identity drifted")


def validate_comparison(
    root: SecureRoot,
    cases: list[dict[str, Any]],
    case_rows: list[dict[str, Any]],
    plan_sha256: str,
    pairs_sha256: str,
) -> list[dict[str, Any]]:
    expected_raw = {f"{case['id']}.differential.raw.json" for case in cases}
    if root.names("comparison.bundle") != {"raw", "records.json"}:
        fail("comparison bundle roster drifted")
    if root.names("comparison.bundle/raw") != expected_raw:
        fail("raw comparison roster drifted")
    records, records_raw = root.read_json(
        "comparison.bundle/records.json", "differential records"
    )
    records = exact_object(
        records, {"format", "observations", "plan_sha256", "suite"}, "records"
    )
    if (
        records["format"] != RECORDS_FORMAT
        or records["plan_sha256"] != plan_sha256
        or records["suite"] != SUITE
    ):
        fail("differential records binding drifted")
    observations = exact_array(records["observations"], 7, "record observation")
    artifacts = [descriptor("comparison.bundle/records.json", records_raw)]
    for case, summary, observation in zip(cases, case_rows, observations):
        observation = exact_object(
            observation,
            {
                "attributes",
                "case_id",
                "kind",
                "measurements",
                "recorded_samples",
                "status",
                "warmups",
            },
            "record observation",
        )
        raw_path = f"comparison.bundle/raw/{case['id']}.differential.raw.json"
        raw_value, raw_bytes = root.read_json(raw_path, "raw differential record")
        raw_record = exact_object(
            raw_value,
            {
                "authority",
                "case_id",
                "comparison",
                "ferric_output",
                "format",
                "kind",
                "nonclaim",
                "pairs_sha256",
                "plan_sha256",
                "reference_output",
                "runner_transcript_sha256",
                "shape",
                "status",
            },
            "raw differential record",
        )
        expected_raw_values = {
            "authority": "computed-differential-comparison-only",
            "case_id": case["id"],
            "comparison": summary["comparison"],
            "ferric_output": summary["ferric_output"],
            "format": RAW_FORMAT,
            "kind": case["kind"],
            "nonclaim": DIFFERENTIAL_NONCLAIM,
            "pairs_sha256": pairs_sha256,
            "plan_sha256": plan_sha256,
            "reference_output": summary["reference_output"],
            "runner_transcript_sha256": summary["runner_transcript_sha256"],
            "shape": {"rows": ROWS[case["kind"]], "vocabulary_size": VOCABULARY_SIZE},
            "status": "compared",
        }
        if raw_record != expected_raw_values:
            fail(f"raw comparison drifted for {case['id']}")
        expected_observation = {
            "attributes": {
                "ferric-output-sha256": summary["ferric_output"]["manifest_sha256"],
                "raw-record-sha256": digest_bytes(raw_bytes),
                "reference-output-sha256": summary["reference_output"][
                    "manifest_sha256"
                ],
                "runner-transcript-sha256": summary["runner_transcript_sha256"],
            },
            "case_id": case["id"],
            "kind": case["kind"],
            "measurements": {
                "compared-logits": [summary["comparison"]["compared_logits"]],
                "compared-tokens": [summary["comparison"]["compared_tokens"]],
                "maximum-logit-ulp-error": [
                    summary["comparison"]["maximum_logit_ulp_error"]
                ],
                "token-mismatches": [summary["comparison"]["token_mismatches"]],
            },
            "recorded_samples": 1,
            "status": "completed",
            "warmups": 0,
        }
        if observation != expected_observation:
            fail(f"record observation drifted for {case['id']}")
        artifacts.append(descriptor(raw_path, raw_bytes))
    return artifacts


def build_documents(intake_path: Path) -> tuple[bytes, bytes]:
    root = SecureRoot(intake_path, "r29 intake root", private=True)
    try:
        if root.names() != {
            "acceptance-policy.json",
            "acceptance.json",
            "captures",
            "comparison.bundle",
            "intake.json",
            "pairs.json",
            "plan.json",
            "policy-review.json",
            "references",
        }:
            fail("r29 intake root roster drifted")
        intake, intake_raw = root.read_json("intake.json", "r29 intake")
        sources, tcb, toolchain = validate_intake(intake)
        plan, plan_raw = root.read_json("plan.json", "differential plan")
        cases, identities = validate_plan(plan)
        plan_sha256 = digest_bytes(plan_raw)
        policy, policy_raw = root.read_json(
            "acceptance-policy.json", "acceptance policy"
        )
        thresholds = validate_policy(policy)
        policy_sha256 = digest_bytes(policy_raw)
        if identities["differential-acceptance-policy"] != policy_sha256:
            fail("plan acceptance-policy identity drifted")
        review, review_raw = root.read_json("policy-review.json", "policy review")
        validate_review(review, policy_sha256)
        if intake["policy_review_sha256"] != digest_bytes(review_raw):
            fail("intake policy-review identity drifted")
        if sources[0]["source_closure_sha256"] != identities["fe2o3-source-closure"]:
            fail("fe2o3 source closure differs from the differential plan")
        if sources[1]["source_closure_sha256"] != identities["ferric-source-closure"]:
            fail("Ferric source closure differs from the differential plan")
        toolchain_plan_bindings = {
            "benchmark_executable_sha256": "benchmark-executable",
            "benchmark_protocol_sha256": "benchmark-protocol",
            "qualification_protocol_sha256": "benchmark-protocol",
            "reference_implementation_sha256": "reference-implementation",
            "reference_protocol_sha256": "reference-protocol",
        }
        for toolchain_name, identity_name in toolchain_plan_bindings.items():
            if toolchain[toolchain_name] != identities[identity_name]:
                fail(f"toolchain {toolchain_name} differs from the differential plan")

        expected_bundles = {f"{kind}.capture.bundle" for kind in CASE_KINDS}
        expected_references = {f"{kind}.reference.bundle" for kind in CASE_KINDS}
        if root.names("captures") != expected_bundles:
            fail("Ferric capture bundle roster drifted")
        if root.names("references") != expected_references:
            fail("reference output bundle roster drifted")
        for kind in CASE_KINDS:
            for base in (
                f"captures/{kind}.capture.bundle",
                f"references/{kind}.reference.bundle",
            ):
                if root.names(base) != {
                    "logits.bf16le",
                    "output.json",
                    "runner.json",
                    "tokens.u32le",
                }:
                    fail(f"differential output bundle roster drifted: {base}")

        pairs, pairs_raw = root.read_json("pairs.json", "differential pairs")
        pairs = exact_object(
            pairs, {"authority", "format", "pairs", "plan_sha256", "suite"}, "pairs"
        )
        if (
            pairs["authority"] != "externally-collected-differential-pairs-only"
            or pairs["format"] != PAIRS_FORMAT
            or pairs["plan_sha256"] != plan_sha256
            or pairs["suite"] != SUITE
        ):
            fail("differential pairs binding drifted")
        pair_rows = exact_array(pairs["pairs"], 7, "differential pair")
        acceptance, acceptance_raw = root.read_json(
            "acceptance.json", "acceptance result"
        )
        acceptance = exact_object(
            acceptance,
            {
                "authority",
                "cases",
                "format",
                "nonclaim",
                "obligation_id",
                "pairs_sha256",
                "path_id",
                "plan_sha256",
                "policy_sha256",
                "status",
                "suite",
                "target",
            },
            "acceptance result",
        )
        pairs_sha256 = digest_bytes(pairs_raw)
        acceptance_expected = {
            "authority": "checked-differential-policy-conformance-only",
            "format": ACCEPTANCE_FORMAT,
            "nonclaim": ACCEPTANCE_NONCLAIM,
            "obligation_id": OBLIGATION_ID,
            "pairs_sha256": pairs_sha256,
            "path_id": PATH_ID,
            "plan_sha256": plan_sha256,
            "policy_sha256": policy_sha256,
            "status": "POLICY_CONFORMING",
            "suite": SUITE,
            "target": TARGET,
        }
        for key, expected_value in acceptance_expected.items():
            if acceptance[key] != expected_value:
                fail(f"acceptance result {key} drifted")
        acceptance_rows = exact_array(acceptance["cases"], 7, "acceptance case")

        artifacts = [
            descriptor("acceptance-policy.json", policy_raw),
            descriptor("acceptance.json", acceptance_raw),
            descriptor("intake.json", intake_raw),
            descriptor("pairs.json", pairs_raw),
            descriptor("plan.json", plan_raw),
            descriptor("policy-review.json", review_raw),
        ]
        case_summaries: list[dict[str, Any]] = []
        for case, pair_value, accepted in zip(cases, pair_rows, acceptance_rows):
            kind = case["kind"]
            pair = exact_object(
                pair_value,
                {
                    "case_id",
                    "ferric_output_manifest",
                    "kind",
                    "reference_output_manifest",
                    "runner_transcript",
                },
                "differential pair",
            )
            if pair["case_id"] != case["id"] or pair["kind"] != kind:
                fail("differential pair case roster drifted")
            capture_base = f"captures/{kind}.capture.bundle"
            reference_base = f"references/{kind}.reference.bundle"
            runner_path = f"{capture_base}/runner.json"
            reference_runner_path = f"{reference_base}/runner.json"
            runner_value, runner_raw = root.read_json(
                runner_path, "Ferric runner transcript"
            )
            reference_runner_value, reference_runner_raw = root.read_json(
                reference_runner_path, "reference runner transcript"
            )
            if (
                runner_raw != reference_runner_raw
                or runner_value != reference_runner_value
            ):
                fail(f"reference runner differs from Ferric runner for {case['id']}")
            runner_sha256 = digest_bytes(runner_raw)
            runner = validate_runner(runner_value, case, identities, plan_sha256)
            ferric_path = f"{capture_base}/output.json"
            reference_path = f"{reference_base}/output.json"
            ferric, ferric_raw, ferric_artifacts, ferric_row_sha256 = validate_output(
                root,
                ferric_path,
                "ferric",
                case,
                identities,
                plan_sha256,
                runner_sha256,
            )
            reference, reference_raw, reference_artifacts, _ = validate_output(
                root,
                reference_path,
                "reference",
                case,
                identities,
                plan_sha256,
                runner_sha256,
            )
            validate_runner_payload_bindings(runner, ferric, ferric_row_sha256, case)
            validate_companion(
                pair["runner_transcript"], runner_path, runner_raw, "runner companion"
            )
            validate_companion(
                pair["ferric_output_manifest"],
                ferric_path,
                ferric_raw,
                "Ferric output companion",
            )
            validate_companion(
                pair["reference_output_manifest"],
                reference_path,
                reference_raw,
                "reference output companion",
            )
            accepted = exact_object(
                accepted,
                {
                    "case_id",
                    "comparison",
                    "ferric_output",
                    "kind",
                    "reference_output",
                    "runner_transcript_sha256",
                    "status",
                    "threshold",
                },
                "acceptance case",
            )
            comparison = exact_object(
                accepted["comparison"],
                {
                    "compared_logits",
                    "compared_tokens",
                    "maximum_logit_ulp_error",
                    "token_mismatches",
                },
                "acceptance comparison",
            )
            expected_logits = ROWS[kind] * VOCABULARY_SIZE
            if (
                require_uint(comparison["compared_logits"], "compared logits")
                != expected_logits
                or require_uint(comparison["compared_tokens"], "compared tokens")
                != ROWS[kind]
            ):
                fail(f"acceptance comparison count drifted for {case['id']}")
            maximum_ulp = require_uint(
                comparison["maximum_logit_ulp_error"], "maximum ULP"
            )
            token_mismatches = require_uint(
                comparison["token_mismatches"], "token mismatches"
            )
            if (
                maximum_ulp > thresholds[kind]["maximum_logit_ulp_error"]
                or token_mismatches > thresholds[kind]["maximum_token_mismatches"]
            ):
                fail(f"acceptance comparison exceeds policy for {case['id']}")
            ferric_record = output_record(ferric, ferric_raw)
            reference_record = output_record(reference, reference_raw)
            expected_acceptance_case = {
                "case_id": case["id"],
                "comparison": comparison,
                "ferric_output": ferric_record,
                "kind": kind,
                "reference_output": reference_record,
                "runner_transcript_sha256": runner_sha256,
                "status": "within-policy",
                "threshold": thresholds[kind],
            }
            if accepted != expected_acceptance_case:
                fail(f"acceptance case binding drifted for {case['id']}")
            artifacts.extend(ferric_artifacts)
            artifacts.extend(reference_artifacts)
            artifacts.append(descriptor(runner_path, runner_raw))
            artifacts.append(descriptor(reference_runner_path, reference_runner_raw))
            case_summaries.append(expected_acceptance_case)

        artifacts.extend(
            validate_comparison(root, cases, case_summaries, plan_sha256, pairs_sha256)
        )
        if len({row["path"] for row in artifacts}) != len(artifacts):
            fail("r29 artifact path was reused")
        artifacts.sort(key=lambda row: row["path"])
        roster = {
            "artifacts": artifacts,
            "authority": ROSTER_AUTHORITY,
            "cases": case_summaries,
            "format": ROSTER_FORMAT,
            "intake_sha256": digest_bytes(intake_raw),
            "milestone": "M1",
            "obligation_id": OBLIGATION_ID,
            "pairs_sha256": pairs_sha256,
            "path_id": PATH_ID,
            "plan_sha256": plan_sha256,
            "policy_review_sha256": digest_bytes(review_raw),
            "policy_sha256": policy_sha256,
            "sources": sources,
            "target": TARGET,
            "tcb": tcb,
            "toolchain": toolchain,
        }
        roster_raw = canonical_bytes(roster)
        report = {
            "artifact_count": len(artifacts),
            "artifact_roster_sha256": digest_bytes(roster_raw),
            "authority": REPORT_AUTHORITY,
            "format": REPORT_FORMAT,
            "hardware_claim": "external-identities-only",
            "independent_validation": False,
            "milestone": "M1",
            "nonclaim": NONCLAIM,
            "obligation_id": OBLIGATION_ID,
            "path_id": PATH_ID,
            "policy_review_status": "declared-not-independently-validated",
            "qualification_evidence": False,
            "r29_closed": False,
            "source_roster_sha256": canonical_digest(sources),
            "status": "partial-non-evidence",
            "target": TARGET,
            "tcb_roster_sha256": canonical_digest(tcb),
            "toolchain_roster_sha256": canonical_digest(toolchain),
        }
        return roster_raw, canonical_bytes(report)
    finally:
        root.close()


def _rename_noreplace(parent_fd: int, source: str, destination: str) -> None:
    libc = ctypes.CDLL(None, use_errno=True)
    renameat2 = getattr(libc, "renameat2", None)
    if renameat2 is None:
        fail("renameat2 is unavailable; refusing non-atomic publication")
    result = renameat2(
        ctypes.c_int(parent_fd),
        ctypes.c_char_p(os.fsencode(source)),
        ctypes.c_int(parent_fd),
        ctypes.c_char_p(os.fsencode(destination)),
        ctypes.c_uint(RENAME_NOREPLACE),
    )
    if result != 0:
        error = ctypes.get_errno()
        if error == errno.EEXIST:
            fail("r29 output bundle already exists")
        fail(f"cannot publish r29 output bundle: {os.strerror(error)}")


def _read_exact_bound_file(
    directory_fd: int,
    name: str,
    expected: bytes,
    expected_identity: tuple[int, ...],
    description: str,
) -> None:
    try:
        before = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        fd = os.open(
            name,
            os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC,
            dir_fd=directory_fd,
        )
        opened = os.fstat(fd)
    except OSError as error:
        fail(f"cannot open {description} {name}: {error}")
    try:
        if (
            stat.S_ISLNK(before.st_mode)
            or not stat.S_ISREG(before.st_mode)
            or before.st_nlink != 1
            or stat.S_IMODE(before.st_mode) != 0o600
            or before.st_uid != os.geteuid()
            or file_snapshot(before) != expected_identity
            or file_snapshot(opened) != expected_identity
        ):
            fail(f"{description} {name} is not the created staged file")
        raw = SecureRoot._read_fd(fd, len(expected), f"{description} {name}")
        final = os.fstat(fd)
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if (
            raw != expected
            or file_snapshot(final) != expected_identity
            or file_snapshot(named) != expected_identity
        ):
            fail(f"{description} {name} changed during publication")
    finally:
        os.close(fd)


def _require_directory_roster(fd: int, expected: set[str], description: str) -> None:
    before = os.fstat(fd)
    names = set(os.listdir(fd))
    after = os.fstat(fd)
    if names != expected or directory_snapshot(before) != directory_snapshot(after):
        fail(f"{description} roster drifted")


def _validate_published_bundle(
    parent_fd: int,
    name: str,
    staging_identity: tuple[int, ...],
    staged_files: dict[str, tuple[int, ...]],
    roster: bytes,
    report: bytes,
) -> None:
    try:
        before = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        fd = os.open(
            name,
            os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
            dir_fd=parent_fd,
        )
        opened = os.fstat(fd)
    except OSError as error:
        fail(f"cannot reopen published r29 output bundle: {error}")
    try:
        if (
            stat.S_ISLNK(before.st_mode)
            or not stat.S_ISDIR(before.st_mode)
            or directory_binding(before) != staging_identity
            or directory_binding(opened) != staging_identity
            or stat.S_IMODE(opened.st_mode) != 0o700
            or opened.st_uid != os.geteuid()
        ):
            fail("published r29 output bundle is not the staged directory")
        _require_directory_roster(
            fd, {"report.json", "roster.json"}, "published r29 output bundle"
        )
        _read_exact_bound_file(
            fd,
            "report.json",
            report,
            staged_files["report.json"],
            "published r29 output",
        )
        _read_exact_bound_file(
            fd,
            "roster.json",
            roster,
            staged_files["roster.json"],
            "published r29 output",
        )
        final = os.fstat(fd)
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        if (
            directory_binding(final) != staging_identity
            or directory_binding(named) != staging_identity
        ):
            fail("published r29 output bundle changed after verification")
        _require_directory_roster(
            fd, {"report.json", "roster.json"}, "published r29 output bundle"
        )
    finally:
        os.close(fd)


def produce(intake: Path, output: Path) -> None:
    roster, report = build_documents(intake)
    intake = intake.absolute()
    output = output.absolute()
    if output.is_relative_to(intake):
        fail("r29 output bundle must be outside the intake root")
    safe_relative(output.name, "r29 output name")
    parent = SecureRoot(output.parent, "r29 output parent", private=True)
    nonce = f".{output.name}.staging.{os.getpid()}.{os.urandom(8).hex()}"
    staging_named = False
    staging_fd = -1
    staging_identity: tuple[int, ...] | None = None
    staged_files: dict[str, tuple[int, ...]] = {}
    try:
        try:
            os.stat(output.name, dir_fd=parent.fd, follow_symlinks=False)
            fail("r29 output bundle already exists")
        except FileNotFoundError:
            pass
        os.mkdir(nonce, 0o700, dir_fd=parent.fd)
        staging_named = True
        staging_fd = os.open(
            nonce,
            os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
            dir_fd=parent.fd,
        )
        staged = os.fstat(staging_fd)
        named = os.stat(nonce, dir_fd=parent.fd, follow_symlinks=False)
        staging_identity = directory_binding(staged)
        if (
            not stat.S_ISDIR(staged.st_mode)
            or stat.S_IMODE(staged.st_mode) != 0o700
            or staged.st_uid != os.geteuid()
            or directory_binding(named) != staging_identity
        ):
            fail("r29 staging directory was substituted after creation")
        for name, raw in (("report.json", report), ("roster.json", roster)):
            fd = os.open(
                name,
                os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                0o600,
                dir_fd=staging_fd,
            )
            try:
                offset = 0
                while offset != len(raw):
                    written = os.write(fd, raw[offset:])
                    if written <= 0:
                        fail(f"cannot completely write staged r29 output: {name}")
                    offset += written
                os.fsync(fd)
                staged_files[name] = file_snapshot(os.fstat(fd))
            finally:
                os.close(fd)
        os.fsync(staging_fd)
        staged = os.fstat(staging_fd)
        named = os.stat(nonce, dir_fd=parent.fd, follow_symlinks=False)
        staging_identity = directory_binding(staged)
        if directory_binding(named) != staging_identity:
            fail("r29 staging directory changed before publication")
        _require_directory_roster(
            staging_fd, {"report.json", "roster.json"}, "r29 staging directory"
        )
        _read_exact_bound_file(
            staging_fd,
            "report.json",
            report,
            staged_files["report.json"],
            "staged r29 output",
        )
        _read_exact_bound_file(
            staging_fd,
            "roster.json",
            roster,
            staged_files["roster.json"],
            "staged r29 output",
        )
        _rename_noreplace(parent.fd, nonce, output.name)
        staging_named = False
        os.fsync(parent.fd)
        _validate_published_bundle(
            parent.fd,
            output.name,
            staging_identity,
            staged_files,
            roster,
            report,
        )
    finally:
        if staging_named and staging_fd >= 0 and staging_identity is not None:
            try:
                named = os.stat(nonce, dir_fd=parent.fd, follow_symlinks=False)
            except OSError:
                named = None
            if named is not None and directory_binding(named) == staging_identity:
                for name, identity in staged_files.items():
                    try:
                        current = os.stat(
                            name, dir_fd=staging_fd, follow_symlinks=False
                        )
                    except OSError:
                        continue
                    if file_snapshot(current) == identity:
                        try:
                            os.unlink(name, dir_fd=staging_fd)
                        except OSError:
                            pass
                try:
                    if not os.listdir(staging_fd):
                        os.rmdir(nonce, dir_fd=parent.fd)
                except OSError:
                    pass
        if staging_fd >= 0:
            os.close(staging_fd)
        parent.close()


def validate(intake: Path, output: Path) -> None:
    expected_roster, expected_report = build_documents(intake)
    root = SecureRoot(output, "r29 output bundle", private=True)
    try:
        if root.names() != {"report.json", "roster.json"}:
            fail("r29 output bundle roster drifted")
        actual_report = root.read("report.json", "r29 report", MAX_JSON_BYTES)
        actual_roster = root.read("roster.json", "r29 artifact roster", MAX_JSON_BYTES)
        parse_canonical(actual_report, "r29 report")
        parse_canonical(actual_roster, "r29 artifact roster")
        if actual_report != expected_report or actual_roster != expected_roster:
            fail("r29 output bundle differs from the authenticated intake")
    finally:
        root.close()


def main(arguments: list[str]) -> None:
    if len(arguments) != 3 or arguments[0] not in {"produce", "validate"}:
        fail(
            "usage: validate-r29-differential-evidence.py "
            "{produce|validate} INTAKE-ROOT OUTPUT-BUNDLE"
        )
    command, intake, output = arguments
    if command == "produce":
        produce(Path(intake), Path(output))
    else:
        validate(Path(intake), Path(output))
    print(f"PASS: r29 differential {command} partial-non-evidence")


if __name__ == "__main__":
    main(sys.argv[1:])
