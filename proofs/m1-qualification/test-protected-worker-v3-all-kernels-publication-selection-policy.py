#!/usr/bin/env python3
"""Exercise aggregate build validation and noncurrent selection publication."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
from types import ModuleType
from typing import Any, Callable, NoReturn


Mutation = Callable[[dict[str, Any]], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_validator(path: Path) -> ModuleType:
    specification = importlib.util.spec_from_file_location("_selection_test_validator", path)
    if specification is None or specification.loader is None:
        fail("cannot load aggregate validator")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def canonical(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def compact(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def digest(label: str) -> str:
    return hashlib.sha256(label.encode("ascii")).hexdigest()


def git(repository: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repository), *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={
            "GIT_AUTHOR_NAME": "Ferric Policy",
            "GIT_AUTHOR_EMAIL": "policy@example.invalid",
            "GIT_COMMITTER_NAME": "Ferric Policy",
            "GIT_COMMITTER_EMAIL": "policy@example.invalid",
            "PATH": os.environ.get("PATH", ""),
        },
        timeout=20,
    )
    return result.stdout.strip()


def commit(repository: Path, message: str) -> str:
    git(repository, "add", "-A")
    git(repository, "commit", "-q", "-m", message)
    return git(repository, "rev-parse", "HEAD")


def source_records(paths: tuple[str, ...], prefix: str) -> list[dict[str, Any]]:
    return [
        {
            "git_blob": hashlib.sha1(f"blob-{prefix}-{path}".encode("ascii")).hexdigest(),
            "path": path,
            "sha256": digest(f"source-{prefix}-{path}"),
            "size_bytes": 100 + index,
        }
        for index, path in enumerate(paths)
    ]


def build_record(
    validator: ModuleType,
    ferric_commit: str,
    ferric_tree: str,
    compiler_commit: str,
    compiler_tree: str,
    provider_commit: str,
    provider_tree: str,
) -> dict[str, Any]:
    artifact_sha = digest("finalized-artifact")
    artifact_size = 23_771
    claim_sha = digest("claim")
    claim_size = 2_048
    envelope_sha = digest("envelope")
    envelope_size = 17_113
    namespace = digest("namespace")
    source_pin = {
        "compiler_handoff_length": 31_337,
        "compiler_handoff_sha256": digest("compiler-handoff"),
        "compiler_module_length": 29_911,
        "compiler_module_sha256": digest("compiler-module"),
        "symbol_manifest_length": 2_117,
        "symbol_manifest_sha256": digest("symbol-manifest"),
    }
    adapter_files = source_records(validator.ADAPTER_FILES, "adapter")
    closure = {
        "cargo_binding_trampoline_sha256": digest("cargo-binding-trampoline"),
        "cargo_executable_sha256": digest("cargo-executable"),
        "cargo_fe2o3_binding_wrapper_sha256": digest("cargo-fe2o3"),
        "codegen_backend_sha256": digest("backend"),
        "identity_sha256": digest("closure"),
        "rustc_executable_sha256": digest("rustc"),
        "rustc_runtime_tree_sha256": digest("rustc-runtime"),
        "transition_protocol_version": 1,
    }
    kernels = []
    for name in validator.KERNELS:
        kernels.append(
            {
                "explicit_argument_count": 6,
                "group_segment_size_bytes": 0,
                "hidden_argument_count": 13,
                "kernarg_alignment_bytes": 8,
                "kernarg_size_bytes": 304,
                "name": name,
                "private_segment_size_bytes": 0,
                "sgpr_count": 84,
                "symbol": f"{name}.kd",
                "vgpr_count": 11,
                "wavefront_size": 64,
            }
        )
    custody = [
        {"kind": ".codegen-generation-v1", "path": ".codegen-generation-v1", "sha256": digest("generation"), "size_bytes": 32},
        {"kind": ".fe2o3-artifacts.lock", "path": ".fe2o3-artifacts.lock", "sha256": hashlib.sha256(b"").hexdigest(), "size_bytes": 0},
        {"kind": ".fe2o3-attempts-v1", "path": ".fe2o3-attempts-v1", "sha256": digest("attempts"), "size_bytes": 42},
        {"kind": "consumed", "path": f".fe2o3-compiler-module-handoff-v3-{digest('handoff-dir')}/attempt-{digest('attempt-dir')}/consumed", "sha256": digest("consumed"), "size_bytes": 128},
        {"kind": "artifact", "path": f".fe2o3-link-artifact-v1-{artifact_sha}.bin", "sha256": artifact_sha, "size_bytes": artifact_size},
        {"kind": "publication", "path": f".fe2o3-link-publication-v1-{digest('publication-name')}.record", "sha256": digest("publication-record"), "size_bytes": 1_024},
        {"kind": "claim", "path": f".fe2o3-worker-v3-load-readiness-v1-{namespace}.claim", "sha256": claim_sha, "size_bytes": claim_size},
        {"kind": "envelope", "path": f".fe2o3-worker-v3-load-readiness-v1-{namespace}.envelope", "sha256": envelope_sha, "size_bytes": envelope_size},
        {"kind": "receipt", "path": f".fe2o3-worker-v3-load-readiness-v1-{namespace}.receipt", "sha256": digest("receipt"), "size_bytes": 512},
        {"kind": ".fe2o3-owned-v1", "path": ".fe2o3-owned-v1", "sha256": digest("owned"), "size_bytes": 24},
    ]
    custody.sort(key=lambda item: item["path"])
    return {
        "artifact": {
            "path": f".fe2o3-link-artifact-v1-{artifact_sha}.bin",
            "sha256": artifact_sha,
            "size_bytes": artifact_size,
        },
        "authority": validator.AUTHORITY,
        "custody_records": custody,
        "declared_release_entrypoint": [
            "cargo-fe2o3", "authority", "release", "build", "--locked"
        ],
        "established_claims": validator.ESTABLISHED,
        "excluded_claims": validator.EXCLUDED,
        "format": validator.FORMAT,
        "inspection": {
            "authority": "descriptive-only",
            "format": "hsaco-v6",
            "kernel_count": 12,
            "kernels": kernels,
            "metadata_version": "1.2",
            "ordering_claim": "none",
            "target": validator.TARGET,
            "transcript_sha256": digest("inspection"),
        },
        "milestone": "M1",
        "nonclaim": validator.NONCLAIM,
        "observed_compiler_inputs": {
            "cargo_fe2o3_sha256": closure["cargo_fe2o3_binding_wrapper_sha256"],
            "claim_embedded_closure": closure,
            "codegen_backend_sha256": closure["codegen_backend_sha256"],
            "commit": compiler_commit,
            "rustc_wrapper_sha256": digest("wrapper"),
            "tree": compiler_tree,
        },
        "observed_production_recipe": {
            "candidate_output_max_bytes": 4_194_304,
            "limits": {"stderr_bytes": 65_536, "stdout_bytes": 8_388_608, "timeout_ms": 120_000},
            "link_options": [
                {"name": "code-object-version", "value": "6"},
                {"name": "opt-level", "value": "2"},
                {"name": "strip-debug", "value": "true"},
                {"name": "verify-each", "value": "true"},
            ],
            "sha256": digest("config"),
            "unit": {
                "crate_name": "ferric_qwen3_all_kernels_device_v1",
                "source": "src/lib.rs",
                "working_directory_relative": "device/qwen3-all-kernels-v1",
            },
            "worker": {
                "byte_len": 42,
                "llvm_build_identity": "7.2.4",
                "sha256": digest("worker"),
                "worker_build_identity": f"fe2o3-worker-v1-sha256-{digest('worker-build')}",
            },
        },
        "observed_worker_v3_records": {
            "checksummed_claim": {
                "backend_receipt_sha256": digest("backend-receipt"),
                "sha256": claim_sha,
                "size_bytes": claim_size,
            },
            "declared_finalization_identity_sha256": digest("finalization"),
            "declared_finalized_output_identity_sha256": digest("finalized-output-identity"),
            "declared_publication_identity_sha256": digest("publication-identity"),
            "receipt_checksum_observation": {
                "backend_receipt_sha256": digest("backend-receipt"),
                "claim_sha256": claim_sha,
                "claim_size_bytes": claim_size,
                "envelope_sha256": envelope_sha,
                "envelope_size_bytes": envelope_size,
                "receipt_identity_sha256": digest("receipt-identity"),
            },
            "shallow_worker_v3_binding_observation": {
                "compiler_handoff_sha256": source_pin["compiler_handoff_sha256"],
                "finalization_sha256": digest("finalization"),
                "finalized_output_sha256": artifact_sha,
                "finalized_output_size_bytes": artifact_size,
                "publication_intent_sha256": digest("publication-intent"),
                "raw_inspection_sha256": digest("raw-inspection"),
                "raw_output_sha256": digest("raw-output"),
                "raw_output_size_bytes": 24_000,
                "source_evidence_sha256": digest("source-evidence"),
            },
            "typed_current_publication_reacquisition": False,
            "typed_durable_record_decoding": False,
        },
        "source": {
            "commit": ferric_commit,
            "device_files": source_records(validator.DEVICE_FILES, "device"),
            "device_provider_commit": provider_commit,
            "device_provider_tree": provider_tree,
            "tree": ferric_tree,
        },
        "source_pin_observation": {
            "adapter_execution": {
                "envelope_sha256": envelope_sha,
                "output_sha256": digest("adapter-output"),
            },
            "adapter_prebinding": {
                "binding_git_blob": hashlib.sha1(b"binding-blob").hexdigest(),
                "binding_sha256": digest("binding"),
                "binary_sha256": digest("adapter-binary"),
                "binary_size_bytes": 8_192,
                "name": "ferric-qwen3-all-kernels-worker-v3-source-pin-v1",
                "protocol": "ferric.m1-all-kernels-worker-v3-source-pin.v1",
                "source_closure_sha256": hashlib.sha256(compact(adapter_files)).hexdigest(),
                "source_files": adapter_files,
            },
            "projection": {
                "authority": "identity-observation-only",
                "authenticates_compiler_origin": False,
                "code_object_version": 6,
                "format": "ferric.m1-all-kernels-worker-v3-source-pin.v1",
                "grants_launch_authority": False,
                "grants_load_authority": False,
                "grants_publication_authority": False,
                "grants_verifier_authority": False,
                "policy_kernel_symbols": list(validator.KERNELS),
                "program_count": 12,
                "source_pin": source_pin,
                "target": validator.TARGET,
            },
        },
        "target": validator.TARGET,
    }


def invoke(script: Path, arguments: list[Path]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [sys.executable, "-I", "-B", str(script), *map(str, arguments)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=20,
    )


def set_path(path: tuple[str | int, ...], value: Any) -> Mutation:
    def mutate(record: dict[str, Any]) -> None:
        current: Any = record
        for component in path[:-1]:
            current = current[component]
        current[path[-1]] = value

    return mutate


def require_rejection(
    validator: Path,
    root: Path,
    canonical_record: dict[str, Any],
    name: str,
    mutation: Mutation,
) -> None:
    hostile = copy.deepcopy(canonical_record)
    mutation(hostile)
    path = root / f"hostile-{name}.json"
    path.write_bytes(canonical(hostile))
    result = invoke(validator, [path])
    if result.returncode == 0 or b"FAIL:" not in result.stdout:
        fail(f"validator accepted hostile {name}: {result.stdout!r}")


def main() -> None:
    repo = Path(__file__).resolve().parents[2]
    validator_path = repo / "proofs/m1-qualification/validate-protected-worker-v3-all-kernels-build.py"
    producer = repo / "proofs/m1-qualification/produce-protected-worker-v3-all-kernels-publication-selection.py"
    validator = load_validator(validator_path)
    engine_source = (repo / "crates/ferric-engine/src/authenticated_kernel_programs.rs").read_text(encoding="ascii")
    required_engine_custody = [
        "struct M1AggregatePublicationSelectionV1",
        "finalized_artifact_sha256: [u8; 32]",
        "finalized_artifact_length: u64",
        "const M1_CURRENT_AGGREGATE_PUBLICATION_SELECTION_V1: Option<M1AggregatePublicationSelectionV1> =",
        "    None;",
    ]
    for required in required_engine_custody:
        if required not in engine_source:
            fail(f"private engine selection custody drifted: missing {required}")
    forbidden_engine_overrides = [
        "pub struct M1AggregatePublicationSelectionV1",
        "std::env", "var_os(", "var(", "from_path", "read_to_end", "read_to_string",
    ]
    for forbidden in forbidden_engine_overrides:
        if forbidden in engine_source:
            fail(f"private engine selection gained a forbidden override: {forbidden}")

    with tempfile.TemporaryDirectory(prefix="ferric-aggregate-selection-policy-") as raw:
        root = Path(raw)
        os.chmod(root, 0o700)
        ferric = root / "ferric"
        compiler = root / "fe2o3"
        ferric.mkdir()
        compiler.mkdir()
        git(ferric, "init", "-q")
        git(compiler, "init", "-q")
        (compiler / "provider.txt").write_text("provider\n", encoding="ascii")
        provider_commit = commit(compiler, "provider")
        provider_tree = git(compiler, "rev-parse", "HEAD^{tree}")
        (compiler / "compiler.txt").write_text("compiler\n", encoding="ascii")
        compiler_commit = commit(compiler, "compiler")
        compiler_tree = git(compiler, "rev-parse", "HEAD^{tree}")
        (ferric / "ferric.txt").write_text("ferric\n", encoding="ascii")
        ferric_commit = commit(ferric, "ferric")
        ferric_tree = git(ferric, "rev-parse", "HEAD^{tree}")

        record = build_record(
            validator, ferric_commit, ferric_tree, compiler_commit, compiler_tree,
            provider_commit, provider_tree,
        )
        record_path = root / "aggregate-build.json"
        record_path.write_bytes(canonical(record))
        accepted = invoke(validator_path, [record_path])
        if accepted.returncode != 0 or not accepted.stdout.startswith(
            b"PASS: canonical aggregate protected Worker V3 build record sha256="
        ):
            fail(f"validator rejected canonical record: {accepted.stdout!r}")

        mutations: list[tuple[str, Mutation]] = [
            ("schema", lambda value: value.__setitem__("hostile", True)),
            ("field", lambda value: value["source_pin_observation"]["projection"]["source_pin"].pop("compiler_module_length")),
            ("order", lambda value: value["inspection"]["kernels"].reverse()),
            ("target", set_path(("target",), "gfx942:xnack+")),
            ("cov", set_path(("source_pin_observation", "projection", "code_object_version"), 5)),
            ("roster", set_path(("source_pin_observation", "projection", "policy_kernel_symbols", 0), "hostile_kernel")),
            ("artifact", set_path(("artifact", "sha256"), digest("other-artifact"))),
            ("envelope", set_path(("source_pin_observation", "adapter_execution", "envelope_sha256"), digest("other-envelope"))),
            ("authority", set_path(("authority",), "runtime-selection-authority")),
            ("currentness", set_path(("observed_worker_v3_records", "typed_current_publication_reacquisition"), True)),
        ]
        for name, mutation in mutations:
            require_rejection(validator_path, root, record, name, mutation)

        noncanonical = root / "noncanonical.json"
        noncanonical.write_bytes(json.dumps(record, sort_keys=True).encode("ascii"))
        if invoke(validator_path, [noncanonical]).returncode == 0:
            fail("validator accepted noncanonical record serialization")
        symlink = root / "record-symlink.json"
        symlink.symlink_to(record_path)
        if invoke(validator_path, [symlink]).returncode == 0:
            fail("validator accepted a symlinked observational record")

        output = root / "selection.json"
        positive = invoke(producer, [ferric, compiler, record_path, output])
        if positive.returncode != 0 or not positive.stdout.startswith(
            b"PASS: published noncurrent aggregate publication-selection candidate sha256="
        ):
            fail(f"candidate producer rejected canonical sources and record: {positive.stdout!r}")
        candidate = json.loads(output.read_bytes())
        projection = record["source_pin_observation"]["projection"]
        if (
            output.read_bytes() != canonical(candidate)
            or stat.S_IMODE(output.stat().st_mode) != 0o600
            or candidate.get("format") != "FERRIC-M1-ALL-KERNELS-PUBLICATION-SELECTION-CANDIDATE-V1"
            or candidate.get("authority") != "publication-selection-candidate-only"
            or candidate.get("status") != "noncurrent-candidate"
            or candidate.get("selection", {}).get("source_pin") != projection["source_pin"]
            or candidate["selection"]["kernel_symbols"] != list(validator.KERNELS)
            or candidate["selection"]["code_object_version"] != 6
            or candidate["selection"]["target"] != validator.TARGET
            or candidate["selection"]["finalized_artifact_sha256"] != record["artifact"]["sha256"]
            or candidate["selection"]["worker_v3_envelope_sha256"]
            != record["observed_worker_v3_records"]["receipt_checksum_observation"]["envelope_sha256"]
            or candidate["observational_build_record"]["sha256"]
            != hashlib.sha256(record_path.read_bytes()).hexdigest()
        ):
            fail("selection candidate lost an exact binding or nonauthority field")
        replacement = invoke(producer, [ferric, compiler, record_path, output])
        if replacement.returncode == 0 or b"replacement is forbidden" not in replacement.stdout:
            fail(f"candidate producer replaced an existing output: {replacement.stdout!r}")
        output_symlink = root / "selection-symlink.json"
        output_symlink.symlink_to(output)
        symlink_output_result = invoke(
            producer, [ferric, compiler, record_path, output_symlink]
        )
        if (
            symlink_output_result.returncode == 0
            or b"replacement is forbidden" not in symlink_output_result.stdout
        ):
            fail(
                "candidate producer accepted a symlinked output: "
                f"{symlink_output_result.stdout!r}"
            )

        dirty = ferric / "dirty"
        dirty.write_text("dirty\n", encoding="ascii")
        dirty_result = invoke(producer, [ferric, compiler, record_path, root / "dirty-output.json"])
        if dirty_result.returncode == 0 or b"exact clean worktree" not in dirty_result.stdout:
            fail(f"candidate producer accepted a dirty Ferric source: {dirty_result.stdout!r}")
        dirty.unlink()

        wrong_provider = copy.deepcopy(record)
        wrong_provider["source"]["device_provider_tree"] = compiler_tree
        wrong_provider_path = root / "wrong-provider.json"
        wrong_provider_path.write_bytes(canonical(wrong_provider))
        identity_result = invoke(
            producer, [ferric, compiler, wrong_provider_path, root / "wrong-provider-output.json"]
        )
        if identity_result.returncode == 0 or b"device-provider source identity drifted" not in identity_result.stdout:
            fail(f"candidate producer accepted wrong provider identity: {identity_result.stdout!r}")

    print(
        "PASS: canonical aggregate validator and private noncurrent selection producer "
        f"rejected {len(mutations) + 8} schema, field, order, identity, target, COV6, "
        "roster, artifact, envelope, authority, replacement, symlink, and dirty-source cases"
    )


if __name__ == "__main__":
    main()
