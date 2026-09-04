#!/usr/bin/env python3
"""Produce one aggregate, observational Worker V3 protected-build record."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import selectors
import signal
import stat
import subprocess
import sys
import time
import tomllib
from types import ModuleType
from typing import Any, BinaryIO, NoReturn


FORMAT = "FERRIC-M1-PROTECTED-WORKER-V3-ALL-KERNELS-BUILD-V2"
AUTHORITY = "identity-and-structure-observation-only"
NONCLAIM = (
    "This record preserves byte identities, bounded namespace custody, descriptive HSACO "
    "inspection, and output from a source-prebound typed source-pin adapter only. It records "
    "the configured Source/ISA observation request but does not retain or authenticate emitted "
    "Source/ISA telemetry. Its shallow checksum parsing is not typed decoding of every durable "
    "record and does not reauthenticate a current durable publication lease. It does not "
    "establish protected compiler origin, compilation or finalization authenticity, durable "
    "publication, verifier authority, GPU load or dispatch, numerical correctness, performance, "
    "Qwen model execution, or M1 qualification."
)
ESTABLISHED = [
    "aggregate-byte-identity-observation",
    "aggregate-bounded-namespace-custody-observation",
    "aggregate-descriptive-hsaco-inspection",
    "aggregate-source-prebound-typed-adapter-output-observation",
]
EXCLUDED = [
    "compiler-origin-authentication",
    "current-publication-custody",
    "durable-publication-authentication",
    "gpu-dispatch",
    "gpu-load",
    "m1-qualification",
    "numerical-correctness",
    "performance",
    "protected-compilation-authentication",
    "qwen-execution",
    "source-isa-observation-authentication",
    "verifier-authority",
    "worker-v3-finalization-authentication",
]
TARGET = "gfx942:xnack-"
DEVICE_CRATE = "ferric-qwen3-all-kernels-device-v1"
DEVICE_CRATE_RUST = "ferric_qwen3_all_kernels_device_v1"
DEVICE_RELATIVE = PurePosixPath("device/qwen3-all-kernels-v1")
SOURCE_PIN_FORMAT = "ferric.m1-all-kernels-worker-v3-source-pin.v1"
SOURCE_PIN_ADAPTER = "ferric-qwen3-all-kernels-worker-v3-source-pin-v1"
SOURCE_PIN_BINDING_FORMAT = "FERRIC-M1-ALL-KERNELS-SOURCE-PIN-ADAPTER-BINDING-V1"
SOURCE_PIN_BINDING_RELATIVE = PurePosixPath(
    "adapters/qwen3-all-kernels-worker-v3-source-pin-v1/"
    "SOURCE_PIN_ADAPTER_BINDING_V1.json"
)
SOURCE_PIN_BINDING_AUTHORITY = "binary-identity-prebinding-only"
SOURCE_PIN_BINDING_NONCLAIM = (
    "This source-controlled record pre-binds one executable identity to the exact adapter "
    "source closure. It is not a reproducible-build proof, compiler-origin attestation, "
    "semantic-correctness proof, or runtime authority."
)
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
DEVICE_SOURCE_RELATIVES = (
    PurePosixPath("device/qwen3-all-kernels-v1/Cargo.lock"),
    PurePosixPath("device/qwen3-all-kernels-v1/Cargo.toml"),
    PurePosixPath("device/qwen3-all-kernels-v1/build.rs"),
    PurePosixPath("device/qwen3-all-kernels-v1/rust-toolchain.toml"),
    PurePosixPath("device/qwen3-all-kernels-v1/src/gemm.rs"),
    PurePosixPath("device/qwen3-all-kernels-v1/src/lib.rs"),
    PurePosixPath("device/qwen3-all-kernels-v1/src/logits.rs"),
    PurePosixPath("device/qwen3-all-kernels-v1/src/paged_decode.rs"),
    PurePosixPath("device/qwen3-all-kernels-v1/src/prefill.rs"),
    PurePosixPath("device/qwen3-all-kernels-v1/src/rmsnorm.rs"),
    PurePosixPath("device/qwen3-all-kernels-v1/src/rope_kv.rs"),
    PurePosixPath("device/qwen3-all-kernels-v1/src/swiglu.rs"),
)
ADAPTER_SOURCE_RELATIVES = (
    PurePosixPath("adapters/qwen3-all-kernels-worker-v3-source-pin-v1/Cargo.lock"),
    PurePosixPath("adapters/qwen3-all-kernels-worker-v3-source-pin-v1/Cargo.toml"),
    PurePosixPath("adapters/qwen3-all-kernels-worker-v3-source-pin-v1/src/lib.rs"),
    PurePosixPath("adapters/qwen3-all-kernels-worker-v3-source-pin-v1/src/main.rs"),
)
MAX_ENVELOPE_BYTES = 264 * 1024 * 1024
MAX_ADAPTER_OUTPUT_BYTES = 64 * 1024
MAX_INSPECTION_OUTPUT_BYTES = 128 * 1024
MAX_ARTIFACT_NAMESPACE_DIRECTORIES = 4
MAX_ARTIFACT_NAMESPACE_FILES = 16
MAX_ARTIFACT_NAMESPACE_ENTRIES = 24
MAX_ARTIFACT_NAMESPACE_TOTAL_BYTES = MAX_ENVELOPE_BYTES + 64 * 1024 * 1024
SUBPROCESS_TIMEOUT_SECONDS = 30.0
INSPECTION_KERNEL = re.compile(
    r"kernel\[([0-9]+)\]: name=([a-zA-Z0-9_]+) symbol=([a-zA-Z0-9_.]+) "
    r"kernarg-bytes=([0-9]+) kernarg-align=([0-9]+) wave=([0-9]+) "
    r"lds-bytes=([0-9]+) private-bytes=([0-9]+) explicit-args=([0-9]+) "
    r"hidden-args=([0-9]+) sgprs=([0-9]+) vgprs=([0-9]+)\Z"
)


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_custody_module() -> ModuleType:
    path = Path(__file__).with_name("produce-protected-worker-v3-build.py")
    specification = importlib.util.spec_from_file_location(
        "_ferric_protected_worker_v3_custody", path
    )
    if specification is None or specification.loader is None:
        fail("cannot load the historical Worker V3 custody implementation")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


CUSTODY = load_custody_module()

Held = tuple[str, BinaryIO, os.stat_result, bytes]
DirectoryHeld = tuple[str, int, os.stat_result]


def git_blob_oid(repository: Path, relative: PurePosixPath, data: bytes) -> str:
    expected = CUSTODY.git(
        repository,
        ["rev-parse", f"HEAD:{relative}"],
        f"resolve committed Git blob {relative}",
    )
    result = subprocess.run(
        ["git", "-C", str(repository), "hash-object", "--stdin"],
        input=data,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
        env={"PATH": os.environ.get("PATH", "")},
    )
    try:
        actual = result.stdout.decode("ascii").strip()
    except UnicodeDecodeError as error:
        fail(f"cannot hash committed Git blob {relative}: {error}")
    if result.returncode != 0 or actual != expected or len(expected) != 40:
        fail(f"held source bytes do not equal the exact HEAD Git blob: {relative}")
    return expected


def hold_git_files(
    source_repo: Path,
    relatives: tuple[PurePosixPath, ...],
    description: str,
) -> tuple[list[dict[str, Any]], list[Held]]:
    records = []
    held = []
    for relative in relatives:
        item = CUSTODY.hold(
            source_repo.joinpath(*relative.parts),
            f"{description} {relative}",
            2 * 1024 * 1024,
        )
        blob = git_blob_oid(source_repo, relative, item[3])
        records.append(
            {
                "git_blob": blob,
                "path": str(relative),
                "sha256": CUSTODY.sha256(item[3]),
                "size_bytes": len(item[3]),
            }
        )
        held.append(item)
    return records, held


def revalidate_git_files(
    source_repo: Path, records: list[dict[str, Any]], held: list[Held]
) -> None:
    if len(records) != len(held):
        fail("held Git source roster changed during evidence production")
    for record, item in zip(records, held, strict=True):
        CUSTODY.revalidate(item)
        path = source_repo.joinpath(*PurePosixPath(record["path"]).parts)
        try:
            named = path.lstat()
        except OSError as error:
            fail(f"cannot revalidate named Git source {record['path']}: {error}")
        if CUSTODY.identity(named) != CUSTODY.identity(item[2]):
            fail(f"named Git source changed custody: {record['path']}")
        if git_blob_oid(source_repo, PurePosixPath(record["path"]), item[3]) != record["git_blob"]:
            fail(f"Git source blob changed during evidence production: {record['path']}")


def source_closure_sha256(records: list[dict[str, Any]]) -> str:
    return CUSTODY.sha256(CUSTODY.compact_bytes(records))


def terminate_process_group(process: subprocess.Popen[bytes]) -> None:
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            fail("cannot reap terminated subprocess group")


def run_bounded_held(
    executable: Held,
    arguments: list[str],
    inherited_fds: tuple[int, ...],
    maximum_output_bytes: int,
    description: str,
) -> tuple[int, bytes]:
    executable_fd = executable[1].fileno()
    command = [f"/proc/self/fd/{executable_fd}", *arguments]
    try:
        process = subprocess.Popen(
            command,
            executable=command[0],
            pass_fds=(executable_fd, *inherited_fds),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            start_new_session=True,
            env={"LANG": "C", "LC_ALL": "C", "TZ": "UTC"},
        )
    except OSError as error:
        fail(f"cannot execute held {description}: {error}")
    if process.stdout is None:
        terminate_process_group(process)
        fail(f"cannot capture held {description} output")
    output = bytearray()
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    deadline = time.monotonic() + SUBPROCESS_TIMEOUT_SECONDS
    try:
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                terminate_process_group(process)
                fail(f"held {description} timed out")
            events = selector.select(remaining)
            if not events:
                terminate_process_group(process)
                fail(f"held {description} timed out")
            for key, _ in events:
                chunk = os.read(
                    key.fileobj.fileno(),
                    min(64 * 1024, maximum_output_bytes + 1 - len(output)),
                )
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                output.extend(chunk)
                if len(output) > maximum_output_bytes:
                    terminate_process_group(process)
                    fail(f"held {description} output exceeded {maximum_output_bytes} bytes")
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            terminate_process_group(process)
            fail(f"held {description} timed out")
        try:
            returncode = process.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            terminate_process_group(process)
            fail(f"held {description} timed out")
    finally:
        selector.close()
        process.stdout.close()
    return returncode, bytes(output)


def exact_config(config: Any, source_repo: Path) -> dict[str, Any]:
    raw = config[3]
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"production config is not JSON: {error}")
    if raw != CUSTODY.compact_bytes(value) or not isinstance(value, dict):
        fail("production config is not canonical compact ASCII JSON")
    if set(value) != {
        "candidate_output_max_bytes",
        "format",
        "limits",
        "link_options",
        "observation",
        "providers",
        "units",
        "worker",
    }:
        fail("production config fields drifted")
    observation = value["observation"]
    if (
        not isinstance(observation, dict)
        or set(observation) != {"kind"}
        or observation.get("kind") != "source-isa-summary-v1"
    ):
        fail("production config must select the exact source-isa-summary-v1 observation")
    if (
        value["format"] != "fe2o3-production-build-config-v2"
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
        fail("production config does not select the aggregate Worker V3 recipe")
    unit = value["units"][0]
    if (
        not isinstance(unit, dict)
        or set(unit) != {"crate_name", "source", "working_directory"}
        or unit.get("crate_name") != DEVICE_CRATE_RUST
        or unit.get("source") != "src/lib.rs"
        or not isinstance(unit.get("working_directory"), str)
    ):
        fail("production config aggregate compilation unit drifted")
    working = PurePosixPath(unit["working_directory"])
    if not working.is_absolute():
        fail("production config working directory is not absolute")
    if tuple(working.parts[-len(DEVICE_RELATIVE.parts) :]) != DEVICE_RELATIVE.parts:
        fail("production config working directory does not name the aggregate device crate")
    if Path(unit["working_directory"]) != source_repo.joinpath(*DEVICE_RELATIVE.parts):
        fail("production config working directory is outside the Ferric source checkout")
    if not (source_repo.joinpath(*DEVICE_RELATIVE.parts) / "src/lib.rs").is_file():
        fail("Ferric source checkout lacks the configured aggregate device unit")
    worker = value["worker"]
    if (
        not isinstance(worker, dict)
        or set(worker)
        != {
            "byte_len",
            "llvm_build_identity",
            "path",
            "sha256",
            "worker_build_identity",
        }
        or not isinstance(worker["byte_len"], int)
        or isinstance(worker["byte_len"], bool)
        or worker["byte_len"] <= 0
        or worker["llvm_build_identity"] != "7.2.4"
        or not isinstance(worker["sha256"], str)
        or CUSTODY.SHA256.fullmatch(worker["sha256"]) is None
        or not isinstance(worker["worker_build_identity"], str)
        or not worker["worker_build_identity"].startswith("fe2o3-worker-v1-sha256-")
    ):
        fail("production config worker pin drifted")
    return {
        "candidate_output_max_bytes": value["candidate_output_max_bytes"],
        "format": value["format"],
        "limits": value["limits"],
        "link_options": value["link_options"],
        "observation": {"kind": observation["kind"]},
        "sha256": CUSTODY.sha256(raw),
        "unit": {
            "crate_name": unit["crate_name"],
            "source": unit["source"],
            "working_directory_relative": str(DEVICE_RELATIVE),
        },
        "worker": {key: worker[key] for key in sorted(worker) if key != "path"},
    }


def device_source(
    source_repo: Path,
) -> tuple[str, list[dict[str, Any]], list[Held]]:
    records, held = hold_git_files(
        source_repo, DEVICE_SOURCE_RELATIVES, "aggregate device source"
    )
    manifest_index = DEVICE_SOURCE_RELATIVES.index(
        PurePosixPath("device/qwen3-all-kernels-v1/Cargo.toml")
    )
    manifest = held[manifest_index]
    try:
        parsed = tomllib.loads(manifest[3].decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        fail(f"aggregate device Cargo manifest is invalid: {error}")
    package = parsed.get("package", {})
    dependencies = parsed.get("dependencies", {})
    target = parsed.get("target", {}).get('cfg(not(target_arch = "amdgpu"))', {})
    host = target.get("dependencies", {}).get("fe2o3-host", {})
    device = dependencies.get("fe2o3-device", {})
    provider = device.get("rev")
    if (
        package.get("name") != DEVICE_CRATE
        or not isinstance(provider, str)
        or len(provider) != 40
        or host.get("rev") != provider
        or device.get("git") != "https://github.com/harsh-nod/fe2o3.git"
        or host.get("git") != "https://github.com/harsh-nod/fe2o3.git"
    ):
        fail("aggregate device crate does not carry one exact fe2o3 provider revision")
    return provider, records, held


def adapter_binding(
    source_repo: Path, adapter: Held
) -> tuple[dict[str, Any], Held, list[Held]]:
    source_records, source_files = hold_git_files(
        source_repo, ADAPTER_SOURCE_RELATIVES, "aggregate source-pin adapter source"
    )
    binding = CUSTODY.hold(
        source_repo.joinpath(*SOURCE_PIN_BINDING_RELATIVE.parts),
        "source-controlled aggregate source-pin adapter binding",
        128 * 1024,
    )
    binding_blob = git_blob_oid(source_repo, SOURCE_PIN_BINDING_RELATIVE, binding[3])
    try:
        value = json.loads(binding[3])
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"aggregate source-pin adapter binding is not JSON: {error}")
    if not isinstance(value, dict) or binding[3] != CUSTODY.canonical_bytes(value):
        fail("aggregate source-pin adapter binding is not canonical ASCII JSON")
    if set(value) != {
        "authority",
        "binary",
        "format",
        "nonclaim",
        "protocol",
        "source_closure_sha256",
        "source_files",
    }:
        fail("aggregate source-pin adapter binding fields drifted")
    binary = value["binary"]
    if (
        value["authority"] != SOURCE_PIN_BINDING_AUTHORITY
        or value["format"] != SOURCE_PIN_BINDING_FORMAT
        or value["nonclaim"] != SOURCE_PIN_BINDING_NONCLAIM
        or value["protocol"] != SOURCE_PIN_FORMAT
        or value["source_files"] != source_records
        or value["source_closure_sha256"] != source_closure_sha256(source_records)
        or not isinstance(binary, dict)
        or set(binary) != {"name", "sha256", "size_bytes"}
        or binary["name"] != SOURCE_PIN_ADAPTER
        or binary["sha256"] != CUSTODY.sha256(adapter[3])
        or binary["size_bytes"] != len(adapter[3])
    ):
        fail("aggregate source-pin adapter is not pre-bound by the exact Ferric source")
    return {
        "binding_git_blob": binding_blob,
        "binding_sha256": CUSTODY.sha256(binding[3]),
        "binary_sha256": binary["sha256"],
        "binary_size_bytes": binary["size_bytes"],
        "name": SOURCE_PIN_ADAPTER,
        "protocol": SOURCE_PIN_FORMAT,
        "source_closure_sha256": value["source_closure_sha256"],
        "source_files": source_records,
    }, binding, source_files


def single_component(name: str, description: str) -> str:
    if not name or name in {".", ".."} or "/" in name or "\0" in name:
        fail(f"{description} is not a single path component")
    return name


def directory_flags() -> int:
    required = ("O_CLOEXEC", "O_DIRECTORY", "O_NOFOLLOW")
    if any(not hasattr(os, name) for name in required):
        fail("artifact custody requires O_CLOEXEC/O_DIRECTORY/O_NOFOLLOW")
    return os.O_RDONLY | os.O_CLOEXEC | os.O_DIRECTORY | os.O_NOFOLLOW


def directory_binding(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
    )


def open_directory_at(parent_fd: int, name: str, description: str) -> DirectoryHeld:
    component = single_component(name, description)
    try:
        before = os.stat(component, dir_fd=parent_fd, follow_symlinks=False)
        descriptor = os.open(component, directory_flags(), dir_fd=parent_fd)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(before.st_mode)
        or not stat.S_ISDIR(before.st_mode)
        or directory_binding(before) != directory_binding(opened)
    ):
        os.close(descriptor)
        fail(f"{description} is not a stable nonsymlink directory")
    return component, descriptor, opened


def enumerate_directory(directory_fd: int, description: str) -> tuple[str, ...]:
    names = []
    try:
        with os.scandir(directory_fd) as entries:
            for entry in entries:
                names.append(single_component(entry.name, description))
                if len(names) > MAX_ARTIFACT_NAMESPACE_ENTRIES:
                    fail("artifact namespace exceeds its admitted entry bound")
    except OSError as error:
        fail(f"cannot enumerate {description}: {error}")
    return tuple(sorted(names))


def hold_file_at(
    directory_fd: int, name: str, maximum: int, description: str
) -> Held:
    component = single_component(name, description)
    flags = os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW
    try:
        before = os.stat(component, dir_fd=directory_fd, follow_symlinks=False)
        descriptor = os.open(component, flags, dir_fd=directory_fd)
        source = os.fdopen(descriptor, "rb")
        opened = os.fstat(source.fileno())
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(before.st_mode)
        or not stat.S_ISREG(before.st_mode)
        or not stat.S_ISREG(opened.st_mode)
        or opened.st_nlink != 1
        or CUSTODY.identity(before) != CUSTODY.identity(opened)
        or opened.st_size < 0
        or opened.st_size > maximum
    ):
        source.close()
        fail(f"{description} is outside the admitted regular-file policy")
    data = source.read(maximum + 1)
    named = os.stat(component, dir_fd=directory_fd, follow_symlinks=False)
    after = os.fstat(source.fileno())
    if (
        len(data) != opened.st_size
        or CUSTODY.identity(opened) != CUSTODY.identity(after)
        or CUSTODY.identity(after) != CUSTODY.identity(named)
    ):
        source.close()
        fail(f"{description} changed while held")
    return description, source, after, data


def artifact_files(
    root: Path,
) -> tuple[dict[str, Held], list[dict[str, Any]], dict[str, Any]]:
    if not root.is_absolute():
        fail("artifact root must be absolute")
    components = root.parts[1:]
    if not components:
        fail("artifact root must not be the filesystem root")
    filesystem_fd = os.open("/", directory_flags())
    chain = []
    parent_fd = filesystem_fd
    for ordinal, component in enumerate(components, 1):
        held_directory = open_directory_at(
            parent_fd, component, f"artifact root component {ordinal}"
        )
        chain.append((parent_fd, *held_directory))
        parent_fd = held_directory[1]
    root_metadata = os.fstat(parent_fd)
    if root_metadata.st_uid != os.geteuid() or stat.S_IMODE(root_metadata.st_mode) != 0o700:
        fail("artifact root must be an owner-private 0700 directory")
    pending = [("", parent_fd)]
    directories = []
    path_files: list[tuple[str, int, Held]] = []
    total_entries = 0
    total_bytes = 0
    while pending:
        relative_directory, directory_fd = pending.pop()
        names = enumerate_directory(directory_fd, "artifact namespace directory")
        directories.append((relative_directory, directory_fd, names))
        if len(directories) > MAX_ARTIFACT_NAMESPACE_DIRECTORIES:
            fail("artifact namespace exceeds its admitted directory bound")
        total_entries += len(names)
        if total_entries > MAX_ARTIFACT_NAMESPACE_ENTRIES:
            fail("artifact namespace exceeds its admitted entry bound")
        for name in names:
            metadata = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            relative_name = f"{relative_directory}/{name}" if relative_directory else name
            if stat.S_ISLNK(metadata.st_mode):
                fail("artifact namespace contains a symlink")
            if stat.S_ISDIR(metadata.st_mode):
                component = open_directory_at(
                    directory_fd, name, f"artifact namespace directory {relative_name}"
                )
                chain.append((directory_fd, *component))
                pending.append((relative_name, component[1]))
            elif stat.S_ISREG(metadata.st_mode):
                maximum = (
                    MAX_ENVELOPE_BYTES
                    if relative_name.endswith(".envelope")
                    else CUSTODY.MAX_FILE_BYTES
                )
                item = hold_file_at(
                    directory_fd,
                    name,
                    maximum,
                    f"artifact-root file {relative_name}",
                )
                path_files.append((relative_name, directory_fd, item))
                total_bytes += len(item[3])
                if (
                    len(path_files) > MAX_ARTIFACT_NAMESPACE_FILES
                    or total_bytes > MAX_ARTIFACT_NAMESPACE_TOTAL_BYTES
                ):
                    fail("artifact namespace exceeds its admitted file or byte bound")
            else:
                fail("artifact namespace contains a non-regular entry")
    relative = sorted(relative for relative, _, _ in path_files)
    fixed = {
        ".codegen-generation-v1",
        ".fe2o3-artifacts.lock",
        ".fe2o3-attempts-v1",
        ".fe2o3-owned-v1",
    }
    classified: dict[str, str] = {}
    for name in relative:
        if name in fixed:
            classified[name] = name
        elif CUSTODY.ARTIFACT_NAME.fullmatch(name):
            classified["artifact"] = name
        elif CUSTODY.PUBLICATION_NAME.fullmatch(name):
            classified["publication"] = name
        elif (match := CUSTODY.READINESS_NAME.fullmatch(name)) is not None:
            classified[match.group(2)] = name
        elif CUSTODY.HANDOFF_PATH.fullmatch(name):
            classified["consumed"] = name
        else:
            fail(f"artifact root contains an unrecognized file: {name}")
    expected = fixed | {
        "artifact",
        "publication",
        "claim",
        "envelope",
        "receipt",
        "consumed",
    }
    if set(classified) != expected or len(classified) != len(relative):
        fail("artifact root does not contain one exact Worker V3 publication roster")
    consumed_path = PurePosixPath(classified["consumed"])
    expected_directories = {
        "",
        consumed_path.parent.parent.as_posix(),
        consumed_path.parent.as_posix(),
    }
    if {relative_directory for relative_directory, _, _ in directories} != expected_directories:
        fail("artifact root does not contain the exact Worker V3 directory roster")
    held: dict[str, Held] = {}
    parent_fds: dict[str, int] = {}
    roster = []
    by_name = {name: (directory_fd, item) for name, directory_fd, item in path_files}
    for kind, name in sorted(classified.items(), key=lambda item: item[1]):
        directory_fd, item = by_name[name]
        held[kind] = item
        parent_fds[kind] = directory_fd
        roster.append(
            {
                "kind": kind,
                "path": name,
                "sha256": CUSTODY.sha256(item[3]),
                "size_bytes": len(item[3]),
            }
        )
    if held[".fe2o3-artifacts.lock"][3] != b"":
        fail("artifact output lock snapshot is not empty")
    for kind, magic in [
        (".codegen-generation-v1", CUSTODY.GENERATION_MAGIC),
        (".fe2o3-attempts-v1", CUSTODY.ATTEMPTS_MAGIC),
        (".fe2o3-owned-v1", CUSTODY.OWNED_MAGIC),
        ("consumed", CUSTODY.CONSUMED_MAGIC),
        ("publication", CUSTODY.PUBLICATION_MAGIC),
    ]:
        if not held[kind][3].startswith(magic):
            fail(f"artifact-root {kind} record magic drifted")
    return held, roster, {
        "chain": chain,
        "directories": directories,
        "filesystem_fd": filesystem_fd,
        "parent_fds": parent_fds,
        "root_fd": parent_fd,
        "root_metadata": root_metadata,
    }


def revalidate_artifact_namespace(namespace: dict[str, Any], files: dict[str, Held]) -> None:
    try:
        filesystem = os.fstat(namespace["filesystem_fd"])
    except OSError as error:
        fail(f"cannot revalidate filesystem root for artifact custody: {error}")
    if not stat.S_ISDIR(filesystem.st_mode):
        fail("filesystem root changed during artifact custody")
    for parent_fd, name, descriptor, opened in namespace["chain"]:
        try:
            named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
            current = os.fstat(descriptor)
        except OSError as error:
            fail(f"cannot revalidate artifact namespace directory {name}: {error}")
        if (
            stat.S_ISLNK(named.st_mode)
            or directory_binding(named) != directory_binding(opened)
            or directory_binding(current) != directory_binding(opened)
        ):
            fail(f"artifact namespace directory changed custody: {name}")
    root_metadata = os.fstat(namespace["root_fd"])
    if (
        directory_binding(root_metadata) != directory_binding(namespace["root_metadata"])
        or root_metadata.st_uid != os.geteuid()
        or stat.S_IMODE(root_metadata.st_mode) != 0o700
    ):
        fail("artifact root private custody changed during evidence production")
    for relative, directory_fd, expected_names in namespace["directories"]:
        if enumerate_directory(directory_fd, f"artifact namespace directory {relative}") != expected_names:
            fail(f"artifact namespace membership changed during evidence production: {relative}")
    for kind, item in files.items():
        parent_fd = namespace["parent_fds"][kind]
        name = PurePosixPath(item[0].removeprefix("artifact-root file ")).name
        try:
            before = os.fstat(item[1].fileno())
            item[1].seek(0)
            data = item[1].read(len(item[3]) + 1)
            after = os.fstat(item[1].fileno())
            named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        except OSError as error:
            fail(f"cannot revalidate artifact-root file {name}: {error}")
        if (
            stat.S_ISLNK(named.st_mode)
            or named.st_nlink != 1
            or data != item[3]
            or CUSTODY.identity(item[2]) != CUSTODY.identity(before)
            or CUSTODY.identity(before) != CUSTODY.identity(after)
            or CUSTODY.identity(after) != CUSTODY.identity(named)
        ):
            fail(f"artifact-root file changed custody: {name}")


def inspect_hsaco(cargo: Any, artifact: Any) -> dict[str, Any]:
    artifact_fd = artifact[1].fileno()
    returncode, raw_output = run_bounded_held(
        cargo,
        [
        "inspect",
        "--format",
        "hsaco",
        f"/proc/self/fd/{artifact_fd}",
        ],
        (artifact_fd,),
        MAX_INSPECTION_OUTPUT_BYTES,
        "cargo-fe2o3 inspector",
    )
    try:
        output = raw_output.decode("ascii")
    except UnicodeDecodeError as error:
        fail(f"cargo-fe2o3 inspection is not ASCII: {error}")
    lines = output.splitlines()
    expected_header = [
        "format: hsaco-v6",
        "authority: descriptive-only",
        "metadata-version: 1.2",
        f"target: {TARGET}",
        "printf-metadata: false",
        f"kernels: {len(KERNELS)}",
    ]
    if returncode != 0 or lines[:6] != expected_header or len(lines) != 6 + len(KERNELS):
        fail(f"cargo-fe2o3 rejected aggregate finalized HSACO: {output.strip()}")
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
    kernels = []
    for expected_index, line in enumerate(lines[6:]):
        match = INSPECTION_KERNEL.fullmatch(line)
        if match is None or int(match.group(1)) != expected_index:
            fail("cargo-fe2o3 aggregate inspection kernel metadata drifted")
        name, symbol = match.group(2), match.group(3)
        if symbol != f"{name}.kd":
            fail("cargo-fe2o3 aggregate inspection descriptor pairing drifted")
        values = [int(value) for value in match.groups()[3:]]
        kernels.append({"name": name, "symbol": symbol, **dict(zip(keys, values, strict=True))})
    names = [kernel["name"] for kernel in kernels]
    if len(set(names)) != len(KERNELS) or set(names) != set(KERNELS):
        fail("cargo-fe2o3 inspection did not report the exact aggregate kernel set")
    return {
        "authority": "descriptive-only",
        "format": "hsaco-v6",
        "kernel_count": len(kernels),
        "kernels": kernels,
        "metadata_version": "1.2",
        "ordering_claim": "none",
        "target": TARGET,
        "transcript_sha256": CUSTODY.sha256(raw_output),
    }


def invoke_source_pin_adapter(adapter: Any, envelope: Any) -> tuple[dict[str, Any], dict[str, Any]]:
    if stat.S_IMODE(adapter[2].st_mode) & 0o111 == 0:
        fail("aggregate source-pin adapter is not executable")
    envelope_fd = envelope[1].fileno()
    returncode, raw_output = run_bounded_held(
        adapter,
        [f"/proc/self/fd/{envelope_fd}"],
        (envelope_fd,),
        MAX_ADAPTER_OUTPUT_BYTES,
        "aggregate source-pin adapter",
    )
    if returncode != 0:
        fail(f"aggregate source-pin adapter rejected the held envelope: {raw_output[:1000]!r}")
    try:
        projection = json.loads(raw_output)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"aggregate source-pin adapter output is not JSON: {error}")
    if not isinstance(projection, dict) or raw_output != CUSTODY.canonical_bytes(projection):
        fail("aggregate source-pin adapter output is not canonical ASCII JSON")
    if set(projection) != {
        "authority",
        "authenticates_compiler_origin",
        "code_object_version",
        "format",
        "grants_launch_authority",
        "grants_load_authority",
        "grants_publication_authority",
        "grants_verifier_authority",
        "policy_kernel_symbols",
        "program_count",
        "source_pin",
        "target",
    }:
        fail("aggregate source-pin projection fields drifted")
    if (
        projection["authority"] != "identity-observation-only"
        or projection["authenticates_compiler_origin"] is not False
        or projection["code_object_version"] != 6
        or projection["format"] != SOURCE_PIN_FORMAT
        or projection["grants_launch_authority"] is not False
        or projection["grants_load_authority"] is not False
        or projection["grants_publication_authority"] is not False
        or projection["grants_verifier_authority"] is not False
        or projection["policy_kernel_symbols"] != list(KERNELS)
        or projection["program_count"] != len(KERNELS)
        or projection["target"] != TARGET
    ):
        fail("aggregate source-pin projection policy drifted")
    source_pin = projection["source_pin"]
    expected_pin_fields = {
        "compiler_handoff_length",
        "compiler_handoff_sha256",
        "compiler_module_length",
        "compiler_module_sha256",
        "symbol_manifest_length",
        "symbol_manifest_sha256",
    }
    if not isinstance(source_pin, dict) or set(source_pin) != expected_pin_fields:
        fail("aggregate source-pin coordinate fields drifted")
    for field in sorted(expected_pin_fields):
        value = source_pin[field]
        if field.endswith("_length"):
            if not isinstance(value, int) or isinstance(value, bool) or value <= 0 or value > 2**64 - 1:
                fail(f"aggregate source-pin {field} is not a positive u64")
        elif not isinstance(value, str) or CUSTODY.SHA256.fullmatch(value) is None or len(set(value)) == 1:
            fail(f"aggregate source-pin {field} is not a nondegenerate SHA-256")
    return projection, {
        "envelope_sha256": CUSTODY.sha256(envelope[3]),
        "output_sha256": CUSTODY.sha256(raw_output),
    }


def revalidate_named(item: Held, path: Path, description: str) -> None:
    CUSTODY.revalidate(item)
    try:
        named = path.lstat()
    except OSError as error:
        fail(f"cannot revalidate named {description}: {error}")
    if CUSTODY.identity(named) != CUSTODY.identity(item[2]):
        fail(f"named {description} changed custody during evidence production")


def main() -> None:
    if len(sys.argv) != 10:
        fail(
            "usage: produce-protected-worker-v3-all-kernels-build.py "
            "FERRIC_SOURCE_REPO FE2O3_COMPILER_REPO PRODUCTION_CONFIG "
            "ARTIFACT_ROOT CARGO_FE2O3 RUSTC_WRAPPER CODEGEN_BACKEND "
            "SOURCE_PIN_ADAPTER OUTPUT"
        )
    (
        source_repo,
        compiler_repo,
        config_path,
        artifact_root,
        cargo_path,
        wrapper_path,
        backend_path,
        adapter_path,
        output_path,
    ) = map(Path, sys.argv[1:])
    source = CUSTODY.source_identity(source_repo, "Ferric aggregate build source")
    compiler = CUSTODY.source_identity(compiler_repo, "fe2o3 compiler source")
    config = CUSTODY.hold(config_path, "production config", 64_000)
    recipe = exact_config(config, source_repo)
    provider_commit, source_files, held_device_sources = device_source(source_repo)
    CUSTODY.git(
        compiler_repo,
        ["cat-file", "-e", f"{provider_commit}^{{commit}}"],
        "locate aggregate device provider commit",
    )
    provider_tree = CUSTODY.git(
        compiler_repo,
        ["rev-parse", f"{provider_commit}^{{tree}}"],
        "resolve aggregate device provider tree",
    )
    cargo = CUSTODY.hold(cargo_path, "cargo-fe2o3 inspector", 64 * 1024 * 1024)
    wrapper = CUSTODY.hold(wrapper_path, "fe2o3 rustc wrapper", 16 * 1024 * 1024)
    backend = CUSTODY.hold(backend_path, "fe2o3 codegen backend", 256 * 1024 * 1024)
    adapter = CUSTODY.hold(adapter_path, "aggregate source-pin adapter", 64 * 1024 * 1024)
    adapter_prebinding, binding_file, held_adapter_sources = adapter_binding(
        source_repo, adapter
    )
    files, roster, artifact_namespace = artifact_files(artifact_root)
    artifact_name = files["artifact"][0].removeprefix("artifact-root file ")
    artifact_match = CUSTODY.ARTIFACT_NAME.fullmatch(artifact_name)
    artifact_sha = CUSTODY.sha256(files["artifact"][3])
    if artifact_match is None or artifact_match.group(1) != artifact_sha:
        fail("finalized aggregate HSACO filename does not equal its SHA-256")
    claim = CUSTODY.parse_claim(files["claim"][3], artifact_sha, len(files["artifact"][3]))
    readiness = CUSTODY.parse_readiness(
        files["receipt"][3], files["claim"][3], files["envelope"][3], claim
    )
    readiness_names = {
        kind: CUSTODY.READINESS_NAME.fullmatch(
            files[kind][0].removeprefix("artifact-root file ")
        )
        for kind in ("claim", "envelope", "receipt")
    }
    if any(
        match is None or match.group(1) != claim["namespace_key"]
        for match in readiness_names.values()
    ):
        fail("aggregate load-readiness filenames do not equal the backend namespace key")
    closure = claim["compiler_closure"]
    if (
        closure["cargo_fe2o3_binding_wrapper_sha256"] != CUSTODY.sha256(cargo[3])
        or closure["codegen_backend_sha256"] != CUSTODY.sha256(backend[3])
    ):
        fail("compiler images do not match the authenticated compiler closure")
    inspection = inspect_hsaco(cargo, files["artifact"])
    projection, adapter_observation = invoke_source_pin_adapter(adapter, files["envelope"])
    if adapter_observation["envelope_sha256"] != readiness["envelope_sha256"]:
        fail("aggregate source-pin adapter input differs from the receipt-bound envelope")
    record = {
        "artifact": {
            "path": next(item["path"] for item in roster if item["kind"] == "artifact"),
            "sha256": artifact_sha,
            "size_bytes": len(files["artifact"][3]),
        },
        "authority": AUTHORITY,
        "observed_compiler_inputs": {
            **compiler,
            "cargo_fe2o3_sha256": CUSTODY.sha256(cargo[3]),
            "claim_embedded_closure": closure,
            "codegen_backend_sha256": CUSTODY.sha256(backend[3]),
            "rustc_wrapper_sha256": CUSTODY.sha256(wrapper[3]),
        },
        "custody_records": roster,
        "established_claims": ESTABLISHED,
        "excluded_claims": EXCLUDED,
        "format": FORMAT,
        "inspection": inspection,
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "observed_production_recipe": recipe,
        "observed_worker_v3_records": {
            "checksummed_claim": {
                "backend_receipt_sha256": claim["backend_receipt_sha256"],
                "sha256": CUSTODY.sha256(files["claim"][3]),
                "size_bytes": len(files["claim"][3]),
            },
            "declared_finalization_identity_sha256": claim["plan"]["finalization"],
            "declared_finalized_output_identity_sha256": claim["plan"]["finalized_output"],
            "declared_publication_identity_sha256": claim["plan"]["publication"],
            "receipt_checksum_observation": readiness,
            "shallow_worker_v3_binding_observation": claim["worker_v3_binding"],
            "typed_durable_record_decoding": False,
            "typed_current_publication_reacquisition": False,
        },
        "declared_release_entrypoint": [
            "cargo-fe2o3",
            "authority",
            "release",
            "build",
            "--locked",
        ],
        "source": {
            **source,
            "device_files": source_files,
            "device_provider_commit": provider_commit,
            "device_provider_tree": provider_tree,
        },
        "source_pin_observation": {
            "adapter_execution": adapter_observation,
            "adapter_prebinding": adapter_prebinding,
            "projection": projection,
        },
        "target": TARGET,
    }
    revalidate_named(config, config_path, "production config")
    revalidate_named(cargo, cargo_path, "cargo-fe2o3 inspector")
    revalidate_named(wrapper, wrapper_path, "fe2o3 rustc wrapper")
    revalidate_named(backend, backend_path, "fe2o3 codegen backend")
    revalidate_named(adapter, adapter_path, "aggregate source-pin adapter")
    revalidate_git_files(source_repo, source_files, held_device_sources)
    adapter_source_records = adapter_prebinding["source_files"]
    revalidate_git_files(source_repo, adapter_source_records, held_adapter_sources)
    CUSTODY.revalidate(binding_file)
    binding_path = source_repo.joinpath(*SOURCE_PIN_BINDING_RELATIVE.parts)
    if (
        CUSTODY.identity(binding_path.lstat()) != CUSTODY.identity(binding_file[2])
        or git_blob_oid(source_repo, SOURCE_PIN_BINDING_RELATIVE, binding_file[3])
        != adapter_prebinding["binding_git_blob"]
    ):
        fail("source-controlled aggregate adapter binding changed custody")
    revalidate_artifact_namespace(artifact_namespace, files)
    if (
        CUSTODY.source_identity(source_repo, "Ferric aggregate build source") != source
        or CUSTODY.source_identity(compiler_repo, "fe2o3 compiler source") != compiler
    ):
        fail("source repository identity changed during aggregate evidence production")
    encoded = CUSTODY.canonical_bytes(record)
    CUSTODY.publish(output_path, encoded)
    print(
        "PASS: published aggregate protected Worker V3 build record "
        f"sha256={CUSTODY.sha256(encoded)}"
    )


if __name__ == "__main__":
    main()
