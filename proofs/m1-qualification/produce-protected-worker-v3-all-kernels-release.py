#!/usr/bin/env python3
"""Prepare and run the exact protected Qwen aggregate Worker V3 release."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tomllib
from typing import Any, NoReturn


FE2O3_REVISION = "b2cce9c271e85a97c35ce7a1ccffe17bb330f07c"
PLIRON_REVISION = "5bdf861bf03e7f20242b25717fb653336d02e487"
DEVICE_RELATIVE = Path("device/qwen3-all-kernels-v1")
DEVICE_CRATE = "ferric-qwen3-all-kernels-device-v1"
DEVICE_CRATE_RUST = "ferric_qwen3_all_kernels_device_v1"
CONFIG_FORMAT = "fe2o3-production-build-config-v2"
CONFIG_DOMAIN = b"fe2o3-build-config-transitive-v2"
CONFIG_PROFILE = b"production-v2"
OBSERVATION_KIND = "source-isa-summary-v1"
CLIENT_PROFILE = Path("/etc/fe2o3/compiler-execution/client-profile-v1")
SUPERVISOR_SOCKET = Path("/run/fe2o3/compiler-execution-supervisor.sock")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
WORKER_BUILD_ID = re.compile(r"fe2o3-worker-v1-sha256-[0-9a-f]{64}\Z")
MAX_WORKER_BYTES = 512 * 1024 * 1024
MAX_CONFIG_BYTES = 1024 * 1024
KERNELS = (
    "ferric_qwen3_lowest_id_argmax_bf16_v1",
    "ferric_qwen3_gemm_vector_a4_bf16_f32_bf16_v1",
    "qwen3_rope_v1",
    "ferric_qwen3_compact_completion_v1",
    "qwen3_paged_kv_write_v1",
    "qwen3_paged_gqa_decode_bf16_f32_v1",
    "qwen3_swiglu_bf16_f32_v1",
    "ferric_qwen3_gemm_reference_bf16_f32_bf16_v1",
    "qwen3_rmsnorm_v1",
    "ferric_qwen3_token_embedding_bf16_copy_v1",
    "ferric_qwen3_speculative_token_assembly_v1",
    "qwen3_gqa_prefill_causal_bf16_f32_v1",
)


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def compact(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_absolute(path: Path, description: str) -> Path:
    if not path.is_absolute():
        fail(f"{description} path must be absolute")
    try:
        canonical = path.resolve(strict=True)
    except OSError as error:
        fail(f"cannot resolve {description}: {error}")
    if canonical != path:
        fail(f"{description} path must be canonical and may not use aliases")
    return path


def git(repository: Path, arguments: list[str], description: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repository), *arguments],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=30,
            env={"LANG": "C", "LC_ALL": "C", "PATH": "/usr/bin:/bin", "TZ": "UTC"},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"cannot {description}: {error}")
    try:
        output = result.stdout.decode("ascii")
    except UnicodeDecodeError as error:
        fail(f"non-ASCII output while trying to {description}: {error}")
    if result.returncode != 0:
        fail(f"cannot {description}: {output.strip()}")
    return output.strip()


def clean_repository(repository: Path, description: str) -> str:
    canonical_absolute(repository, description)
    if not repository.is_dir():
        fail(f"{description} is not a directory")
    head = git(repository, ["rev-parse", "HEAD"], f"resolve {description} HEAD")
    if re.fullmatch(r"[0-9a-f]{40}", head) is None:
        fail(f"{description} HEAD is not a full Git identity")
    status = git(
        repository,
        ["status", "--porcelain=v1", "--untracked-files=all"],
        f"inspect {description}",
    )
    if status:
        fail(f"{description} must be an exact clean Git worktree")
    return head


def require_fe2o3_repository(repository: Path) -> None:
    head = clean_repository(repository, "fe2o3 source repository")
    if head != FE2O3_REVISION:
        fail(
            "fe2o3 source repository must be exact public main revision "
            f"{FE2O3_REVISION}; found {head}"
        )


def require_ferric_device(repository: Path) -> Path:
    clean_repository(repository, "Ferric source repository")
    device = repository / DEVICE_RELATIVE
    manifest_path = device / "Cargo.toml"
    lock_path = device / "Cargo.lock"
    source_path = device / "src/lib.rs"
    try:
        manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
        lock = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot read aggregate device dependency closure: {error}")
    package = manifest.get("package", {})
    device_dependency = manifest.get("dependencies", {}).get("fe2o3-device", {})
    host_dependency = (
        manifest.get("target", {})
        .get('cfg(not(target_arch = "amdgpu"))', {})
        .get("dependencies", {})
        .get("fe2o3-host", {})
    )
    expected_dependency = {
        "git": "https://github.com/harsh-nod/fe2o3.git",
        "rev": FE2O3_REVISION,
        "version": "=0.1.0",
    }
    if (
        package.get("name") != DEVICE_CRATE
        or device_dependency != expected_dependency
        or host_dependency != expected_dependency
        or not source_path.is_file()
    ):
        fail("aggregate device crate is not exactly pinned to public fe2o3 main")
    expected_source = (
        "git+https://github.com/harsh-nod/fe2o3.git?rev="
        f"{FE2O3_REVISION}#{FE2O3_REVISION}"
    )
    locked = {
        item.get("name"): item.get("source")
        for item in lock.get("package", [])
        if item.get("name") in {"fe2o3-device", "fe2o3-host"}
    }
    if locked != {"fe2o3-device": expected_source, "fe2o3-host": expected_source}:
        fail("aggregate device lockfile is not exactly pinned to public fe2o3 main")
    git_sources = {
        item.get("source")
        for item in lock.get("package", [])
        if isinstance(item.get("source"), str)
        and item["source"].startswith("git+")
    }
    if git_sources != {
        expected_source,
        "git+https://github.com/harsh-nod/pliron.git?rev="
        f"{PLIRON_REVISION}#{PLIRON_REVISION}",
    }:
        fail("aggregate device lockfile Git source closure drifted")
    return device


def held_regular(path: Path, description: str, maximum: int) -> tuple[bytes, os.stat_result]:
    canonical_absolute(path, description)
    flags = os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW
    try:
        before = path.stat(follow_symlinks=False)
        descriptor = os.open(path, flags)
        with os.fdopen(descriptor, "rb") as source:
            opened = os.fstat(source.fileno())
            data = source.read(maximum + 1)
            after = os.fstat(source.fileno())
        named = path.stat(follow_symlinks=False)
    except OSError as error:
        fail(f"cannot retain {description}: {error}")
    identity = lambda value: (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_uid,
        value.st_gid,
        value.st_nlink,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )
    if (
        not stat.S_ISREG(before.st_mode)
        or opened.st_nlink != 1
        or opened.st_size <= 0
        or opened.st_size > maximum
        or len(data) != opened.st_size
        or identity(before) != identity(opened)
        or identity(opened) != identity(after)
        or identity(after) != identity(named)
    ):
        fail(f"{description} is outside the stable bounded regular-file policy")
    return data, opened


def read_identity(path: Path, description: str, pattern: re.Pattern[str]) -> str:
    data, _ = held_regular(path, description, 4096)
    try:
        value = data.decode("ascii").strip()
    except UnicodeDecodeError as error:
        fail(f"{description} is not ASCII: {error}")
    if pattern.fullmatch(value) is None:
        fail(f"{description} has an invalid value")
    return value


def worker_record(worker: Path) -> dict[str, Any]:
    data, metadata = held_regular(worker, "Worker V3 linker", MAX_WORKER_BYTES)
    if metadata.st_mode & 0o111 == 0:
        fail("Worker V3 linker is not executable")
    llvm_identity = read_identity(
        worker.parent / "fe2o3-llvm-build-id.txt",
        "Worker V3 LLVM build identity",
        re.compile(r"[0-9]+(?:\.[0-9]+)+\Z"),
    )
    if llvm_identity != "7.2.4":
        fail("Worker V3 LLVM build identity must be exactly 7.2.4")
    worker_identity = read_identity(
        worker.parent / "fe2o3-worker-build-id.txt",
        "Worker V3 build identity",
        WORKER_BUILD_ID,
    )
    return {
        "byte_len": len(data),
        "llvm_build_identity": llvm_identity,
        "path": str(worker),
        "sha256": sha256(data),
        "worker_build_identity": worker_identity,
    }


def production_config(device: Path, worker: dict[str, Any]) -> dict[str, Any]:
    return {
        "candidate_output_max_bytes": 4_194_304,
        "format": CONFIG_FORMAT,
        "limits": {
            "stderr_bytes": 65_536,
            "stdout_bytes": 8_388_608,
            "timeout_ms": 120_000,
        },
        "link_options": [
            {"name": "code-object-version", "value": "6"},
            {"name": "opt-level", "value": "2"},
            {"name": "strip-debug", "value": "true"},
            {"name": "verify-each", "value": "true"},
        ],
        "observation": {"kind": OBSERVATION_KIND},
        "providers": [],
        "units": [
            {
                "crate_name": DEVICE_CRATE_RUST,
                "source": "src/lib.rs",
                "working_directory": str(device),
            }
        ],
        "worker": worker,
    }


def update_identity(digest: Any, value: bytes) -> None:
    digest.update(len(value).to_bytes(8, "little"))
    digest.update(value)


def config_identity(raw: bytes, worker: dict[str, Any]) -> str:
    digest = hashlib.sha256()
    for value in [
        CONFIG_DOMAIN,
        CONFIG_PROFILE,
        raw,
        bytes.fromhex(worker["sha256"]),
        worker["byte_len"].to_bytes(8, "little"),
        worker["worker_build_identity"].encode("ascii"),
        worker["llvm_build_identity"].encode("ascii"),
        (0).to_bytes(8, "little"),
    ]:
        update_identity(digest, value)
    return digest.hexdigest()


def private_new_output(path: Path, data: bytes, description: str) -> None:
    if not path.is_absolute():
        fail(f"{description} path must be absolute")
    parent = path.parent
    canonical_absolute(parent, f"{description} parent")
    metadata = parent.stat(follow_symlinks=False)
    if (
        not stat.S_ISDIR(metadata.st_mode)
        or metadata.st_uid != os.geteuid()
        or metadata.st_mode & 0o077 != 0
    ):
        fail(f"{description} parent must be an owner-private directory")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW
    try:
        descriptor = os.open(path, flags, 0o600)
        with os.fdopen(descriptor, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
    except OSError as error:
        fail(f"cannot publish {description} without replacement: {error}")


def prepare_config(ferric: Path, worker_path: Path, output: Path) -> None:
    device = require_ferric_device(ferric)
    worker = worker_record(worker_path)
    raw = compact(production_config(device, worker))
    private_new_output(output, raw, "production configuration")
    print(
        "PASS: published aggregate production configuration "
        f"sha256={sha256(raw)} identity={config_identity(raw, worker)} "
        f"worker_sha256={worker['sha256']}"
    )


def exact_config(path: Path, ferric: Path) -> tuple[bytes, dict[str, Any]]:
    raw, _ = held_regular(path, "production configuration", MAX_CONFIG_BYTES)
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"production configuration is not JSON: {error}")
    if not isinstance(value, dict) or raw != compact(value):
        fail("production configuration is not canonical compact ASCII JSON")
    device = ferric / DEVICE_RELATIVE
    expected = production_config(device, value.get("worker"))
    if value != expected or not isinstance(value.get("worker"), dict):
        fail("production configuration does not select the exact aggregate recipe")
    worker = value["worker"]
    if set(worker) != {
        "byte_len",
        "llvm_build_identity",
        "path",
        "sha256",
        "worker_build_identity",
    } or (
        not isinstance(worker["byte_len"], int)
        or isinstance(worker["byte_len"], bool)
        or worker["byte_len"] <= 0
        or not isinstance(worker["path"], str)
        or not isinstance(worker["sha256"], str)
        or SHA256.fullmatch(worker["sha256"]) is None
        or not isinstance(worker["llvm_build_identity"], str)
        or worker["llvm_build_identity"] != "7.2.4"
        or not isinstance(worker["worker_build_identity"], str)
        or WORKER_BUILD_ID.fullmatch(worker["worker_build_identity"]) is None
    ):
        fail("production configuration worker fields drifted")
    measured = worker_record(Path(worker["path"]))
    if worker != measured:
        fail("production configuration Worker V3 measurement drifted")
    return raw, worker


def validate_protected_infrastructure(profile: Path, socket: Path) -> None:
    try:
        profile_metadata = profile.stat(follow_symlinks=False)
    except OSError as error:
        fail(f"protected compiler client profile is unavailable at {profile}: {error}")
    if (
        not stat.S_ISREG(profile_metadata.st_mode)
        or profile_metadata.st_uid != 0
        or profile_metadata.st_nlink != 1
        or profile_metadata.st_size <= 0
        or profile_metadata.st_size > 1024 * 1024
        or profile_metadata.st_mode & 0o022 != 0
    ):
        fail("protected compiler client profile is not a bounded root-owned policy file")
    try:
        socket_metadata = socket.stat(follow_symlinks=False)
    except OSError as error:
        fail(f"protected compiler supervisor socket is unavailable at {socket}: {error}")
    if not stat.S_ISSOCK(socket_metadata.st_mode):
        fail("protected compiler supervisor endpoint is not a Unix socket")


def executable(path: Path, description: str) -> str:
    _, metadata = held_regular(path, description, 512 * 1024 * 1024)
    if metadata.st_mode & 0o111 == 0:
        fail(f"{description} is not executable")
    return str(path)


def digest_file(path: Path, description: str) -> str:
    data, _ = held_regular(path, description, 512 * 1024 * 1024)
    return sha256(data)


def run_build(arguments: argparse.Namespace) -> None:
    validate_protected_infrastructure(CLIENT_PROFILE, SUPERVISOR_SOCKET)
    device = require_ferric_device(arguments.ferric)
    require_fe2o3_repository(arguments.fe2o3)
    raw, worker = exact_config(arguments.config, arguments.ferric)
    target = arguments.target
    if not target.is_absolute() or target.exists():
        fail("protected target directory must be an absolute absent path")
    canonical_absolute(target.parent, "protected target parent")
    target_parent = target.parent.stat(follow_symlinks=False)
    if target_parent.st_uid != os.geteuid() or target_parent.st_mode & 0o077 != 0:
        fail("protected target parent must be an owner-private directory")
    cargo_fe2o3 = executable(arguments.cargo_fe2o3, "cargo-fe2o3")
    cargo = executable(arguments.cargo, "Cargo")
    trampoline = executable(arguments.trampoline, "Cargo binding trampoline")
    rustc = executable(arguments.rustc, "rustc")
    backend = canonical_absolute(arguments.backend, "codegen backend")
    runtime_sha256 = arguments.rustc_runtime_sha256
    if SHA256.fullmatch(runtime_sha256) is None or runtime_sha256 == "0" * 64:
        fail("rustc runtime-tree SHA-256 is invalid")
    environment = {
        "CARGO": cargo,
        "FE2O3_AUTHORITY_BACKEND_SHA256_V1": digest_file(backend, "codegen backend"),
        "FE2O3_AUTHORITY_CARGO_SHA256_V1": digest_file(arguments.cargo, "Cargo"),
        "FE2O3_AUTHORITY_CARGO_BINDING_TRAMPOLINE_PATH_V1": trampoline,
        "FE2O3_AUTHORITY_CARGO_BINDING_TRAMPOLINE_SHA256_V1": digest_file(
            arguments.trampoline, "Cargo binding trampoline"
        ),
        "FE2O3_AUTHORITY_RUSTC_PATH_V1": rustc,
        "FE2O3_AUTHORITY_RUSTC_RUNTIME_SHA256_V1": runtime_sha256,
        "FE2O3_AUTHORITY_RUSTC_SHA256_V1": digest_file(arguments.rustc, "rustc"),
        "FE2O3_BACKEND": str(backend),
        "FE2O3_PRODUCTION_BUILD_CONFIG_V2": str(arguments.config),
        "FE2O3_TARGET": "gfx942",
        "LANG": "C",
        "LC_ALL": "C",
        "TZ": "UTC",
    }
    command = [
        cargo_fe2o3,
        "authority",
        "release",
        "build",
        "--locked",
        "--target-dir",
        str(target),
    ]
    try:
        result = subprocess.run(command, cwd=device, env=environment, check=False)
    except OSError as error:
        fail(f"cannot launch protected aggregate build: {error}")
    if result.returncode != 0:
        fail(f"protected aggregate build failed with status {result.returncode}")
    after, _ = held_regular(
        arguments.config, "production configuration after build", MAX_CONFIG_BYTES
    )
    if after != raw:
        fail("production configuration changed across the protected build")
    print(
        "PASS: protected aggregate build completed "
        f"config_identity={config_identity(raw, worker)} artifact_root={target / 'fe2o3'}"
    )


def run_evidence(arguments: argparse.Namespace) -> None:
    require_ferric_device(arguments.ferric)
    require_fe2o3_repository(arguments.fe2o3)
    exact_config(arguments.config, arguments.ferric)
    if arguments.record.exists() or arguments.candidate.exists():
        fail("record and candidate outputs must both be absent")
    commands = [
        [
            sys.executable,
            "-I",
            "-B",
            str(
                arguments.ferric
                / "proofs/m1-qualification/produce-protected-worker-v3-all-kernels-build.py"
            ),
            str(arguments.ferric),
            str(arguments.fe2o3),
            str(arguments.config),
            str(arguments.artifact_root),
            str(arguments.cargo_fe2o3),
            str(arguments.rustc_wrapper),
            str(arguments.backend),
            str(arguments.source_pin_adapter),
            str(arguments.record),
        ],
        [
            sys.executable,
            "-I",
            "-B",
            str(
                arguments.ferric
                / "proofs/m1-qualification/validate-protected-worker-v3-all-kernels-build.py"
            ),
            str(arguments.record),
        ],
        [
            sys.executable,
            "-I",
            "-B",
            str(
                arguments.ferric
                / "proofs/m1-qualification/produce-protected-worker-v3-all-kernels-publication-selection.py"
            ),
            str(arguments.ferric),
            str(arguments.fe2o3),
            str(arguments.record),
            str(arguments.candidate),
        ],
    ]
    environment = {
        "LANG": "C",
        "LC_ALL": "C",
        "PATH": "/usr/local/bin:/usr/bin:/bin",
        "TZ": "UTC",
    }
    for command in commands:
        try:
            result = subprocess.run(
                command, cwd=arguments.ferric, env=environment, check=False
            )
        except OSError as error:
            fail(f"cannot run aggregate evidence stage: {error}")
        if result.returncode != 0:
            fail(f"aggregate evidence stage failed with status {result.returncode}")
    record = digest_file(arguments.record, "aggregate build record")
    candidate = digest_file(arguments.candidate, "aggregate publication candidate")
    print(
        "PASS: published aggregate evidence candidate "
        f"record_sha256={record} candidate_sha256={candidate}"
    )


def exact_engineering_observation(output_root: Path) -> tuple[Path, str, str]:
    try:
        entries = list(output_root.iterdir())
    except OSError as error:
        fail(f"cannot enumerate engineering observation: {error}")
    if len(entries) != 1 or not entries[0].is_dir() or entries[0].is_symlink():
        fail("engineering namespace does not contain one exact content directory")
    content = entries[0]
    try:
        names = {entry.name for entry in content.iterdir()}
    except OSError as error:
        fail(f"cannot enumerate engineering content: {error}")
    if names != {"observation.hsaco", "observation.json"}:
        fail("engineering content roster drifted")
    manifest_raw, _ = held_regular(
        content / "observation.json", "engineering observation manifest", MAX_CONFIG_BYTES
    )
    hsaco, _ = held_regular(
        content / "observation.hsaco", "engineering HSACO", 4_194_304
    )
    try:
        manifest = json.loads(manifest_raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"engineering observation manifest is invalid: {error}")
    if not isinstance(manifest, dict):
        fail("engineering observation manifest root is not an object")
    expected_options = {
        "maximum_output_bytes": 4_194_304,
        "optimization": "O2",
        "strip_debug": True,
        "timeout_seconds": 120,
        "verify_each": True,
    }
    hsaco_record = manifest.get("hsaco")
    if not isinstance(hsaco_record, dict):
        fail("engineering observation HSACO record is not an object")
    identity = hsaco_record.get("identity")
    names = hsaco_record.get("kernel_names")
    if (
        manifest.get("schema") != "EngineeringHsacoObservationV1"
        or manifest.get("namespace") != "fe2o3-engineering-v1"
        or manifest.get("authority") != "none"
        or manifest.get("artifact") != "observation.hsaco"
        or manifest.get("crate_name") != DEVICE_CRATE_RUST
        or manifest.get("target") != "gfx942:xnack-"
        or manifest.get("code_object_version") != 6
        or manifest.get("providers") != []
        or manifest.get("options") != expected_options
        or manifest.get("grants")
        != {"launch": False, "load": False, "publication": False}
        or not isinstance(names, list)
        or names != list(KERNELS)
        or identity
        != {"byte_len": len(hsaco), "sha256": sha256(hsaco)}
    ):
        fail("engineering observation does not describe the exact authority-free aggregate")
    return content, sha256(manifest_raw), sha256(hsaco)


def prepare_engineering_vendor(arguments: argparse.Namespace) -> None:
    require_ferric_device(arguments.ferric)
    require_fe2o3_repository(arguments.fe2o3)
    vendor = canonical_absolute(arguments.cargo_vendor, "Cargo vendor directory")
    package = vendor / "fe2o3-device-0.1.0"
    canonical_absolute(package, "vendored fe2o3-device package")
    source = arguments.fe2o3 / "crates/fe2o3-device"
    source_files = {
        path.relative_to(source / "src"): path
        for path in (source / "src").rglob("*.rs")
        if path.is_file() and not path.is_symlink()
    }
    vendor_files = {
        path.relative_to(package / "src"): path
        for path in (package / "src").rglob("*.rs")
        if path.is_file() and not path.is_symlink()
    }
    if source_files.keys() != vendor_files.keys() or any(
        source_files[relative].read_bytes() != vendor_files[relative].read_bytes()
        for relative in source_files
    ):
        fail("vendored fe2o3-device Rust source differs from exact public main")
    manifest, _ = held_regular(
        source / "Cargo.toml", "reviewed fe2o3-device manifest", MAX_CONFIG_BYTES
    )
    checksum_path = package / ".cargo-checksum.json"
    checksum_raw, _ = held_regular(
        checksum_path, "vendored fe2o3-device checksum", MAX_CONFIG_BYTES
    )
    try:
        checksum = json.loads(checksum_raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"vendored fe2o3-device checksum is invalid: {error}")
    if (
        not isinstance(checksum, dict)
        or set(checksum) != {"files", "package"}
        or checksum["package"] is not None
        or not isinstance(checksum["files"], dict)
        or "Cargo.toml" not in checksum["files"]
    ):
        fail("vendored fe2o3-device checksum fields drifted")
    checksum["files"]["Cargo.toml"] = sha256(manifest)
    replacement = compact(checksum)
    manifest_path = package / "Cargo.toml"
    temporary = package / ".ferric-cargo-checksum-v1.tmp"
    workspace = vendor / "Cargo.toml"
    workspace_bytes, _ = held_regular(
        Path(__file__).with_name("ENGINEERING_VENDOR_WORKSPACE_V1.toml"),
        "engineering vendor workspace template",
        MAX_CONFIG_BYTES,
    )
    if workspace.exists() or temporary.exists():
        fail("engineering vendor overlay already exists")
    try:
        manifest_path.write_bytes(manifest)
        temporary.write_bytes(replacement)
        os.replace(temporary, checksum_path)
        descriptor = os.open(
            workspace,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW,
            0o600,
        )
        with os.fdopen(descriptor, "wb") as output:
            output.write(workspace_bytes)
    except OSError as error:
        fail(f"cannot install engineering vendor source overlay: {error}")
    print(
        "PASS: prepared exact reviewed fe2o3-device vendor overlay "
        f"manifest_sha256={sha256(manifest)}"
    )


def run_engineering(arguments: argparse.Namespace) -> None:
    device = require_ferric_device(arguments.ferric)
    require_fe2o3_repository(arguments.fe2o3)
    output_root = arguments.output_root
    if (
        not output_root.is_absolute()
        or output_root.name != "fe2o3-engineering-v1"
        or output_root.exists()
    ):
        fail("engineering output must be an absent absolute fe2o3-engineering-v1 path")
    canonical_absolute(output_root.parent, "engineering output parent")
    output_parent = output_root.parent.stat(follow_symlinks=False)
    if output_parent.st_uid != os.geteuid() or output_parent.st_mode & 0o077 != 0:
        fail("engineering output parent must be owner-private")
    canonical_absolute(arguments.cargo_vendor, "Cargo vendor directory")
    if not arguments.cargo_vendor.is_dir():
        fail("Cargo vendor path is not a directory")
    worker = worker_record(arguments.worker)
    cargo_fe2o3 = executable(arguments.cargo_fe2o3, "cargo-fe2o3")
    tools = {
        "extractor": (
            executable(arguments.extractor, "fe2o3 rustc extractor"),
            digest_file(arguments.extractor, "fe2o3 rustc extractor"),
        ),
        "extractor-backend": (
            str(canonical_absolute(arguments.extractor_backend, "extractor backend")),
            digest_file(arguments.extractor_backend, "extractor backend"),
        ),
        "cargo": (
            executable(arguments.cargo, "Cargo"),
            digest_file(arguments.cargo, "Cargo"),
        ),
        "rustc": (
            executable(arguments.rustc, "rustc"),
            digest_file(arguments.rustc, "rustc"),
        ),
        "host-linker": (
            executable(arguments.host_linker, "host linker"),
            digest_file(arguments.host_linker, "host linker"),
        ),
        "host-lld": (
            executable(arguments.host_lld, "host lld"),
            digest_file(arguments.host_lld, "host lld"),
        ),
        "host-lld-proxy": (
            executable(arguments.host_lld_proxy, "host lld proxy"),
            digest_file(arguments.host_lld_proxy, "host lld proxy"),
        ),
    }
    command = [
        cargo_fe2o3,
        "engineering",
        "hsaco",
        "--crate",
        DEVICE_CRATE_RUST,
        "--output-root",
        str(output_root),
        "--target",
        "gfx942:xnack-",
        "--code-object-version",
        "6",
    ]
    for option in [
        "extractor",
        "extractor-backend",
        "cargo",
        "rustc",
        "host-linker",
        "host-lld",
        "host-lld-proxy",
    ]:
        path, digest = tools[option]
        command.extend([f"--{option}", path, f"--{option}-sha256", digest])
    command.extend(
        [
            "--worker",
            worker["path"],
            "--worker-sha256",
            worker["sha256"],
            "--worker-build-id",
            worker["worker_build_identity"],
            "--llvm-build-id",
            worker["llvm_build_identity"],
            "--cargo-vendor",
            str(arguments.cargo_vendor),
            "--cargo-git-source",
            f"https://github.com/harsh-nod/fe2o3.git@{FE2O3_REVISION}",
            "--cargo-git-source",
            f"https://github.com/harsh-nod/pliron.git@{PLIRON_REVISION}",
            "--timeout-seconds",
            "120",
            "--max-output-bytes",
            "4194304",
            "--",
            "--manifest-path",
            str(device / "Cargo.toml"),
            "--lib",
        ]
    )
    environment = {"LANG": "C", "LC_ALL": "C", "TZ": "UTC"}
    try:
        result = subprocess.run(command, cwd=device, env=environment, check=False)
    except OSError as error:
        fail(f"cannot launch aggregate engineering build: {error}")
    if result.returncode != 0:
        fail(f"aggregate engineering build failed with status {result.returncode}")
    content, manifest, hsaco = exact_engineering_observation(output_root)
    require_ferric_device(arguments.ferric)
    require_fe2o3_repository(arguments.fe2o3)
    print(
        "PASS: published non-authoritative aggregate engineering observation "
        f"path={content} manifest_sha256={manifest} hsaco_sha256={hsaco}"
    )


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)
    prepare = commands.add_parser("prepare-config")
    prepare.add_argument("ferric", type=Path)
    prepare.add_argument("worker", type=Path)
    prepare.add_argument("output", type=Path)
    prepare.set_defaults(
        entrypoint=lambda args: prepare_config(args.ferric, args.worker, args.output)
    )
    build = commands.add_parser("build")
    for name in [
        "ferric",
        "fe2o3",
        "config",
        "target",
        "cargo_fe2o3",
        "cargo",
        "trampoline",
        "rustc",
    ]:
        build.add_argument(name, type=Path)
    build.add_argument("rustc_runtime_sha256")
    build.add_argument("backend", type=Path)
    build.set_defaults(entrypoint=run_build)
    evidence = commands.add_parser("produce-candidate")
    for name in [
        "ferric",
        "fe2o3",
        "config",
        "artifact_root",
        "cargo_fe2o3",
        "rustc_wrapper",
        "backend",
        "source_pin_adapter",
        "record",
        "candidate",
    ]:
        evidence.add_argument(name, type=Path)
    evidence.set_defaults(entrypoint=run_evidence)
    engineering = commands.add_parser("engineering-hsaco")
    for name in [
        "ferric",
        "fe2o3",
        "output_root",
        "cargo_fe2o3",
        "extractor",
        "extractor_backend",
        "worker",
        "cargo",
        "rustc",
        "host_linker",
        "host_lld",
        "host_lld_proxy",
        "cargo_vendor",
    ]:
        engineering.add_argument(name, type=Path)
    engineering.set_defaults(entrypoint=run_engineering)
    vendor = commands.add_parser("prepare-engineering-vendor")
    vendor.add_argument("ferric", type=Path)
    vendor.add_argument("fe2o3", type=Path)
    vendor.add_argument("cargo_vendor", type=Path)
    vendor.set_defaults(entrypoint=prepare_engineering_vendor)
    return result


def main() -> None:
    arguments = parser().parse_args()
    arguments.entrypoint(arguments)


if __name__ == "__main__":
    main()
