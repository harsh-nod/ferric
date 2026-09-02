#!/usr/bin/env python3
"""Produce one noncurrent aggregate publication-selection candidate."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tomllib
from types import ModuleType
from typing import Any, NoReturn


FORMAT = "FERRIC-M1-ALL-KERNELS-PUBLICATION-SELECTION-CANDIDATE-V1"
AUTHORITY = "publication-selection-candidate-only"
STATUS = "noncurrent-candidate"
NONCLAIM = (
    "This owner-private candidate binds one validated observational Worker V3 build record "
    "to exact source, target, COV6, ordered roster, source-pin, envelope, and finalized-HSACO "
    "identities. It does not authenticate compiler origin, finalization, verifier authority, "
    "or current publication custody; select a runtime artifact; grant load, launch, dispatch, "
    "or inference authority; establish Qwen execution, correctness, or performance; close any "
    "M1 obligation; or modify Ferric's private unavailable current selection."
)
ESTABLISHED = [
    "canonical-observational-build-record-validation",
    "noncurrent-publication-selection-candidate-identity-binding",
    "owner-private-no-replace-candidate-publication",
]
EXCLUDED = [
    "compiler-origin-authentication",
    "current-publication-custody",
    "durable-publication-authentication",
    "gpu-dispatch",
    "gpu-launch",
    "gpu-load",
    "m1-qualification",
    "numerical-correctness",
    "performance",
    "qwen-execution",
    "runtime-selection",
    "verifier-authority",
    "worker-v3-finalization-authentication",
]
ADAPTER_BINDING_NONCLAIM = (
    "This source-controlled record pre-binds one executable identity to the exact adapter "
    "source closure. It is not a reproducible-build proof, compiler-origin attestation, "
    "semantic-correctness proof, or runtime authority."
)


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_validator() -> ModuleType:
    path = Path(__file__).with_name(
        "validate-protected-worker-v3-all-kernels-build.py"
    )
    specification = importlib.util.spec_from_file_location(
        "_ferric_aggregate_worker_v3_build_validator", path
    )
    if specification is None or specification.loader is None:
        fail("cannot load aggregate Worker V3 build validator")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


VALIDATOR = load_validator()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def git(repository: Path, arguments: list[str], description: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repository), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=30,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"cannot {description}: {result.stdout.strip()}")
    return result.stdout.strip()


def git_bytes(repository: Path, arguments: list[str], description: str) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(repository), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"cannot {description}: {result.stdout[:1000]!r}")
    return result.stdout


def source_identity(repository: Path, description: str) -> dict[str, str]:
    if not repository.is_absolute() or repository.is_symlink() or not repository.is_dir():
        fail(f"{description} must be an absolute nonsymlink Git worktree")
    if git(repository, ["status", "--porcelain=v1", "--untracked-files=all"], description):
        fail(f"{description} must be an exact clean worktree")
    commit = git(repository, ["rev-parse", "HEAD"], f"resolve {description} commit")
    tree = git(repository, ["rev-parse", "HEAD^{tree}"], f"resolve {description} tree")
    VALIDATOR.git_id(commit, f"{description} commit")
    VALIDATOR.git_id(tree, f"{description} tree")
    return {"commit": commit, "tree": tree}


def same_file_identity(left: os.stat_result, right: os.stat_result) -> bool:
    return (
        left.st_dev,
        left.st_ino,
        left.st_size,
        left.st_mtime_ns,
    ) == (
        right.st_dev,
        right.st_ino,
        right.st_size,
        right.st_mtime_ns,
    )


def same_directory_identity(left: os.stat_result, right: os.stat_result) -> bool:
    return (
        left.st_dev,
        left.st_ino,
        left.st_uid,
        stat.S_IMODE(left.st_mode),
    ) == (
        right.st_dev,
        right.st_ino,
        right.st_uid,
        stat.S_IMODE(right.st_mode),
    )


def bind_source_records(
    repository: Path,
    commit: str,
    records: list[dict[str, Any]],
    description: str,
) -> dict[str, bytes]:
    held: dict[str, bytes] = {}
    for record in records:
        relative = record["path"]
        object_name = f"{commit}:{relative}"
        blob = git(
            repository,
            ["rev-parse", object_name],
            f"resolve {description} Git blob {relative}",
        )
        data = git_bytes(
            repository,
            ["cat-file", "blob", object_name],
            f"read {description} Git blob {relative}",
        )
        if (
            blob != record["git_blob"]
            or hashlib.sha256(data).hexdigest() != record["sha256"]
            or len(data) != record["size_bytes"]
        ):
            fail(f"{description} record does not bind exact committed bytes: {relative}")
        held[relative] = data
    return held


def bind_adapter_binding(
    repository: Path, commit: str, prebinding: dict[str, Any]
) -> None:
    relative = VALIDATOR.ADAPTER_BINDING
    object_name = f"{commit}:{relative}"
    blob = git(
        repository,
        ["rev-parse", object_name],
        "resolve aggregate adapter binding Git blob",
    )
    data = git_bytes(
        repository,
        ["cat-file", "blob", object_name],
        "read aggregate adapter binding Git blob",
    )
    if (
        blob != prebinding["binding_git_blob"]
        or hashlib.sha256(data).hexdigest() != prebinding["binding_sha256"]
    ):
        fail("aggregate adapter binding record does not bind exact committed bytes")
    try:
        value = json.loads(data)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"aggregate adapter binding record is invalid: {error}")
    if not isinstance(value, dict) or data != VALIDATOR.canonical_bytes(value):
        fail("aggregate adapter binding record is not canonical ASCII JSON")
    binary = value.get("binary")
    if (
        set(value)
        != {
            "authority",
            "binary",
            "format",
            "nonclaim",
            "protocol",
            "source_closure_sha256",
            "source_files",
        }
        or value["authority"] != "binary-identity-prebinding-only"
        or value["format"]
        != "FERRIC-M1-ALL-KERNELS-SOURCE-PIN-ADAPTER-BINDING-V1"
        or value["nonclaim"] != ADAPTER_BINDING_NONCLAIM
        or value["protocol"]
        != "ferric.m1-all-kernels-worker-v3-source-pin.v1"
        or value["source_closure_sha256"] != prebinding["source_closure_sha256"]
        or value["source_files"] != prebinding["source_files"]
        or not isinstance(binary, dict)
        or set(binary) != {"name", "sha256", "size_bytes"}
        or binary["name"] != prebinding["name"]
        or binary["sha256"] != prebinding["binary_sha256"]
        or binary["size_bytes"] != prebinding["binary_size_bytes"]
    ):
        fail("aggregate adapter binding JSON does not bind the claimed prebinding")


def validate_provider_manifest(manifest: bytes, provider_commit: str) -> None:
    try:
        value = tomllib.loads(manifest.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        fail(f"aggregate device manifest is invalid: {error}")
    if not isinstance(value, dict):
        fail("aggregate device manifest root is not a table")
    dependencies = value.get("dependencies", {})
    targets = value.get("target", {})
    if not isinstance(dependencies, dict) or not isinstance(targets, dict):
        fail("aggregate device manifest dependency tables drifted")
    host_target = targets.get('cfg(not(target_arch = "amdgpu"))', {})
    if not isinstance(host_target, dict):
        fail("aggregate device manifest host target table drifted")
    target_dependencies = host_target.get("dependencies", {})
    if not isinstance(target_dependencies, dict):
        fail("aggregate device manifest host dependencies drifted")
    device = dependencies.get("fe2o3-device", {})
    host = target_dependencies.get("fe2o3-host", {})
    package = value.get("package", {})
    if (
        not isinstance(package, dict)
        or not isinstance(device, dict)
        or not isinstance(host, dict)
        or package.get("name") != "ferric-qwen3-all-kernels-device-v1"
        or device.get("git") != "https://github.com/harsh-nod/fe2o3.git"
        or host.get("git") != "https://github.com/harsh-nod/fe2o3.git"
        or device.get("rev") != provider_commit
        or host.get("rev") != provider_commit
    ):
        fail("aggregate device manifest does not bind the exact provider revision")


def revalidate_record(path: Path, expected: os.stat_result) -> None:
    try:
        current = path.lstat()
    except OSError as error:
        fail(f"cannot revalidate aggregate protected-build record: {error}")
    if stat.S_ISLNK(current.st_mode) or not same_file_identity(current, expected):
        fail("aggregate protected-build record changed custody")


def publish(path: Path, data: bytes) -> None:
    if not path.is_absolute() or path.name in {"", ".", ".."}:
        fail("selection-candidate output must be an absolute single-file path")
    parent = path.parent
    try:
        parent_metadata = parent.lstat()
    except OSError as error:
        fail(f"cannot inspect selection-candidate output directory: {error}")
    if (
        stat.S_ISLNK(parent_metadata.st_mode)
        or not stat.S_ISDIR(parent_metadata.st_mode)
        or parent_metadata.st_uid != os.geteuid()
        or stat.S_IMODE(parent_metadata.st_mode) != 0o700
    ):
        fail("selection-candidate output directory must be owner-private and nonsymlink")
    directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW
    try:
        directory = os.open(parent, directory_flags)
    except OSError as error:
        fail(f"cannot hold selection-candidate output directory: {error}")
    descriptor = -1
    try:
        opened_parent = os.fstat(directory)
        named_parent = parent.lstat()
        if (
            not same_directory_identity(parent_metadata, opened_parent)
            or not same_directory_identity(opened_parent, named_parent)
        ):
            fail("selection-candidate output directory changed during open")
        descriptor = os.open(
            path.name,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW,
            0o600,
            dir_fd=directory,
        )
        with os.fdopen(descriptor, "wb", closefd=False) as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        held = os.fstat(descriptor)
        named = os.stat(path.name, dir_fd=directory, follow_symlinks=False)
        if (
            not stat.S_ISREG(held.st_mode)
            or held.st_nlink != 1
            or held.st_uid != os.geteuid()
            or stat.S_IMODE(held.st_mode) != 0o600
            or held.st_size != len(data)
            or not same_file_identity(held, named)
        ):
            fail("selection-candidate output lost owner-private file custody")
        os.fsync(directory)
        final_named = os.stat(path.name, dir_fd=directory, follow_symlinks=False)
        final_parent = parent.lstat()
        if (
            not same_file_identity(held, final_named)
            or not same_directory_identity(opened_parent, final_parent)
        ):
            fail("selection-candidate output path changed during final publication sync")
    except FileExistsError:
        fail("selection-candidate output already exists; replacement is forbidden")
    except OSError as error:
        fail(f"cannot publish selection candidate: {error}")
    finally:
        if descriptor >= 0:
            os.close(descriptor)
        os.close(directory)


def main() -> None:
    if len(sys.argv) != 5:
        fail(
            "usage: produce-protected-worker-v3-all-kernels-publication-selection.py "
            "FERRIC_SOURCE_REPO FE2O3_SOURCE_REPO OBSERVATIONAL_BUILD_RECORD OUTPUT"
        )
    source_repo, compiler_repo, record_path, output_path = map(Path, sys.argv[1:])
    record, record_bytes, record_metadata = VALIDATOR.load_and_validate(record_path)
    ferric = source_identity(source_repo, "Ferric selection source")
    compiler = source_identity(compiler_repo, "fe2o3 selection source")
    if ferric != {key: record["source"][key] for key in ("commit", "tree")}:
        fail("observational record does not bind the exact clean Ferric source")
    observed_compiler = record["observed_compiler_inputs"]
    if compiler != {key: observed_compiler[key] for key in ("commit", "tree")}:
        fail("observational record does not bind the exact clean fe2o3 source")
    device_sources = bind_source_records(
        source_repo,
        ferric["commit"],
        record["source"]["device_files"],
        "aggregate device source",
    )
    prebinding = record["source_pin_observation"]["adapter_prebinding"]
    bind_source_records(
        source_repo,
        ferric["commit"],
        prebinding["source_files"],
        "aggregate adapter source",
    )
    bind_adapter_binding(source_repo, ferric["commit"], prebinding)
    provider_commit = record["source"]["device_provider_commit"]
    validate_provider_manifest(
        device_sources["device/qwen3-all-kernels-v1/Cargo.toml"], provider_commit
    )
    provider_tree = git(
        compiler_repo,
        ["rev-parse", f"{provider_commit}^{{tree}}"],
        "resolve aggregate device-provider tree",
    )
    if provider_tree != record["source"]["device_provider_tree"]:
        fail("observational record device-provider source identity drifted")

    projection = record["source_pin_observation"]["projection"]
    readiness = record["observed_worker_v3_records"]["receipt_checksum_observation"]
    artifact = record["artifact"]
    selection = {
        "code_object_version": projection["code_object_version"],
        "ferric_source": ferric,
        "fe2o3_source": compiler,
        "finalized_artifact_length": artifact["size_bytes"],
        "finalized_artifact_sha256": artifact["sha256"],
        "kernel_symbols": projection["policy_kernel_symbols"],
        "provider_source": {
            "commit": provider_commit,
            "tree": provider_tree,
        },
        "source_pin": projection["source_pin"],
        "target": projection["target"],
        "worker_v3_envelope_length": readiness["envelope_size_bytes"],
        "worker_v3_envelope_sha256": readiness["envelope_sha256"],
    }
    candidate = {
        "authority": AUTHORITY,
        "established_claims": ESTABLISHED,
        "excluded_claims": EXCLUDED,
        "format": FORMAT,
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "observational_build_record": {
            "sha256": hashlib.sha256(record_bytes).hexdigest(),
            "size_bytes": len(record_bytes),
        },
        "selection": selection,
        "status": STATUS,
    }
    encoded = canonical_bytes(candidate)
    revalidate_record(record_path, record_metadata)
    if source_identity(source_repo, "Ferric selection source") != ferric:
        fail("Ferric selection source changed during candidate production")
    if source_identity(compiler_repo, "fe2o3 selection source") != compiler:
        fail("fe2o3 selection source changed during candidate production")
    publish(output_path, encoded)
    print(
        "PASS: published noncurrent aggregate publication-selection candidate "
        f"sha256={hashlib.sha256(encoded).hexdigest()}"
    )


if __name__ == "__main__":
    main()
