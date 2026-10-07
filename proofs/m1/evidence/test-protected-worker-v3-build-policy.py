#!/usr/bin/env python3
"""Exercise the exact protected Worker V3 build record and hostile mutations."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


Mutation = Callable[[dict[str, Any]], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def canonical(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def invoke(validator: Path, record: Path) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [sys.executable, "-I", str(validator), str(record)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=15,
    )


def set_path(path: tuple[str | int, ...], value: Any) -> Mutation:
    def mutate(record: dict[str, Any]) -> None:
        current: Any = record
        for component in path[:-1]:
            current = current[component]
        current[path[-1]] = value

    return mutate


def main() -> None:
    repo = Path(__file__).resolve().parents[3]
    validator = repo / "proofs/m1/evidence/validate-protected-worker-v3-build.py"
    record_path = repo / "proofs/m1/evidence/PROTECTED_WORKER_V3_SWIGLU_BUILD.json"
    canonical_record = json.loads(record_path.read_bytes())
    if record_path.read_bytes() != canonical(canonical_record):
        fail("checked protected-build record is not canonical JSON")
    positive = invoke(validator, record_path)
    if positive.returncode != 0 or not positive.stdout.startswith(
        b"PASS: protected Worker V3 build record sha256="
    ):
        fail(f"validator rejected exact protected-build record: {positive.stdout!r}")

    mutations: list[tuple[str, Mutation]] = [
        ("format", set_path(("format",), "FERRIC-M1-PROTECTED-WORKER-V3-BUILD-V2")),
        ("authority", set_path(("authority",), "gpu-dispatch-authority")),
        ("nonclaim", set_path(("nonclaim",), "Compilation complete.")),
        ("target", set_path(("target",), "gfx950:xnack-")),
        ("source-commit", set_path(("source", "commit"), "0" * 40)),
        ("source-tree", set_path(("source", "tree"), "1" * 40)),
        ("provider", set_path(("source", "device_provider_commit"), "2" * 40)),
        ("source-file", set_path(("source", "device_files", 2, "sha256"), "3" * 64)),
        ("compiler-commit", set_path(("compiler", "commit"), "4" * 40)),
        ("cargo-image", set_path(("compiler", "cargo_fe2o3_sha256"), "5" * 64)),
        ("backend-image", set_path(("compiler", "codegen_backend_sha256"), "6" * 64)),
        ("closure", set_path(("compiler", "closure", "identity_sha256"), "7" * 64)),
        ("closure-cargo", set_path(("compiler", "closure", "cargo_executable_sha256"), "f" * 64)),
        ("closure-backend", set_path(("compiler", "closure", "codegen_backend_sha256"), "8" * 64)),
        ("artifact-sha", set_path(("artifact", "sha256"), "9" * 64)),
        ("artifact-size", set_path(("artifact", "size_bytes"), 14_191)),
        ("config", set_path(("production_recipe", "sha256"), "a" * 64)),
        ("recipe-limit", set_path(("production_recipe", "limits", "timeout_ms"), 1)),
        ("worker", set_path(("production_recipe", "worker", "sha256"), "b" * 64)),
        ("custody", set_path(("custody_records", 0, "sha256"), "0" * 64)),
        ("inspection-authority", set_path(("inspection", "authority"), "load-authority")),
        ("inspection-kernel", set_path(("inspection", "kernel", "name"), "other")),
        ("inspection-kernarg", set_path(("inspection", "kernel", "kernarg_size_bytes"), 48)),
        ("claim", set_path(("publication", "claim", "sha256"), "c" * 64)),
        ("envelope", set_path(("publication", "load_readiness", "envelope_sha256"), "d" * 64)),
        ("readiness-receipt", set_path(("publication", "load_readiness", "receipt_identity_sha256"), "1" * 64)),
        ("publication-identity", set_path(("publication", "publication_identity_sha256"), "2" * 64)),
        ("binding-source", set_path(("publication", "worker_v3_binding", "source_evidence_sha256"), "3" * 64)),
        ("finalized-cross-link", set_path(("publication", "worker_v3_binding", "finalized_output_sha256"), "e" * 64)),
        ("status-promotion", set_path(("established_claims",), ["gpu-dispatch"])),
        ("excluded-claim-removal", set_path(("excluded_claims",), [])),
    ]
    with tempfile.TemporaryDirectory(prefix="ferric-protected-build-policy-") as raw:
        root = Path(raw)
        for name, mutation in mutations:
            candidate = copy.deepcopy(canonical_record)
            mutation(candidate)
            path = root / f"{name}.json"
            path.write_bytes(canonical(candidate))
            result = invoke(validator, path)
            if result.returncode == 0 or b"FAIL:" not in result.stdout:
                fail(f"validator accepted hostile {name}: {result.stdout!r}")

        noncanonical = root / "noncanonical.json"
        noncanonical.write_bytes(
            json.dumps(canonical_record, ensure_ascii=True, sort_keys=True).encode("ascii")
        )
        if invoke(validator, noncanonical).returncode == 0:
            fail("validator accepted noncanonical JSON")
        symlink = root / "symlink.json"
        symlink.symlink_to(record_path)
        if invoke(validator, symlink).returncode == 0:
            fail("validator accepted a symlinked record")

    print(
        "PASS: exact protected Worker V3 build record validated and "
        f"{len(mutations) + 2} hostile inputs were rejected"
    )


if __name__ == "__main__":
    main()
