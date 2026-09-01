#!/usr/bin/env python3
"""Exercise aggregate Worker V3 build production with an isolated fixture."""

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
from typing import Any, NoReturn


HISTORICAL_PRODUCER_SHA256 = (
    "25e2049beff3958076f83d596a299cd2d3fb7dc13bc2fb32d32e2a130fc107da"
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
SOURCE_FILES = (
    "Cargo.lock",
    "Cargo.toml",
    "build.rs",
    "rust-toolchain.toml",
    "src/gemm.rs",
    "src/lib.rs",
    "src/logits.rs",
    "src/paged_decode.rs",
    "src/prefill.rs",
    "src/rmsnorm.rs",
    "src/rope_kv.rs",
    "src/swiglu.rs",
)
ADAPTER_SOURCE_FILES = (
    "Cargo.lock",
    "Cargo.toml",
    "src/lib.rs",
    "src/main.rs",
)
ADAPTER_RELATIVE = "adapters/qwen3-all-kernels-worker-v3-source-pin-v1"
BINDING_RELATIVE = f"{ADAPTER_RELATIVE}/SOURCE_PIN_ADAPTER_BINDING_V1.json"
BINDING_NONCLAIM = (
    "This source-controlled record pre-binds one executable identity to the exact adapter "
    "source closure. It is not a reproducible-build proof, compiler-origin attestation, "
    "semantic-correctness proof, or runtime authority."
)
CHECKED_BINDING_GIT_BLOB = "d33dc378bb0b95070ec46f6ce62cbbe82eecc3f4"
CHECKED_BINDING_SHA256 = "f65b80c347db49c435a3eee0a5156c069d7e5b15ddb2c3e041ebcfa5f963e66c"
CHECKED_BINARY_SHA256 = "5bd21ea2c9739756a3ce55729bb63ea1ee5f24c76f834f23ff514f2ad39dbcc0"
CHECKED_BINARY_SIZE_BYTES = 7_622_704
CHECKED_SOURCE_CLOSURE_SHA256 = (
    "daaf12a48aedf14fa868f67842ad300222a4c703bc4d7913b8ba39c288318190"
)


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_historical_helpers(repo: Path) -> ModuleType:
    path = repo / "proofs/m1-qualification/test-protected-worker-v3-build-producer-policy.py"
    specification = importlib.util.spec_from_file_location(
        "_ferric_historical_protected_build_test", path
    )
    if specification is None or specification.loader is None:
        fail("cannot load historical protected-build fixture helpers")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def canonical(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n").encode(
        "ascii"
    )


def compact(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def git_blob(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data, usedforsecurity=False).hexdigest()


def validate_checked_binding(repo: Path) -> None:
    binding_path = repo / BINDING_RELATIVE
    raw = binding_path.read_bytes()
    try:
        binding = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"checked aggregate adapter binding is not JSON: {error}")
    if raw != canonical(binding):
        fail("checked aggregate adapter binding is not canonical ASCII JSON")
    if (
        hashlib.sha256(raw).hexdigest() != CHECKED_BINDING_SHA256
        or git_blob(raw) != CHECKED_BINDING_GIT_BLOB
    ):
        fail("checked aggregate adapter binding identity drifted")
    if not isinstance(binding, dict) or set(binding) != {
        "authority",
        "binary",
        "format",
        "nonclaim",
        "protocol",
        "source_closure_sha256",
        "source_files",
    }:
        fail("checked aggregate adapter binding fields drifted")
    binary = binding["binary"]
    if (
        binding["authority"] != "binary-identity-prebinding-only"
        or binding["format"]
        != "FERRIC-M1-ALL-KERNELS-SOURCE-PIN-ADAPTER-BINDING-V1"
        or binding["nonclaim"] != BINDING_NONCLAIM
        or binding["protocol"] != "ferric.m1-all-kernels-worker-v3-source-pin.v1"
        or not isinstance(binary, dict)
        or binary
        != {
            "name": "ferric-qwen3-all-kernels-worker-v3-source-pin-v1",
            "sha256": CHECKED_BINARY_SHA256,
            "size_bytes": CHECKED_BINARY_SIZE_BYTES,
        }
    ):
        fail("checked aggregate adapter binary prebinding drifted")
    expected_paths = [f"{ADAPTER_RELATIVE}/{path}" for path in ADAPTER_SOURCE_FILES]
    source_records = []
    for path in expected_paths:
        data = (repo / path).read_bytes()
        source_records.append(
            {
                "git_blob": git_blob(data),
                "path": path,
                "sha256": hashlib.sha256(data).hexdigest(),
                "size_bytes": len(data),
            }
        )
    if binding["source_files"] != source_records:
        fail("checked aggregate adapter source records drifted")
    closure = hashlib.sha256(compact(source_records)).hexdigest()
    if (
        closure != CHECKED_SOURCE_CLOSURE_SHA256
        or binding["source_closure_sha256"] != closure
    ):
        fail("checked aggregate adapter source closure drifted")


def inspection(
    kernels: tuple[str, ...] = KERNELS,
    descriptor_override: tuple[int, str] | None = None,
    index_override: tuple[int, int] | None = None,
) -> bytes:
    lines = [
        "format: hsaco-v6",
        "authority: descriptive-only",
        "metadata-version: 1.2",
        "target: gfx942:xnack-",
        "printf-metadata: false",
        f"kernels: {len(kernels)}",
    ]
    for index, kernel in enumerate(kernels):
        rendered_index = index_override[1] if index_override and index == index_override[0] else index
        symbol = (
            descriptor_override[1]
            if descriptor_override and index == descriptor_override[0]
            else f"{kernel}.kd"
        )
        lines.append(
            f"kernel[{rendered_index}]: name={kernel} symbol={symbol} "
            "kernarg-bytes=304 kernarg-align=8 wave=64 lds-bytes=0 "
            "private-bytes=0 explicit-args=6 hidden-args=13 sgprs=84 vgprs=11"
        )
    return ("\n".join(lines) + "\n").encode("ascii")


def source_pin_projection() -> dict[str, Any]:
    return {
        "authority": "identity-observation-only",
        "authenticates_compiler_origin": False,
        "code_object_version": 6,
        "format": "ferric.m1-all-kernels-worker-v3-source-pin.v1",
        "grants_launch_authority": False,
        "grants_load_authority": False,
        "grants_publication_authority": False,
        "grants_verifier_authority": False,
        "policy_kernel_symbols": list(KERNELS),
        "program_count": len(KERNELS),
        "source_pin": {
            "compiler_handoff_length": 31_337,
            "compiler_handoff_sha256": hashlib.sha256(b"compiler-handoff").hexdigest(),
            "compiler_module_length": 29_911,
            "compiler_module_sha256": hashlib.sha256(b"compiler-module").hexdigest(),
            "symbol_manifest_length": 2_117,
            "symbol_manifest_sha256": hashlib.sha256(b"symbol-manifest").hexdigest(),
        },
        "target": "gfx942:xnack-",
    }


def invoke(producer: Path, arguments: list[Path]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [sys.executable, "-I", "-B", str(producer), *map(str, arguments)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
    )


def require_rejection(
    producer: Path, arguments: list[Path], output: Path, description: str
) -> None:
    existed_before = output.exists()
    result = invoke(producer, [*arguments, output])
    if (
        result.returncode == 0
        or b"FAIL:" not in result.stdout
        or (not existed_before and output.exists())
    ):
        fail(f"producer accepted {description}: {result.stdout!r}")


def main() -> None:
    repo = Path(__file__).resolve().parents[2]
    producer = (
        repo
        / "proofs/m1-qualification/produce-protected-worker-v3-all-kernels-build.py"
    )
    historical = repo / "proofs/m1-qualification/produce-protected-worker-v3-build.py"
    if hashlib.sha256(historical.read_bytes()).hexdigest() != HISTORICAL_PRODUCER_SHA256:
        fail("historical singleton SwiGLU producer changed")
    validate_checked_binding(repo)
    producer_source = producer.read_text(encoding="ascii")
    for required in [
        "invoke_source_pin_adapter(adapter, files[\"envelope\"])",
        "start_new_session=True",
        "selectors.DefaultSelector()",
        "terminate_process_group(process)",
        "held_adapter_sources = adapter_binding(",
        "revalidate_artifact_namespace(artifact_namespace, files)",
        "revalidate_git_files(source_repo, source_files, held_device_sources)",
        '"program_count"] != len(KERNELS)',
        'set(names) != set(KERNELS)',
        '"ordering_claim": "none"',
        'AUTHORITY = "identity-and-structure-observation-only"',
        '"typed_current_publication_reacquisition": False',
    ]:
        if required not in producer_source:
            fail(f"aggregate producer lost a required exact-policy step: {required}")

    helpers = load_historical_helpers(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-aggregate-build-producer-") as raw:
        root = Path(raw)
        os.chmod(root, 0o700)

        compiler_repo = root / "compiler"
        compiler_repo.mkdir()
        helpers.git(compiler_repo, "init", "-q")
        (compiler_repo / "provider.txt").write_text("provider\n", encoding="ascii")
        provider = helpers.commit(compiler_repo, "provider")
        (compiler_repo / "compiler.txt").write_text("compiler\n", encoding="ascii")
        helpers.commit(compiler_repo, "compiler")

        source_repo = root / "source"
        device = source_repo / "device/qwen3-all-kernels-v1"
        device.mkdir(parents=True)
        helpers.git(source_repo, "init", "-q")
        for relative in SOURCE_FILES:
            path = device / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"// synthetic {relative}\n", encoding="ascii")
        adapter_source = source_repo / ADAPTER_RELATIVE
        for relative in ADAPTER_SOURCE_FILES:
            path = adapter_source / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"// synthetic adapter {relative}\n", encoding="ascii")
        (device / "Cargo.toml").write_text(
            "\n".join(
                [
                    "[package]",
                    'name = "ferric-qwen3-all-kernels-device-v1"',
                    'version = "0.1.0"',
                    'edition = "2024"',
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
        helpers.commit(source_repo, "aggregate and adapter source")

        inspection_path = root / "inspection.txt"
        inspection_path.write_bytes(inspection())
        mutation_mode = root / "inspection-mutation-mode"
        mutation_mode.write_text("none\n", encoding="ascii")
        artifact_root = root / "artifacts"
        publication_path = artifact_root / f".fe2o3-link-publication-v1-{'1' * 64}.record"
        cargo = root / "cargo-fe2o3"
        cargo.write_text(
            "#!/usr/bin/python3\n"
            "from pathlib import Path\n"
            "import os\n"
            "import sys\n"
            f"inspection = Path({str(inspection_path)!r})\n"
            f"mode = Path({str(mutation_mode)!r}).read_text(encoding='ascii').strip()\n"
            "sys.stdout.buffer.write(inspection.read_bytes())\n"
            "if mode == 'durable-record':\n"
            f"    target = Path({str(publication_path)!r})\n"
            "    replacement = target.with_name(target.name + '.replacement')\n"
            "    replacement.write_bytes(b'FE2O3-DURABLE-LINK-V1\\0substituted')\n"
            "    os.replace(replacement, target)\n"
            "elif mode == 'namespace':\n"
            f"    Path({str(artifact_root / 'hostile-extra')!r}).write_bytes(b'hostile')\n"
            "elif mode == 'device-source':\n"
            f"    target = Path({str(device / 'src/lib.rs')!r})\n"
            "    replacement = target.with_name(target.name + '.replacement')\n"
            "    replacement.write_bytes(target.read_bytes())\n"
            "    os.replace(replacement, target)\n"
            "elif mode == 'adapter-binding':\n"
            f"    target = Path({str(source_repo / BINDING_RELATIVE)!r})\n"
            "    replacement = target.with_name(target.name + '.replacement')\n"
            "    replacement.write_bytes(target.read_bytes())\n"
            "    os.replace(replacement, target)\n",
            encoding="ascii",
        )
        cargo.chmod(stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
        wrapper = root / "fe2o3-rustc-wrapper"
        wrapper.write_bytes(b"synthetic wrapper")
        backend = root / "librustc_codegen_fe2o3.so"
        backend.write_bytes(b"synthetic backend")

        artifact = b"synthetic aggregate finalized hsaco"
        artifact_digest = helpers.sha256(artifact).hex()
        claim, backend_identity, namespace = helpers.exact_claim(
            artifact, cargo.read_bytes(), backend.read_bytes()
        )
        envelope = b"synthetic aggregate receipt-bearing V2 envelope"
        readiness = helpers.exact_readiness(claim, envelope, backend_identity)
        artifact_root.mkdir()
        os.chmod(artifact_root, 0o700)
        files = {
            ".codegen-generation-v1": b"fe2o3-codegen-generation-v1\0fixture",
            ".fe2o3-artifacts.lock": b"",
            ".fe2o3-attempts-v1": b"FE2O3-ATTEMPTS-V1\0fixture",
            f".fe2o3-link-artifact-v1-{artifact_digest}.bin": artifact,
            f".fe2o3-link-publication-v1-{'1' * 64}.record": (
                b"FE2O3-DURABLE-LINK-V1\0fixture"
            ),
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

        projection_path = root / "source-pin.json"
        projection_path.write_bytes(canonical(source_pin_projection()))
        adapter = root / "ferric-qwen3-all-kernels-worker-v3-source-pin-v1"
        adapter.write_text(
            "#!/usr/bin/python3\n"
            "import hashlib\n"
            "from pathlib import Path\n"
            "import sys\n"
            f"expected = {hashlib.sha256(envelope).hexdigest()!r}\n"
            "if len(sys.argv) != 2 or "
            "hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest() != expected:\n"
            "    raise SystemExit(41)\n"
            f"sys.stdout.buffer.write(Path({str(projection_path)!r}).read_bytes())\n",
            encoding="ascii",
        )
        adapter.chmod(stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)

        adapter_records = []
        for relative in ADAPTER_SOURCE_FILES:
            full_relative = f"{ADAPTER_RELATIVE}/{relative}"
            path = source_repo / full_relative
            raw_source = path.read_bytes()
            adapter_records.append(
                {
                    "git_blob": helpers.git(source_repo, "rev-parse", f"HEAD:{full_relative}"),
                    "path": full_relative,
                    "sha256": hashlib.sha256(raw_source).hexdigest(),
                    "size_bytes": len(raw_source),
                }
            )
        binding = {
            "authority": "binary-identity-prebinding-only",
            "binary": {
                "name": "ferric-qwen3-all-kernels-worker-v3-source-pin-v1",
                "sha256": hashlib.sha256(adapter.read_bytes()).hexdigest(),
                "size_bytes": adapter.stat().st_size,
            },
            "format": "FERRIC-M1-ALL-KERNELS-SOURCE-PIN-ADAPTER-BINDING-V1",
            "nonclaim": BINDING_NONCLAIM,
            "protocol": "ferric.m1-all-kernels-worker-v3-source-pin.v1",
            "source_closure_sha256": hashlib.sha256(
                helpers.canonical_compact(adapter_records)
            ).hexdigest(),
            "source_files": adapter_records,
        }
        binding_path = source_repo / BINDING_RELATIVE
        binding_path.write_bytes(canonical(binding))
        helpers.commit(source_repo, "pre-bind aggregate source-pin adapter")

        config = root / "production-config.json"
        config.write_bytes(
            helpers.canonical_compact(
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
                            "crate_name": "ferric_qwen3_all_kernels_device_v1",
                            "source": "src/lib.rs",
                            "working_directory": str(device),
                        }
                    ],
                    "worker": {
                        "byte_len": 42,
                        "llvm_build_identity": "7.2.4",
                        "path": str(root / "worker"),
                        "sha256": hashlib.sha256(b"synthetic aggregate worker").hexdigest(),
                        "worker_build_identity": "fe2o3-worker-v1-sha256-"
                        + hashlib.sha256(b"synthetic aggregate worker build").hexdigest(),
                    },
                }
            )
        )
        arguments = [
            source_repo,
            compiler_repo,
            config,
            artifact_root,
            cargo,
            wrapper,
            backend,
            adapter,
        ]
        output = root / "record.json"
        positive = invoke(producer, [*arguments, output])
        if positive.returncode != 0 or not positive.stdout.startswith(
            b"PASS: published aggregate protected Worker V3 build record sha256="
        ):
            fail(f"producer rejected canonical aggregate fixture: {positive.stdout!r}")
        output_bytes = output.read_bytes()
        record = json.loads(output_bytes)
        if output_bytes != canonical(record):
            fail("aggregate producer output is not canonical JSON")
        if (
            record.get("format")
            != "FERRIC-M1-PROTECTED-WORKER-V3-ALL-KERNELS-BUILD-V1"
            or record.get("authority") != "identity-and-structure-observation-only"
            or record.get("inspection", {}).get("kernel_count") != 12
            or {item["name"] for item in record["inspection"]["kernels"]} != set(KERNELS)
            or record.get("inspection", {}).get("ordering_claim") != "none"
            or record.get("source_pin_observation", {}).get("projection")
            != source_pin_projection()
            or record["source_pin_observation"]["adapter_prebinding"]["binary_sha256"]
            != hashlib.sha256(adapter.read_bytes()).hexdigest()
            or record["observed_worker_v3_records"]["typed_durable_record_decoding"]
            is not False
            or record["observed_worker_v3_records"][
                "typed_current_publication_reacquisition"
            ]
            is not False
            or len(record.get("source", {}).get("device_files", [])) != len(SOURCE_FILES)
            or b"/tmp/" in output_bytes
        ):
            fail("aggregate producer emitted a drifted or machine-local record")
        validator = (
            repo
            / "proofs/m1-qualification/validate-protected-worker-v3-all-kernels-build.py"
        )
        validated = subprocess.run(
            [sys.executable, "-I", "-B", str(validator), str(output)],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=30,
        )
        if validated.returncode != 0 or not validated.stdout.startswith(
            b"PASS: canonical aggregate protected Worker V3 build record sha256="
        ):
            fail(f"canonical validator rejected producer output: {validated.stdout!r}")

        require_rejection(producer, arguments, output, "a preexisting output")

        extra_directory = artifact_root / "hostile-empty-directory"
        extra_directory.mkdir()
        require_rejection(
            producer,
            arguments,
            root / "hostile-empty-directory.json",
            "an extra empty artifact directory",
        )
        extra_directory.rmdir()

        adapter_bytes = adapter.read_bytes()
        adapter.write_bytes(b"#!/bin/sh\nexit 0\n")
        adapter.chmod(stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
        require_rejection(
            producer,
            arguments,
            root / "hostile-adapter-substitution.json",
            "an adapter executable substitution",
        )
        adapter.write_bytes(adapter_bytes)
        adapter.chmod(stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)

        hostile_inspections = [
            (inspection(KERNELS[:-1]), "an 11-kernel inspection"),
            (inspection((*KERNELS, "hostile_extra_kernel")), "a 13-kernel inspection"),
            (inspection((*KERNELS[:-1], KERNELS[0])), "a duplicate-kernel inspection"),
            (
                inspection(descriptor_override=(4, "qwen3_paged_kv_write_hostile.kd")),
                "a substituted descriptor",
            ),
            (inspection(index_override=(7, 19)), "a noncanonical kernel index"),
        ]
        for index, (hostile, description) in enumerate(hostile_inspections):
            inspection_path.write_bytes(hostile)
            require_rejection(
                producer, arguments, root / f"hostile-inspection-{index}.json", description
            )
        inspection_path.write_bytes(inspection())
        inspection_path.write_bytes(b"X" * (128 * 1024 + 1))
        require_rejection(
            producer,
            arguments,
            root / "hostile-inspection-overflow.json",
            "inspector output overflow",
        )
        inspection_path.write_bytes(inspection())

        canonical_projection = source_pin_projection()
        hostile_projections = []
        wrong_target = copy.deepcopy(canonical_projection)
        wrong_target["target"] = "gfx942:xnack+"
        hostile_projections.append((canonical(wrong_target), "a wrong source-pin target"))
        wrong_count = copy.deepcopy(canonical_projection)
        wrong_count["program_count"] = 11
        hostile_projections.append((canonical(wrong_count), "an 11-program source pin"))
        swapped = copy.deepcopy(canonical_projection)
        swapped["policy_kernel_symbols"][0], swapped["policy_kernel_symbols"][1] = (
            swapped["policy_kernel_symbols"][1],
            swapped["policy_kernel_symbols"][0],
        )
        hostile_projections.append((canonical(swapped), "a reordered policy roster"))
        missing_coordinate = copy.deepcopy(canonical_projection)
        del missing_coordinate["source_pin"]["compiler_module_sha256"]
        hostile_projections.append((canonical(missing_coordinate), "a missing source coordinate"))
        for field in [
            "compiler_handoff_sha256",
            "compiler_module_sha256",
            "symbol_manifest_sha256",
        ]:
            hostile = copy.deepcopy(canonical_projection)
            hostile["source_pin"][field] = "0" * 64
            hostile_projections.append((canonical(hostile), f"a degenerate {field}"))
        for field in [
            "compiler_handoff_length",
            "compiler_module_length",
            "symbol_manifest_length",
        ]:
            for value, label in [(0, "zero"), (2**64, "overflowed")]:
                hostile = copy.deepcopy(canonical_projection)
                hostile["source_pin"][field] = value
                hostile_projections.append(
                    (canonical(hostile), f"a {label} {field}")
                )
        hostile_projections.append(
            (
                helpers.canonical_compact(canonical_projection),
                "a noncanonical source-pin serialization",
            )
        )
        for index, (hostile, description) in enumerate(hostile_projections):
            projection_path.write_bytes(hostile)
            require_rejection(
                producer, arguments, root / f"hostile-source-pin-{index}.json", description
            )
        projection_path.write_bytes(canonical(canonical_projection))
        projection_path.write_bytes(b"Y" * (64 * 1024 + 1))
        require_rejection(
            producer,
            arguments,
            root / "hostile-adapter-overflow.json",
            "source-pin adapter output overflow",
        )
        projection_path.write_bytes(canonical(canonical_projection))

        envelope_path = artifact_root / next(
            name for name in files if name.endswith(".envelope")
        )
        receipt_path = artifact_root / next(
            name for name in files if name.endswith(".receipt")
        )
        for index, (hostile_envelope, description) in enumerate(
            [
                (b"synthetic aggregate V1 envelope", "a V1 envelope"),
                (b"\x00malformed aggregate envelope", "a malformed envelope"),
            ]
        ):
            envelope_path.write_bytes(hostile_envelope)
            receipt_path.write_bytes(
                helpers.exact_readiness(claim, hostile_envelope, backend_identity)
            )
            require_rejection(
                producer,
                arguments,
                root / f"hostile-envelope-{index}.json",
                description,
            )
        envelope_path.write_bytes(envelope)
        receipt_path.write_bytes(readiness)

        publication_bytes = publication_path.read_bytes()
        mutation_mode.write_text("durable-record\n", encoding="ascii")
        require_rejection(
            producer,
            arguments,
            root / "hostile-durable-record-race.json",
            "a durable-record replacement after open",
        )
        publication_path.write_bytes(publication_bytes)
        mutation_mode.write_text("namespace\n", encoding="ascii")
        require_rejection(
            producer,
            arguments,
            root / "hostile-namespace-race.json",
            "an artifact namespace addition after open",
        )
        (artifact_root / "hostile-extra").unlink()
        mutation_mode.write_text("device-source\n", encoding="ascii")
        require_rejection(
            producer,
            arguments,
            root / "hostile-device-source-race.json",
            "a committed device-source replacement after open",
        )
        mutation_mode.write_text("adapter-binding\n", encoding="ascii")
        require_rejection(
            producer,
            arguments,
            root / "hostile-adapter-binding-race.json",
            "a source-controlled adapter-binding replacement after open",
        )
        mutation_mode.write_text("none\n", encoding="ascii")

        (source_repo / "untracked").write_text("dirty\n", encoding="ascii")
        require_rejection(
            producer, arguments, root / "dirty-source.json", "a dirty Ferric source checkout"
        )

    print(
        "PASS: checked aggregate adapter binding and Worker V3 producer accepted exact "
        "source-prebound 12-kernel custody and rejected adapter, envelope, coordinate, "
        "bound, kernel, durable record, namespace, output, and source mutations"
    )


if __name__ == "__main__":
    main()
