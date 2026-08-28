#!/usr/bin/env python3
"""Exercise protected Worker V3 build production with an isolated fixture."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import stat
import struct
import subprocess
import sys
import tempfile
from typing import Any, NoReturn


CLAIM_MAGIC = b"FE2O3-PUBLISHED-HSACO-CLAIM-V3\0"
CLAIM_CHECKSUM_DOMAIN = b"fe2o3.published-hsaco-claim.checksum.v3\0"
RECEIPT_MAGIC = b"FE2O3-WORKER-V3-LOAD-READINESS-RECEIPT-V1\0"
RECEIPT_CHECKSUM_DOMAIN = (
    b"fe2o3.worker-v3-load-readiness-receipt.checksum.v1\0"
)
BACKEND_RECEIPT_DOMAIN = b"fe2o3.worker-v3-load-readiness.backend-receipt.v1\0"
NAMESPACE_DOMAIN = b"fe2o3.worker-v3-load-readiness.namespace-key.v1\0"
CLOSURE_DOMAIN = b"fe2o3-compiler-closure-identity-v2\0"
INSPECTION = b"""format: hsaco-v6
authority: descriptive-only
metadata-version: 1.2
target: gfx942:xnack-
printf-metadata: false
kernels: 1
kernel[0]: name=qwen3_swiglu_bf16_f32_v1 symbol=qwen3_swiglu_bf16_f32_v1.kd kernarg-bytes=304 kernarg-align=8 wave=64 lds-bytes=0 private-bytes=0 explicit-args=6 hidden-args=13 sgprs=84 vgprs=11
"""


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def sha256(value: bytes) -> bytes:
    return hashlib.sha256(value).digest()


def canonical_compact(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def git(repository: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repository), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=15,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"git {' '.join(arguments)} failed: {result.stdout.strip()}")
    return result.stdout.strip()


def commit(repository: Path, message: str) -> str:
    git(repository, "add", ".")
    git(
        repository,
        "-c",
        "user.name=Ferric Policy",
        "-c",
        "user.email=policy@invalid.example",
        "commit",
        "-q",
        "-m",
        message,
    )
    return git(repository, "rev-parse", "HEAD")


def exact_claim(artifact: bytes, cargo: bytes, backend: bytes) -> tuple[bytes, bytes, bytes]:
    attempt = b"A" * 56
    scope = b"S" * 96
    axes = [sha256(f"plan-{index}".encode("ascii")) for index in range(8)]
    receipts = [sha256(f"receipt-{index}".encode("ascii")) for index in range(7)]
    receipts[4] = axes[7]
    receipts[5] = axes[5]
    receipts[6] = axes[6]
    pins = [
        sha256(b"cargo"),
        sha256(b"trampoline"),
        sha256(cargo),
        sha256(b"rustc"),
        sha256(b"runtime-tree"),
        sha256(backend),
    ]
    protocol = struct.pack("<H", 1)
    closure = sha256(CLOSURE_DOMAIN + protocol + b"".join(pins))
    bindings = [sha256(f"binding-{index}".encode("ascii")) for index in range(6)]
    backend_receipt = (
        b"".join(receipts)
        + b"".join(pins)
        + protocol
        + closure
        + b"".join(bindings)
        + struct.pack("<Q", len(artifact))
        + sha256(artifact)
        + struct.pack("<Q", len(artifact))
    )
    body = (
        CLAIM_MAGIC
        + struct.pack("<H", 3)
        + attempt
        + scope
        + b"".join(axes)
        + backend_receipt
        + b"F" * 56
    )
    claim = body + sha256(CLAIM_CHECKSUM_DOMAIN + body)
    if len(claim) != 1_219:
        fail(f"synthetic claim length drifted: {len(claim)}")
    backend_identity = sha256(BACKEND_RECEIPT_DOMAIN + backend_receipt)
    namespace = sha256(NAMESPACE_DOMAIN + backend_identity)
    return claim, backend_identity, namespace


def exact_readiness(
    claim: bytes, envelope: bytes, backend_identity: bytes
) -> bytes:
    body = (
        RECEIPT_MAGIC
        + struct.pack("<H", 1)
        + b"A" * 56
        + backend_identity
        + sha256(envelope)
        + struct.pack("<Q", len(envelope))
        + sha256(claim)
        + struct.pack("<Q", len(claim))
        + b"C" * (14 * 8)
    )
    receipt = body + sha256(RECEIPT_CHECKSUM_DOMAIN + body)
    if len(receipt) != 356:
        fail(f"synthetic readiness receipt length drifted: {len(receipt)}")
    return receipt


def invoke(producer: Path, arguments: list[Path]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [sys.executable, "-I", str(producer), *map(str, arguments)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
    )


def require_rejection(
    producer: Path, arguments: list[Path], description: str
) -> None:
    result = invoke(producer, arguments)
    if result.returncode == 0 or b"FAIL:" not in result.stdout:
        fail(f"producer accepted {description}: {result.stdout!r}")


def main() -> None:
    repo = Path(__file__).resolve().parents[2]
    producer = repo / "proofs/m1-qualification/produce-protected-worker-v3-build.py"
    with tempfile.TemporaryDirectory(prefix="ferric-protected-build-producer-") as raw:
        root = Path(raw)
        os.chmod(root, 0o700)
        compiler_repo = root / "compiler"
        compiler_repo.mkdir()
        git(compiler_repo, "init", "-q")
        (compiler_repo / "provider.txt").write_text("provider\n", encoding="ascii")
        provider = commit(compiler_repo, "provider")
        (compiler_repo / "compiler.txt").write_text("compiler\n", encoding="ascii")
        commit(compiler_repo, "compiler")

        source_repo = root / "source"
        device = source_repo / "device/qwen3-swiglu-v1"
        (device / "src").mkdir(parents=True)
        git(source_repo, "init", "-q")
        (device / "Cargo.toml").write_text(
            "\n".join(
                [
                    "[package]",
                    'name = "ferric-qwen3-swiglu-device-v1"',
                    'version = "0.1.0"',
                    'edition = "2021"',
                    "",
                    "[dependencies]",
                    'fe2o3-device = { git = "https://github.com/harsh-nod/fe2o3.git", '
                    f'rev = "{provider}" }}',
                    "",
                    "[target.'cfg(not(target_arch = \"amdgpu\"))'.dependencies]",
                    'fe2o3-host = { git = "https://github.com/harsh-nod/fe2o3.git", '
                    f'rev = "{provider}" }}',
                    "",
                ]
            ),
            encoding="ascii",
        )
        (device / "Cargo.lock").write_text("# synthetic lock\n", encoding="ascii")
        (device / "src/lib.rs").write_text("#![no_std]\n", encoding="ascii")
        commit(source_repo, "source")

        cargo = root / "cargo-fe2o3"
        cargo.write_bytes(b"#!/bin/sh\nprintf '%b' " + repr(INSPECTION.decode("ascii")).encode("ascii") + b"\n")
        cargo.chmod(stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
        wrapper = root / "fe2o3-rustc-wrapper"
        wrapper.write_bytes(b"synthetic wrapper")
        backend = root / "librustc_codegen_fe2o3.so"
        backend.write_bytes(b"synthetic backend")

        artifact = b"synthetic finalized hsaco"
        artifact_digest = sha256(artifact).hex()
        claim, backend_identity, namespace = exact_claim(
            artifact, cargo.read_bytes(), backend.read_bytes()
        )
        envelope = b"synthetic inert load envelope"
        readiness = exact_readiness(claim, envelope, backend_identity)
        artifact_root = root / "artifacts"
        artifact_root.mkdir()
        files = {
            ".codegen-generation-v1": b"fe2o3-codegen-generation-v1\0fixture",
            ".fe2o3-artifacts.lock": b"",
            ".fe2o3-attempts-v1": b"FE2O3-ATTEMPTS-V1\0fixture",
            f".fe2o3-link-artifact-v1-{artifact_digest}.bin": artifact,
            f".fe2o3-link-publication-v1-{'1' * 64}.record": b"FE2O3-DURABLE-LINK-V1\0fixture",
            ".fe2o3-owned-v1": b"fe2o3-owned-v1\0fixture",
            f".fe2o3-worker-v3-load-readiness-v1-{namespace.hex()}.claim": claim,
            f".fe2o3-worker-v3-load-readiness-v1-{namespace.hex()}.envelope": envelope,
            f".fe2o3-worker-v3-load-readiness-v1-{namespace.hex()}.receipt": readiness,
        }
        for name, data in files.items():
            (artifact_root / name).write_bytes(data)
        consumed = (
            artifact_root
            / f".fe2o3-compiler-module-handoff-v3-{'2' * 64}"
            / f"attempt-{'3' * 64}/consumed"
        )
        consumed.parent.mkdir(parents=True)
        consumed.write_bytes(b"FE2O3-COMPILER-MODULE-HANDOFF-V3\0fixture")

        config = root / "production-config.json"
        config.write_bytes(
            canonical_compact(
                {
                    "candidate_output_max_bytes": 4_194_304,
                    "format": "fe2o3-production-build-config-v1",
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
                    "providers": [],
                    "units": [
                        {
                            "crate_name": "ferric_qwen3_swiglu_device_v1",
                            "source": "src/lib.rs",
                            "working_directory": str(device),
                        }
                    ],
                    "worker": {
                        "byte_len": 42,
                        "llvm_build_identity": "7.2.4",
                        "path": str(root / "worker"),
                        "sha256": "4" * 64,
                        "worker_build_identity": "fe2o3-worker-v1-sha256-" + "5" * 64,
                    },
                }
            )
        )
        output = root / "record.json"
        arguments = [
            source_repo,
            compiler_repo,
            config,
            artifact_root,
            cargo,
            wrapper,
            backend,
            output,
        ]
        positive = invoke(producer, arguments)
        if positive.returncode != 0 or not positive.stdout.startswith(
            b"PASS: published protected Worker V3 build record sha256="
        ):
            fail(f"producer rejected canonical fixture: {positive.stdout!r}")
        record = json.loads(output.read_bytes())
        if (
            record.get("authority")
            != "protected-compilation-finalization-and-inert-publication-only"
            or record.get("excluded_claims")
            != [
                "gpu-dispatch",
                "gpu-load",
                "m1-qualification",
                "numerical-correctness",
                "performance",
                "qwen-execution",
                "verifier-authority",
            ]
            or b"/tmp/" in output.read_bytes()
        ):
            fail("producer emitted promoted authority or machine-local paths")

        require_rejection(producer, arguments, "a preexisting output")
        hostile_output = root / "hostile-backend.json"
        backend_original = backend.read_bytes()
        backend.write_bytes(b"substituted backend")
        require_rejection(
            producer, [*arguments[:-1], hostile_output], "a substituted backend"
        )
        backend.write_bytes(backend_original)

        claim_path = artifact_root / next(
            name for name in files if name.endswith(".claim")
        )
        hostile_claim = bytearray(claim_path.read_bytes())
        hostile_claim[-1] ^= 1
        claim_path.write_bytes(hostile_claim)
        require_rejection(
            producer,
            [*arguments[:-1], root / "hostile-claim.json"],
            "a claim checksum mutation",
        )
        claim_path.write_bytes(claim)

        (source_repo / "untracked").write_text("dirty\n", encoding="ascii")
        require_rejection(
            producer,
            [*arguments[:-1], root / "dirty-source.json"],
            "a dirty Ferric source checkout",
        )

    print(
        "PASS: protected Worker V3 build producer accepted canonical custody and "
        "rejected preexisting output, backend substitution, claim mutation, and "
        "dirty source"
    )


if __name__ == "__main__":
    main()
