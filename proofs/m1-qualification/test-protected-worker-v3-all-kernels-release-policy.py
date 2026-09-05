#!/usr/bin/env python3
"""Exercise exact aggregate production configuration and release policy."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
from types import ModuleType, SimpleNamespace
from typing import NoReturn


REVISION = "5c4759711775210d3094fd71ebc579fdd00c4db8"
WORKER_BYTES = b"synthetic Worker V3 linker\n"
WORKER_ID = "fe2o3-worker-v1-sha256-" + hashlib.sha256(b"worker build").hexdigest()


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def git(repository: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repository), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
        env={"LANG": "C", "LC_ALL": "C", "PATH": "/usr/bin:/bin", "TZ": "UTC"},
    )
    if result.returncode != 0:
        fail(f"fixture Git failed: {result.stdout!r}")
    return result.stdout.decode("ascii").strip()


def commit(repository: Path) -> None:
    git(repository, "add", ".")
    result = subprocess.run(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=Ferric Test",
            "-c",
            "user.email=ferric-test@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
        env={"LANG": "C", "LC_ALL": "C", "PATH": "/usr/bin:/bin", "TZ": "UTC"},
    )
    if result.returncode != 0:
        fail(f"cannot commit fixture: {result.stdout!r}")


def load(producer: Path) -> ModuleType:
    specification = importlib.util.spec_from_file_location("_aggregate_release", producer)
    if specification is None or specification.loader is None:
        fail("cannot load aggregate release producer")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def invoke(producer: Path, *arguments: Path | str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [sys.executable, "-I", "-B", str(producer), *map(str, arguments)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
    )


def require_rejection(result: subprocess.CompletedProcess[bytes], description: str) -> None:
    if result.returncode == 0 or b"FAIL:" not in result.stdout:
        fail(f"producer accepted {description}: {result.stdout!r}")


def independent_identity(raw: bytes, worker: dict[str, object]) -> str:
    digest = hashlib.sha256()
    values = [
        b"fe2o3-build-config-transitive-v2",
        b"production-v2",
        raw,
        bytes.fromhex(str(worker["sha256"])),
        int(worker["byte_len"]).to_bytes(8, "little"),
        str(worker["worker_build_identity"]).encode("ascii"),
        str(worker["llvm_build_identity"]).encode("ascii"),
        (0).to_bytes(8, "little"),
    ]
    for value in values:
        digest.update(len(value).to_bytes(8, "little"))
        digest.update(value)
    return digest.hexdigest()


def make_ferric_fixture(root: Path) -> Path:
    ferric = root / "ferric"
    device = ferric / "device/qwen3-all-kernels-v1"
    (device / "src").mkdir(parents=True)
    git(ferric, "init", "-q")
    (device / "Cargo.toml").write_text(
        """[package]
name = "ferric-qwen3-all-kernels-device-v1"
version = "0.1.0"
edition = "2024"

[dependencies]
fe2o3-device = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "5c4759711775210d3094fd71ebc579fdd00c4db8", version = "=0.1.0" }

