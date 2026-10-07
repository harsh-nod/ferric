#!/usr/bin/env python3
"""Produce one canonical, scope-limited Worker V3 protected-build record."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import struct
import subprocess
import sys
import tomllib
from typing import Any, BinaryIO, NoReturn


FORMAT = "FERRIC-M1-PROTECTED-WORKER-V3-BUILD-V1"
AUTHORITY = "protected-compilation-finalization-and-inert-publication-only"
NONCLAIM = (
    "This record establishes the observed protected compilation, Worker V3 HSACO "
    "finalization, and inert load-envelope publication only. It does not establish "
    "verifier authority, GPU load or dispatch, numerical correctness, performance, "
    "Qwen model execution, or M1 qualification."
)
ESTABLISHED = [
    "protected-compiler-closure-bound-compilation",
    "worker-v3-hsaco-finalization-and-inspection",
    "inert-load-envelope-durable-publication",
]
EXCLUDED = [
    "gpu-dispatch",
    "gpu-load",
    "m1-qualification",
    "numerical-correctness",
    "performance",
    "qwen-execution",
    "verifier-authority",
]
TARGET = "gfx942:xnack-"
DEVICE_CRATE = "ferric-qwen3-swiglu-device-v1"
DEVICE_CRATE_RUST = "ferric_qwen3_swiglu_device_v1"
DEVICE_RELATIVE = PurePosixPath("device/qwen3-swiglu-v1")
CANONICAL_SOURCE_RELATIVE = PurePosixPath(
    "device/qwen3-all-kernels-v1/src/swiglu.rs"
)
KERNEL = "qwen3_swiglu_bf16_f32_v1"

CLAIM_MAGIC = b"FE2O3-PUBLISHED-HSACO-CLAIM-V3\0"
CLAIM_VERSION = 3
CLAIM_CHECKSUM_DOMAIN = b"fe2o3.published-hsaco-claim.checksum.v3\0"
RECEIPT_MAGIC = b"FE2O3-WORKER-V3-LOAD-READINESS-RECEIPT-V1\0"
RECEIPT_VERSION = 1
RECEIPT_CHECKSUM_DOMAIN = b"fe2o3.worker-v3-load-readiness-receipt.checksum.v1\0"
BACKEND_RECEIPT_DOMAIN = b"fe2o3.worker-v3-load-readiness.backend-receipt.v1\0"
NAMESPACE_DOMAIN = b"fe2o3.worker-v3-load-readiness.namespace-key.v1\0"
COMPILER_CLOSURE_DOMAIN = b"fe2o3-compiler-closure-identity-v2\0"
PUBLICATION_MAGIC = b"FE2O3-DURABLE-LINK-V1\0"
ATTEMPTS_MAGIC = b"FE2O3-ATTEMPTS-V1\0"
CONSUMED_MAGIC = b"FE2O3-COMPILER-MODULE-HANDOFF-V3\0"
GENERATION_MAGIC = b"fe2o3-codegen-generation-v1\0"
OWNED_MAGIC = b"fe2o3-owned-v1\0"

CLAIM_BYTES = 1_219
RECEIPT_BYTES = 356
MAX_FILE_BYTES = 4 * 1024 * 1024
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
ARTIFACT_NAME = re.compile(r"\.fe2o3-link-artifact-v1-([0-9a-f]{64})\.bin\Z")
PUBLICATION_NAME = re.compile(r"\.fe2o3-link-publication-v1-([0-9a-f]{64})\.record\Z")
READINESS_NAME = re.compile(
    r"\.fe2o3-worker-v3-load-readiness-v1-([0-9a-f]{64})"
    r"\.(claim|envelope|receipt)\Z"
)
HANDOFF_PATH = re.compile(
    r"\.fe2o3-compiler-module-handoff-v3-([0-9a-f]{64})/"
    r"attempt-([0-9a-f]{64})/consumed\Z"
)
INSPECTION_KERNEL = re.compile(
    r"kernel\[0\]: name=([a-zA-Z0-9_]+) symbol=([a-zA-Z0-9_.]+) "
    r"kernarg-bytes=([0-9]+) kernarg-align=([0-9]+) wave=([0-9]+) "
    r"lds-bytes=([0-9]+) private-bytes=([0-9]+) explicit-args=([0-9]+) "
    r"hidden-args=([0-9]+) sgprs=([0-9]+) vgprs=([0-9]+)\Z"
)


Held = tuple[str, BinaryIO, os.stat_result, bytes]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def compact_bytes(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def hold(path: Path, description: str, maximum: int = MAX_FILE_BYTES) -> Held:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
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
        or opened.st_size < 0
        or opened.st_size > maximum
    ):
        source.close()
        fail(f"{description} must be a bounded stable regular nonsymlink file")
    data = source.read(maximum + 1)
    if len(data) != opened.st_size:
        source.close()
        fail(f"{description} changed or exceeded its byte bound while read")
    return description, source, opened, data


def revalidate(item: Held) -> None:
    description, source, opened, _ = item
    try:
        current = os.fstat(source.fileno())
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if identity(current) != identity(opened):
        fail(f"{description} changed during evidence production")


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


def source_identity(repository: Path, description: str) -> dict[str, str]:
    if not repository.is_absolute():
        fail(f"{description} repository path must be absolute")
    if git(repository, ["status", "--porcelain=v1", "--untracked-files=all"], description):
        fail(f"{description} repository must be an exact clean worktree")
    commit = git(repository, ["rev-parse", "HEAD"], f"resolve {description} commit")
    tree = git(repository, ["rev-parse", "HEAD^{tree}"], f"resolve {description} tree")
    if len(commit) != 40 or len(tree) != 40:
        fail(f"{description} Git identity is not canonical")
    return {"commit": commit, "tree": tree}


def exact_config(config: Held, source_repo: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    raw = config[3]
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"production config is not JSON: {error}")
    if raw != compact_bytes(value) or not isinstance(value, dict):
        fail("production config is not canonical compact ASCII JSON")
    if set(value) != {
        "candidate_output_max_bytes",
        "format",
        "limits",
        "link_options",
        "providers",
        "units",
        "worker",
    }:
        fail("production config fields drifted")
    if (
        value["format"] != "fe2o3-production-build-config-v1"
        or value["candidate_output_max_bytes"] != 4_194_304
        or value["providers"] != []
        or value["limits"]
        != {"stderr_bytes": 65_536, "stdout_bytes": 8_388_608, "timeout_ms": 120_000}
        or value["link_options"]
        != [
            {"name": "code-object-version", "value": "6"},
            {"name": "opt-level", "value": "2"},
            {"name": "strip-debug", "value": "true"},
            {"name": "verify-each", "value": "true"},
        ]
        or not isinstance(value["units"], list)
        or len(value["units"]) != 1
    ):
        fail("production config does not select the admitted Worker V3 recipe")
    unit = value["units"][0]
    if (
        unit.get("crate_name") != DEVICE_CRATE_RUST
        or unit.get("source") != "src/lib.rs"
        or set(unit) != {"crate_name", "source", "working_directory"}
    ):
        fail("production config compilation unit drifted")
    working = PurePosixPath(unit["working_directory"])
    if not working.is_absolute() or not working.is_relative_to(PurePosixPath("/")):
        fail("production config working directory is not absolute")
    if tuple(working.parts[-len(DEVICE_RELATIVE.parts) :]) != DEVICE_RELATIVE.parts:
        fail("production config working directory does not name the SwiGLU device crate")
    device = source_repo.joinpath(*DEVICE_RELATIVE.parts)
    if not (device / "src/lib.rs").is_file():
        fail("Ferric source checkout lacks the configured device unit")
    worker = value["worker"]
    if (
        set(worker)
        != {
            "byte_len",
            "llvm_build_identity",
            "path",
            "sha256",
            "worker_build_identity",
        }
        or not isinstance(worker["byte_len"], int)
        or worker["byte_len"] <= 0
        or worker["llvm_build_identity"] != "7.2.4"
        or SHA256.fullmatch(worker["sha256"]) is None
        or not worker["worker_build_identity"].startswith("fe2o3-worker-v1-sha256-")
    ):
        fail("production config worker pin drifted")
    projected = {
        "candidate_output_max_bytes": value["candidate_output_max_bytes"],
        "limits": value["limits"],
        "link_options": value["link_options"],
        "sha256": sha256(raw),
        "unit": {
            "crate_name": unit["crate_name"],
            "source": unit["source"],
            "working_directory_relative": str(DEVICE_RELATIVE),
        },
        "worker": {key: worker[key] for key in sorted(worker) if key != "path"},
    }
    return value, projected


def device_source(source_repo: Path) -> tuple[str, list[dict[str, Any]]]:
    device = source_repo.joinpath(*DEVICE_RELATIVE.parts)
    manifest = hold(device / "Cargo.toml", "device Cargo manifest", 64_000)
    lock = hold(device / "Cargo.lock", "device Cargo lock", 1_000_000)
    wrapper = hold(device / "src/lib.rs", "device Rust wrapper", 64_000)
    canonical = hold(
        source_repo.joinpath(*CANONICAL_SOURCE_RELATIVE.parts),
        "canonical device Rust source",
        256_000,
    )
    try:
        parsed = tomllib.loads(manifest[3].decode("utf-8"))
        wrapper_text = wrapper[3].decode("utf-8")
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        fail(f"device Cargo manifest or wrapper is invalid: {error}")
    package = parsed.get("package", {})
    dependencies = parsed.get("dependencies", {})
    target = parsed.get("target", {}).get("cfg(not(target_arch = \"amdgpu\"))", {})
    host = target.get("dependencies", {}).get("fe2o3-host", {})
    device_dependency = dependencies.get("fe2o3-device", {})
    provider = device_dependency.get("rev")
    if (
        package.get("name") != DEVICE_CRATE
        or not isinstance(provider, str)
        or len(provider) != 40
        or host.get("rev") != provider
        or device_dependency.get("git") != "https://github.com/harsh-nod/fe2o3.git"
        or host.get("git") != "https://github.com/harsh-nod/fe2o3.git"
    ):
        fail("device crate does not carry one exact fe2o3 provider revision")
    if '#[path = "../../qwen3-all-kernels-v1/src/swiglu.rs"]' not in wrapper_text:
        fail("device compatibility wrapper does not select the canonical SwiGLU source")
    records = []
    held_sources = [
        (DEVICE_RELATIVE / "Cargo.lock", lock),
        (DEVICE_RELATIVE / "Cargo.toml", manifest),
        (DEVICE_RELATIVE / "src/lib.rs", wrapper),
        (CANONICAL_SOURCE_RELATIVE, canonical),
    ]
    for relative, held in sorted(held_sources, key=lambda item: str(item[0])):
        records.append(
            {"path": str(relative), "sha256": sha256(held[3]), "size_bytes": len(held[3])}
        )
        revalidate(held)
        held[1].close()
    return provider, records


def take(data: bytes, offset: int, count: int, description: str) -> tuple[bytes, int]:
    end = offset + count
    if end > len(data):
        fail(f"{description} is truncated")
    return data[offset:end], end


def parse_claim(data: bytes, artifact_sha: str, artifact_size: int) -> dict[str, Any]:
    if len(data) != CLAIM_BYTES:
        fail("published claim has a noncanonical length")
    body, checksum = data[:-32], data[-32:]
    if hashlib.sha256(CLAIM_CHECKSUM_DOMAIN + body).digest() != checksum:
        fail("published claim checksum mismatch")
    offset = 0
    magic, offset = take(body, offset, len(CLAIM_MAGIC), "published claim")
    version, offset = take(body, offset, 2, "published claim")
    if magic != CLAIM_MAGIC or struct.unpack("<H", version)[0] != CLAIM_VERSION:
        fail("published claim magic or version drifted")
    attempt, offset = take(body, offset, 56, "published claim attempt")
    scope, offset = take(body, offset, 96, "published claim scope")
    plan_names = (
        "request",
        "worker",
        "response",
        "linked_output",
        "finalization",
        "finalized_output",
        "publication",
        "upstream_evidence",
    )
    plan: dict[str, str] = {}
    for name in plan_names:
        item, offset = take(body, offset, 32, f"published claim {name}")
        plan[name] = item.hex()
    receipt_start = offset
    receipt_names = (
        "attempt",
        "producer",
        "scope",
        "plan_commitment",
        "upstream_evidence",
        "finalized_output",
        "publication",
    )
    receipt: dict[str, str] = {}
    for name in receipt_names:
        item, offset = take(body, offset, 32, f"backend receipt {name}")
        receipt[name] = item.hex()
    pin_names = (
        "cargo_executable",
        "cargo_binding_trampoline",
        "cargo_fe2o3_binding_wrapper",
        "rustc_executable",
        "rustc_runtime_tree",
        "codegen_backend",
    )
    pins: dict[str, str] = {}
    pin_bytes = []
    for name in pin_names:
        item, offset = take(body, offset, 32, f"compiler closure {name}")
        pins[f"{name}_sha256"] = item.hex()
        pin_bytes.append(item)
    protocol_raw, offset = take(body, offset, 2, "compiler closure protocol")
    protocol = struct.unpack("<H", protocol_raw)[0]
    closure_identity, offset = take(body, offset, 32, "compiler closure identity")
    expected_closure = hashlib.sha256(
        COMPILER_CLOSURE_DOMAIN + protocol_raw + b"".join(pin_bytes)
    ).digest()
    if protocol != 1 or closure_identity != expected_closure:
        fail("compiler closure identity is not canonical")
    binding_names = (
        "publication_intent",
        "finalization",
        "source_evidence",
        "compiler_handoff",
        "raw_inspection",
        "raw_output",
    )
    binding: dict[str, Any] = {}
    for name in binding_names:
        item, offset = take(body, offset, 32, f"Worker V3 binding {name}")
        binding[f"{name}_sha256"] = item.hex()
    raw_length, offset = take(body, offset, 8, "Worker V3 raw length")
    finalized_sha, offset = take(body, offset, 32, "Worker V3 finalized digest")
    finalized_length, offset = take(body, offset, 8, "Worker V3 finalized length")
    binding["raw_output_size_bytes"] = struct.unpack("<Q", raw_length)[0]
    binding["finalized_output_sha256"] = finalized_sha.hex()
    binding["finalized_output_size_bytes"] = struct.unpack("<Q", finalized_length)[0]
    receipt_end = offset
    _, offset = take(body, offset, 56, "published claim output file identities")
    if offset != len(body):
        fail("published claim contains trailing bytes")
    if (
        binding["finalized_output_sha256"] != artifact_sha
        or binding["finalized_output_size_bytes"] != artifact_size
        or plan["upstream_evidence"] != receipt["upstream_evidence"]
        or plan["finalized_output"] != receipt["finalized_output"]
        or plan["publication"] != receipt["publication"]
    ):
        fail("published claim does not bind the finalized artifact and receipt axes")
    backend_receipt = body[receipt_start:receipt_end]
    backend_identity = hashlib.sha256(BACKEND_RECEIPT_DOMAIN + backend_receipt).hexdigest()
    namespace = hashlib.sha256(NAMESPACE_DOMAIN + bytes.fromhex(backend_identity)).hexdigest()
    return {
        "attempt": attempt.hex(),
        "backend_receipt_sha256": backend_identity,
        "compiler_closure": {
            **pins,
            "identity_sha256": closure_identity.hex(),
            "transition_protocol_version": protocol,
        },
        "namespace_key": namespace,
        "plan": plan,
        "receipt": receipt,
        "scope": scope.hex(),
        "worker_v3_binding": binding,
    }


def parse_readiness(
    data: bytes, claim: bytes, envelope: bytes, parsed_claim: dict[str, Any]
) -> dict[str, Any]:
    if len(data) != RECEIPT_BYTES:
        fail("load-readiness receipt has a noncanonical length")
    body, checksum = data[:-32], data[-32:]
    if hashlib.sha256(RECEIPT_CHECKSUM_DOMAIN + body).digest() != checksum:
        fail("load-readiness receipt checksum mismatch")
    offset = 0
    magic, offset = take(body, offset, len(RECEIPT_MAGIC), "load-readiness receipt")
    version, offset = take(body, offset, 2, "load-readiness receipt")
    attempt, offset = take(body, offset, 56, "load-readiness attempt")
    backend, offset = take(body, offset, 32, "load-readiness backend")
    envelope_sha, offset = take(body, offset, 32, "load-readiness envelope")
    envelope_length, offset = take(body, offset, 8, "load-readiness envelope")
    claim_sha, offset = take(body, offset, 32, "load-readiness claim")
    claim_length, offset = take(body, offset, 8, "load-readiness claim")
    _, offset = take(body, offset, 14 * 8, "load-readiness file custody")
    if (
        magic != RECEIPT_MAGIC
        or struct.unpack("<H", version)[0] != RECEIPT_VERSION
        or offset != len(body)
        or attempt.hex() != parsed_claim["attempt"]
        or backend.hex() != parsed_claim["backend_receipt_sha256"]
        or envelope_sha.hex() != sha256(envelope)
        or struct.unpack("<Q", envelope_length)[0] != len(envelope)
        or claim_sha.hex() != sha256(claim)
        or struct.unpack("<Q", claim_length)[0] != len(claim)
    ):
        fail("load-readiness receipt does not bind the exact claim and envelope")
    return {
        "backend_receipt_sha256": backend.hex(),
        "claim_sha256": claim_sha.hex(),
        "claim_size_bytes": len(claim),
        "envelope_sha256": envelope_sha.hex(),
        "envelope_size_bytes": len(envelope),
        "receipt_identity_sha256": hashlib.sha256(
            b"fe2o3.worker-v3-load-readiness-receipt.identity.v1\0" + data
        ).hexdigest(),
    }


def artifact_files(root: Path) -> tuple[dict[str, Held], list[dict[str, Any]]]:
    if not root.is_absolute() or root.is_symlink() or not root.is_dir():
        fail("artifact root must be an absolute nonsymlink directory")
    paths = sorted(path for path in root.rglob("*") if path.is_file() or path.is_symlink())
    relative = [path.relative_to(root).as_posix() for path in paths]
    fixed = {".codegen-generation-v1", ".fe2o3-artifacts.lock", ".fe2o3-attempts-v1", ".fe2o3-owned-v1"}
    classified: dict[str, str] = {}
    readiness: dict[str, str] = {}
    for name in relative:
        if name in fixed:
            classified[name] = name
        elif ARTIFACT_NAME.fullmatch(name):
            classified["artifact"] = name
        elif PUBLICATION_NAME.fullmatch(name):
            classified["publication"] = name
        elif (match := READINESS_NAME.fullmatch(name)) is not None:
            readiness[match.group(2)] = name
            classified[match.group(2)] = name
        elif HANDOFF_PATH.fullmatch(name):
            classified["consumed"] = name
        else:
            fail(f"artifact root contains an unrecognized file: {name}")
    expected = fixed | {"artifact", "publication", "claim", "envelope", "receipt", "consumed"}
    if set(classified) != expected or len(classified) != len(relative):
        fail("artifact root does not contain one exact Worker V3 publication roster")
    held: dict[str, Held] = {}
    roster = []
    for kind, name in sorted(classified.items(), key=lambda item: item[1]):
        maximum = 2 * 1024 * 1024 if kind == "envelope" else MAX_FILE_BYTES
        item = hold(root / name, f"artifact-root file {name}", maximum)
        held[kind] = item
        roster.append({"kind": kind, "path": name, "sha256": sha256(item[3]), "size_bytes": len(item[3])})
    if held[".fe2o3-artifacts.lock"][3] != b"":
        fail("artifact output lock snapshot is not empty")
    for kind, magic in [
        (".codegen-generation-v1", GENERATION_MAGIC),
        (".fe2o3-attempts-v1", ATTEMPTS_MAGIC),
        (".fe2o3-owned-v1", OWNED_MAGIC),
        ("consumed", CONSUMED_MAGIC),
        ("publication", PUBLICATION_MAGIC),
    ]:
        if not held[kind][3].startswith(magic):
            fail(f"artifact-root {kind} record magic drifted")
    return held, roster


def inspect_hsaco(cargo: Held, artifact: Held) -> dict[str, Any]:
    cargo_fd = cargo[1].fileno()
    artifact_fd = artifact[1].fileno()
    command = [
        f"/proc/self/fd/{cargo_fd}",
        "inspect",
        "--format",
        "hsaco",
        f"/proc/self/fd/{artifact_fd}",
    ]
    try:
        result = subprocess.run(
            command,
            executable=command[0],
            pass_fds=(cargo_fd, artifact_fd),
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=30,
            env={"LANG": "C", "LC_ALL": "C", "TZ": "UTC"},
        )
    except OSError as error:
        fail(f"cannot execute held cargo-fe2o3 inspector: {error}")
    try:
        output = result.stdout.decode("ascii")
    except UnicodeDecodeError as error:
        fail(f"cargo-fe2o3 inspection is not ASCII: {error}")
    lines = output.splitlines()
    if result.returncode != 0 or len(lines) != 7:
        fail(f"cargo-fe2o3 rejected finalized HSACO: {output.strip()}")
    expected = [
        "format: hsaco-v6",
        "authority: descriptive-only",
        "metadata-version: 1.2",
        f"target: {TARGET}",
        "printf-metadata: false",
        "kernels: 1",
    ]
    if lines[:6] != expected:
        fail("cargo-fe2o3 inspection metadata drifted")
    match = INSPECTION_KERNEL.fullmatch(lines[6])
    if match is None:
        fail("cargo-fe2o3 inspection kernel metadata drifted")
    values = [int(value) for value in match.groups()[2:]]
    if match.group(1) != KERNEL or match.group(2) != f"{KERNEL}.kd":
        fail("cargo-fe2o3 inspection selected the wrong kernel")
    keys = (
        "kernarg_size_bytes",
        "kernarg_alignment_bytes",
        "wavefront_size",
        "group_segment_size_bytes",
        "private_segment_size_bytes",
        "explicit_argument_count",
        "hidden_argument_count",
        "sgpr_count",
        "vgpr_count",
    )
    return {
        "authority": "descriptive-only",
        "format": "hsaco-v6",
        "kernel": {"name": KERNEL, "symbol": f"{KERNEL}.kd", **dict(zip(keys, values, strict=True))},
        "metadata_version": "1.2",
        "target": TARGET,
        "transcript_sha256": sha256(result.stdout),
    }


def publish(path: Path, data: bytes) -> None:
    if not path.is_absolute() or not path.parent.is_dir() or path.exists() or path.is_symlink():
        fail("output must be a new absolute path under an existing directory")
    parent = path.parent.stat()
    if parent.st_uid != os.geteuid() or stat.S_IMODE(parent.st_mode) != 0o700:
        fail("output parent must be owner-private 0700")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
    try:
        descriptor = os.open(path, flags, 0o600)
        with os.fdopen(descriptor, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
    except OSError as error:
        fail(f"cannot publish protected-build record: {error}")


def main() -> None:
    if len(sys.argv) != 9:
        fail(
            "usage: produce-protected-worker-v3-build.py FERRIC_SOURCE_REPO "
            "FE2O3_COMPILER_REPO PRODUCTION_CONFIG ARTIFACT_ROOT CARGO_FE2O3 "
            "RUSTC_WRAPPER CODEGEN_BACKEND OUTPUT"
        )
    source_repo, compiler_repo, config_path, artifact_root, cargo_path, wrapper_path, backend_path, output_path = map(Path, sys.argv[1:])
    source = source_identity(source_repo, "Ferric build source")
    compiler = source_identity(compiler_repo, "fe2o3 compiler source")
    config = hold(config_path, "production config", 64_000)
    _, recipe = exact_config(config, source_repo)
    provider_commit, source_files = device_source(source_repo)
    git(compiler_repo, ["cat-file", "-e", f"{provider_commit}^{{commit}}"], "locate device provider commit")
    provider_tree = git(compiler_repo, ["rev-parse", f"{provider_commit}^{{tree}}"], "resolve device provider tree")
    cargo = hold(cargo_path, "cargo-fe2o3 inspector", 64 * 1024 * 1024)
    wrapper = hold(wrapper_path, "fe2o3 rustc wrapper", 16 * 1024 * 1024)
    backend = hold(backend_path, "fe2o3 codegen backend", 256 * 1024 * 1024)
    files, roster = artifact_files(artifact_root)
    artifact_match = ARTIFACT_NAME.fullmatch(files["artifact"][0].removeprefix("artifact-root file "))
    artifact_sha = sha256(files["artifact"][3])
    if artifact_match is None or artifact_match.group(1) != artifact_sha:
        fail("finalized HSACO filename does not equal its SHA-256")
    claim = parse_claim(files["claim"][3], artifact_sha, len(files["artifact"][3]))
    readiness = parse_readiness(files["receipt"][3], files["claim"][3], files["envelope"][3], claim)
    readiness_names = {
        kind: READINESS_NAME.fullmatch(files[kind][0].removeprefix("artifact-root file "))
        for kind in ("claim", "envelope", "receipt")
    }
    if any(match is None or match.group(1) != claim["namespace_key"] for match in readiness_names.values()):
        fail("load-readiness filenames do not equal the backend namespace key")
    closure = claim["compiler_closure"]
    if (
        closure["cargo_fe2o3_binding_wrapper_sha256"] != sha256(cargo[3])
        or closure["codegen_backend_sha256"] != sha256(backend[3])
    ):
        fail("compiler images do not match the authenticated compiler closure")
    inspection = inspect_hsaco(cargo, files["artifact"])
    record = {
        "artifact": {
            "path": next(item["path"] for item in roster if item["kind"] == "artifact"),
            "sha256": artifact_sha,
            "size_bytes": len(files["artifact"][3]),
        },
        "authority": AUTHORITY,
        "compiler": {
            **compiler,
            "cargo_fe2o3_sha256": sha256(cargo[3]),
            "closure": closure,
            "codegen_backend_sha256": sha256(backend[3]),
            "rustc_wrapper_sha256": sha256(wrapper[3]),
        },
        "custody_records": roster,
        "established_claims": ESTABLISHED,
        "excluded_claims": EXCLUDED,
        "format": FORMAT,
        "inspection": inspection,
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "production_recipe": recipe,
        "publication": {
            "claim": {
                "backend_receipt_sha256": claim["backend_receipt_sha256"],
                "sha256": sha256(files["claim"][3]),
                "size_bytes": len(files["claim"][3]),
            },
            "finalization_identity_sha256": claim["plan"]["finalization"],
            "finalized_output_identity_sha256": claim["plan"]["finalized_output"],
            "load_readiness": readiness,
            "publication_identity_sha256": claim["plan"]["publication"],
            "worker_v3_binding": claim["worker_v3_binding"],
        },
        "release_entrypoint": ["cargo-fe2o3", "authority", "release", "build", "--locked"],
        "source": {
            **source,
            "device_files": source_files,
            "device_provider_commit": provider_commit,
            "device_provider_tree": provider_tree,
        },
        "target": TARGET,
    }
    for item in [config, cargo, wrapper, backend, *files.values()]:
        revalidate(item)
    if source_identity(source_repo, "Ferric build source") != source or source_identity(compiler_repo, "fe2o3 compiler source") != compiler:
        fail("source repository identity changed during evidence production")
    publish(output_path, canonical_bytes(record))
    print(
        f"PASS: published protected Worker V3 build record sha256={sha256(canonical_bytes(record))}"
    )


if __name__ == "__main__":
    main()