[target.'cfg(not(target_arch = "amdgpu"))'.dependencies]
fe2o3-host = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "5c4759711775210d3094fd71ebc579fdd00c4db8", version = "=0.1.0" }
""",
        encoding="ascii",
    )
    source = (
        "git+https://github.com/harsh-nod/fe2o3.git?rev="
        f"{REVISION}#{REVISION}"
    )
    (device / "Cargo.lock").write_text(
        f'''version = 4

[[package]]
name = "fe2o3-device"
version = "0.1.0"
source = "{source}"

[[package]]
name = "fe2o3-host"
version = "0.1.0"
source = "{source}"

[[package]]
name = "pliron"
version = "0.17.0"
source = "git+https://github.com/harsh-nod/pliron.git?rev=5bdf861bf03e7f20242b25717fb653336d02e487#5bdf861bf03e7f20242b25717fb653336d02e487"
''',
        encoding="ascii",
    )
    (device / "src/lib.rs").write_text("#![no_std]\n", encoding="ascii")
    commit(ferric)
    return ferric


def main() -> None:
    repository = Path(__file__).resolve().parents[2]
    producer = (
        repository
        / "proofs/m1-qualification/produce-protected-worker-v3-all-kernels-release.py"
    )
    source = producer.read_text(encoding="ascii")
    for required in [
        'FE2O3_REVISION = "5c4759711775210d3094fd71ebc579fdd00c4db8"',
        '"FE2O3_PRODUCTION_BUILD_CONFIG_V2": str(arguments.config)',
        '"FE2O3_TARGET": "gfx942"',
        '"authority",\n        "release",\n        "build",\n        "--locked"',
        "validate_protected_infrastructure(CLIENT_PROFILE, SUPERVISOR_SOCKET)",
        'CLIENT_PROFILE = Path("/etc/fe2o3/compiler-execution/client-profile-v1")',
        'SUPERVISOR_SOCKET = Path("/run/fe2o3/compiler-execution-supervisor.sock")',
        "produce-protected-worker-v3-all-kernels-build.py",
        "produce-protected-worker-v3-all-kernels-publication-selection.py",
        '"engineering",\n        "hsaco"',
        '"gfx942:xnack-"',
        '"--cargo-git-source"',
        'f"https://github.com/harsh-nod/fe2o3.git@{FE2O3_REVISION}"',
        'f"https://github.com/harsh-nod/pliron.git@{PLIRON_REVISION}"',
    ]:
        if required not in source:
            fail(f"aggregate release producer lost required policy: {required}")

    module = load(producer)
    with tempfile.TemporaryDirectory(prefix="ferric-aggregate-release-") as raw_root:
        root = Path(raw_root)
        os.chmod(root, 0o700)
        ferric = make_ferric_fixture(root)
        worker_root = root / "worker"
        worker_root.mkdir(mode=0o700)
        worker = worker_root / "fe2o3-llvm-link-worker"
        worker.write_bytes(WORKER_BYTES)
        worker.chmod(0o700)
        (worker_root / "fe2o3-llvm-build-id.txt").write_text("7.2.4\n", encoding="ascii")
        (worker_root / "fe2o3-worker-build-id.txt").write_text(
            WORKER_ID + "\n", encoding="ascii"
        )
        output_root = root / "output"
        output_root.mkdir(mode=0o700)
        config = output_root / "production-config.json"
        result = invoke(producer, "prepare-config", ferric, worker, config)
        if result.returncode != 0:
            fail(f"producer rejected the canonical fixture: {result.stdout!r}")
        raw = config.read_bytes()
        value = json.loads(raw)
        if (
            raw
            != json.dumps(
                value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
            ).encode("ascii")
            or stat.S_IMODE(config.stat().st_mode) != 0o600
            or value.get("format") != "fe2o3-production-build-config-v2"
            or value.get("candidate_output_max_bytes") != 4_194_304
            or value.get("limits")
            != {
                "stderr_bytes": 65_536,
                "stdout_bytes": 8_388_608,
                "timeout_ms": 120_000,
            }
            or value.get("link_options")
            != [
                {"name": "code-object-version", "value": "6"},
                {"name": "opt-level", "value": "2"},
                {"name": "strip-debug", "value": "true"},
                {"name": "verify-each", "value": "true"},
            ]
            or value.get("observation") != {"kind": "source-isa-summary-v1"}
            or value.get("providers") != []
            or value.get("units")
            != [
                {
                    "crate_name": "ferric_qwen3_all_kernels_device_v1",
                    "source": "src/lib.rs",
                    "working_directory": str(
                        ferric / "device/qwen3-all-kernels-v1"
                    ),
                }
            ]
            or value.get("worker", {}).get("sha256")
            != hashlib.sha256(WORKER_BYTES).hexdigest()
        ):
            fail("published aggregate configuration drifted")
        identity = independent_identity(raw, value["worker"])
        if not re.search(rb" identity=" + identity.encode("ascii") + rb"(?: |\n)", result.stdout):
            fail("producer did not report the exact transitive V2 identity")

        require_rejection(
            invoke(producer, "prepare-config", ferric, worker, config),
            "replacement of an existing configuration",
        )
        tampered = output_root / "tampered.json"
        changed = dict(value)
        changed["candidate_output_max_bytes"] = 4_194_305
        tampered.write_bytes(
            json.dumps(
                changed, ensure_ascii=True, separators=(",", ":"), sort_keys=True
            ).encode("ascii")
        )
        try:
            module.exact_config(tampered, ferric)
        except SystemExit:
            pass
        else:
            fail("exact config validation accepted a changed recipe")
        malformed = output_root / "malformed.json"
        changed = dict(value)
        changed["worker"] = {**value["worker"], "path": 7}
        malformed.write_bytes(
            json.dumps(
                changed, ensure_ascii=True, separators=(",", ":"), sort_keys=True
            ).encode("ascii")
        )
        try:
            module.exact_config(malformed, ferric)
        except SystemExit:
            pass
        else:
            fail("exact config validation accepted a malformed worker path")

        (ferric / "dirty").write_text("dirty\n", encoding="ascii")
        require_rejection(
            invoke(
                producer,
                "prepare-config",
                ferric,
                worker,
                output_root / "dirty.json",
            ),
            "a dirty Ferric source repository",
        )
        (ferric / "dirty").unlink()

        original_run = module.subprocess.run
        module.CLIENT_PROFILE = root / "missing-profile"
        module.SUPERVISOR_SOCKET = root / "missing-socket"

        def forbidden_run(*_args: object, **_kwargs: object) -> object:
            fail("protected infrastructure rejection occurred after subprocess spawn")

        module.subprocess.run = forbidden_run
        try:
            module.run_build(SimpleNamespace())
        except SystemExit:
            pass
        else:
            fail("build accepted missing protected worker/verifier infrastructure")
        finally:
            module.subprocess.run = original_run

        user_profile = root / "user-profile"
        user_profile.write_bytes(b"not root owned")
        not_socket = root / "not-socket"
        not_socket.write_bytes(b"not a socket")
        try:
            module.validate_protected_infrastructure(user_profile, not_socket)
        except SystemExit:
            pass
        else:
            fail("infrastructure validation accepted an unprivileged profile")

        engineering_parent = root / "engineering"
        engineering_parent.mkdir(mode=0o700)
        engineering_root = engineering_parent / "fe2o3-engineering-v1"
        engineering_content = engineering_root / ("a" * 64)
        engineering_content.mkdir(parents=True, mode=0o700)
        hsaco = b"synthetic aggregate engineering HSACO"
        (engineering_content / "observation.hsaco").write_bytes(hsaco)
        manifest = {
            "artifact": "observation.hsaco",
            "authority": "none",
            "code_object_version": 6,
            "crate_name": "ferric_qwen3_all_kernels_device_v1",
            "grants": {"launch": False, "load": False, "publication": False},
            "hsaco": {
                "identity": {
                    "byte_len": len(hsaco),
                    "sha256": hashlib.sha256(hsaco).hexdigest(),
                },
                "kernel_names": list(module.KERNELS),
            },
            "namespace": "fe2o3-engineering-v1",
            "options": {
                "maximum_output_bytes": 4_194_304,
                "optimization": "O2",
                "strip_debug": True,
                "timeout_seconds": 120,
                "verify_each": True,
            },
            "providers": [],
            "schema": "EngineeringHsacoObservationV1",
            "target": "gfx942:xnack-",
        }
        manifest_path = engineering_content / "observation.json"
        manifest_path.write_bytes(json.dumps(manifest).encode("ascii"))
        content, manifest_sha, hsaco_sha = module.exact_engineering_observation(
            engineering_root
        )
        if (
            content != engineering_content
            or manifest_sha != hashlib.sha256(manifest_path.read_bytes()).hexdigest()
            or hsaco_sha != hashlib.sha256(hsaco).hexdigest()
        ):
            fail("exact engineering observation identities drifted")
        manifest["hsaco"]["kernel_names"][0], manifest["hsaco"]["kernel_names"][1] = (
            manifest["hsaco"]["kernel_names"][1],
            manifest["hsaco"]["kernel_names"][0],
        )
        manifest_path.write_bytes(json.dumps(manifest).encode("ascii"))
        try:
            module.exact_engineering_observation(engineering_root)
        except SystemExit:
            pass
        else:
            fail("engineering observation admitted swapped kernel order")
        manifest["hsaco"]["kernel_names"] = list(module.KERNELS)
        manifest["grants"]["load"] = True
        manifest_path.write_bytes(json.dumps(manifest).encode("ascii"))
        try:
            module.exact_engineering_observation(engineering_root)
        except SystemExit:
            pass
        else:
            fail("engineering observation admitted load authority")

    print("PASS: aggregate protected Worker V3 release policy")


if __name__ == "__main__":
    main()
