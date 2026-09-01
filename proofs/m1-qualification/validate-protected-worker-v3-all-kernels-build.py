#!/usr/bin/env python3
"""Validate one canonical observational aggregate Worker V3 build record."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
from typing import Any, NoReturn


FORMAT = "FERRIC-M1-PROTECTED-WORKER-V3-ALL-KERNELS-BUILD-V1"
AUTHORITY = "identity-and-structure-observation-only"
NONCLAIM = (
    "This record preserves byte identities, bounded namespace custody, descriptive HSACO "
    "inspection, and output from a source-prebound typed source-pin adapter only. Its "
    "shallow checksum parsing is not typed decoding of every durable record and does not "
    "reauthenticate a current durable publication lease. It does not establish protected "
    "compiler origin, compilation or finalization authenticity, durable publication, "
    "verifier authority, GPU load or dispatch, numerical correctness, performance, Qwen "
    "model execution, or M1 qualification."
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
    "verifier-authority",
    "worker-v3-finalization-authentication",
]
TARGET = "gfx942:xnack-"
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
DEVICE_FILES = (
    "device/qwen3-all-kernels-v1/Cargo.lock",
    "device/qwen3-all-kernels-v1/Cargo.toml",
    "device/qwen3-all-kernels-v1/build.rs",
    "device/qwen3-all-kernels-v1/rust-toolchain.toml",
    "device/qwen3-all-kernels-v1/src/gemm.rs",
    "device/qwen3-all-kernels-v1/src/lib.rs",
    "device/qwen3-all-kernels-v1/src/logits.rs",
    "device/qwen3-all-kernels-v1/src/paged_decode.rs",
    "device/qwen3-all-kernels-v1/src/prefill.rs",
    "device/qwen3-all-kernels-v1/src/rmsnorm.rs",
    "device/qwen3-all-kernels-v1/src/rope_kv.rs",
    "device/qwen3-all-kernels-v1/src/swiglu.rs",
)
ADAPTER_FILES = (
    "adapters/qwen3-all-kernels-worker-v3-source-pin-v1/Cargo.lock",
    "adapters/qwen3-all-kernels-worker-v3-source-pin-v1/Cargo.toml",
    "adapters/qwen3-all-kernels-worker-v3-source-pin-v1/src/lib.rs",
    "adapters/qwen3-all-kernels-worker-v3-source-pin-v1/src/main.rs",
)
TOP_KEYS = {
    "artifact",
    "authority",
    "custody_records",
    "declared_release_entrypoint",
    "established_claims",
    "excluded_claims",
    "format",
    "inspection",
    "milestone",
    "nonclaim",
    "observed_compiler_inputs",
    "observed_production_recipe",
    "observed_worker_v3_records",
    "source",
    "source_pin_observation",
    "target",
}
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
ARTIFACT_PATH = re.compile(r"\.fe2o3-link-artifact-v1-([0-9a-f]{64})\.bin\Z")
READINESS_PATH = re.compile(
    r"\.fe2o3-worker-v3-load-readiness-v1-([0-9a-f]{64})\.(claim|envelope|receipt)\Z"
)
PUBLICATION_PATH = re.compile(r"\.fe2o3-link-publication-v1-[0-9a-f]{64}\.record\Z")
CONSUMED_PATH = re.compile(
    r"\.fe2o3-compiler-module-handoff-v3-[0-9a-f]{64}/"
    r"attempt-[0-9a-f]{64}/consumed\Z"
)
MAX_RECORD_BYTES = 512 * 1024


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def compact_bytes(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def exact(value: Any, keys: set[str], description: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        fail(f"{description} fields drifted")
    return value


def positive_u64(value: Any, description: str) -> int:
    if (
        not isinstance(value, int)
        or isinstance(value, bool)
        or value <= 0
        or value > 2**64 - 1
    ):
        fail(f"{description} is not a positive u64")
    return value


def sha256(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or SHA256.fullmatch(value) is None
        or len(set(value)) == 1
    ):
        fail(f"{description} is not a nondegenerate SHA-256")
    return value


def git_id(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or GIT_ID.fullmatch(value) is None
        or len(set(value)) == 1
    ):
        fail(f"{description} is not a nondegenerate Git identity")
    return value


def source_file_records(
    value: Any, expected_paths: tuple[str, ...], description: str
) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(expected_paths):
        fail(f"{description} roster length drifted")
    for index, (item, expected_path) in enumerate(zip(value, expected_paths, strict=True)):
        record = exact(
            item, {"git_blob", "path", "sha256", "size_bytes"}, f"{description} item"
        )
        if record["path"] != expected_path:
            fail(f"{description} order or path drifted at index {index}")
        git_id(record["git_blob"], f"{description} Git blob")
        sha256(record["sha256"], f"{description} SHA-256")
        positive_u64(record["size_bytes"], f"{description} size")
    return value


def load(path: Path) -> tuple[dict[str, Any], bytes, os.stat_result]:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
        with os.fdopen(descriptor, "rb") as source:
            before = os.fstat(source.fileno())
            if (
                not stat.S_ISREG(before.st_mode)
                or before.st_nlink != 1
                or before.st_size <= 0
                or before.st_size > MAX_RECORD_BYTES
            ):
                fail("aggregate protected-build record is not one bounded regular file")
            raw = source.read(MAX_RECORD_BYTES + 1)
            after = os.fstat(source.fileno())
    except OSError as error:
        fail(f"cannot hold aggregate protected-build record: {error}")
    if (
        len(raw) != before.st_size
        or (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
        != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
    ):
        fail("aggregate protected-build record changed during read")
    try:
        named = path.lstat()
    except OSError as error:
        fail(f"cannot revalidate named aggregate protected-build record: {error}")
    if stat.S_ISLNK(named.st_mode) or (
        before.st_dev,
        before.st_ino,
        before.st_size,
        before.st_mtime_ns,
    ) != (
        named.st_dev,
        named.st_ino,
        named.st_size,
        named.st_mtime_ns,
    ):
        fail("named aggregate protected-build record changed custody")
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"aggregate protected-build record is not JSON: {error}")
    if raw != canonical_bytes(value):
        fail("aggregate protected-build record is not canonical ASCII JSON")
    return exact(value, TOP_KEYS, "aggregate protected-build record"), raw, before


def validate(record: dict[str, Any]) -> None:
    if (
        record["format"] != FORMAT
        or record["authority"] != AUTHORITY
        or record["nonclaim"] != NONCLAIM
        or record["milestone"] != "M1"
        or record["target"] != TARGET
        or record["established_claims"] != ESTABLISHED
        or record["excluded_claims"] != EXCLUDED
        or record["declared_release_entrypoint"]
        != ["cargo-fe2o3", "authority", "release", "build", "--locked"]
    ):
        fail("aggregate protected-build scope, authority, or nonclaims drifted")

    artifact = exact(record["artifact"], {"path", "sha256", "size_bytes"}, "artifact")
    artifact_sha = sha256(artifact["sha256"], "finalized artifact SHA-256")
    artifact_size = positive_u64(artifact["size_bytes"], "finalized artifact size")
    match = ARTIFACT_PATH.fullmatch(artifact["path"]) if isinstance(artifact["path"], str) else None
    if match is None or match.group(1) != artifact_sha:
        fail("finalized artifact path does not bind its SHA-256")

    source = exact(
        record["source"],
        {"commit", "device_files", "device_provider_commit", "device_provider_tree", "tree"},
        "Ferric source",
    )
    git_id(source["commit"], "Ferric source commit")
    git_id(source["tree"], "Ferric source tree")
    git_id(source["device_provider_commit"], "device-provider commit")
    git_id(source["device_provider_tree"], "device-provider tree")
    source_file_records(source["device_files"], DEVICE_FILES, "aggregate device source")

    compiler = exact(
        record["observed_compiler_inputs"],
        {
            "cargo_fe2o3_sha256",
            "claim_embedded_closure",
            "codegen_backend_sha256",
            "commit",
            "rustc_wrapper_sha256",
            "tree",
        },
        "observed compiler inputs",
    )
    git_id(compiler["commit"], "fe2o3 compiler commit")
    git_id(compiler["tree"], "fe2o3 compiler tree")
    for field in ("cargo_fe2o3_sha256", "codegen_backend_sha256", "rustc_wrapper_sha256"):
        sha256(compiler[field], f"observed compiler {field}")
    closure = exact(
        compiler["claim_embedded_closure"],
        {
            "cargo_binding_trampoline_sha256",
            "cargo_executable_sha256",
            "cargo_fe2o3_binding_wrapper_sha256",
            "codegen_backend_sha256",
            "identity_sha256",
            "rustc_executable_sha256",
            "rustc_runtime_tree_sha256",
            "transition_protocol_version",
        },
        "compiler closure",
    )
    for field, value in closure.items():
        if field.endswith("sha256"):
            sha256(value, f"compiler closure {field}")
    if (
        closure["transition_protocol_version"] != 1
        or closure["cargo_fe2o3_binding_wrapper_sha256"] != compiler["cargo_fe2o3_sha256"]
        or closure["codegen_backend_sha256"] != compiler["codegen_backend_sha256"]
    ):
        fail("compiler closure cross-link drifted")

    recipe = exact(
        record["observed_production_recipe"],
        {"candidate_output_max_bytes", "limits", "link_options", "sha256", "unit", "worker"},
        "observed production recipe",
    )
    if (
        recipe["candidate_output_max_bytes"] != 4_194_304
        or recipe["limits"]
        != {"stderr_bytes": 65_536, "stdout_bytes": 8_388_608, "timeout_ms": 120_000}
        or recipe["link_options"]
        != [
            {"name": "code-object-version", "value": "6"},
            {"name": "opt-level", "value": "2"},
            {"name": "strip-debug", "value": "true"},
            {"name": "verify-each", "value": "true"},
        ]
    ):
        fail("aggregate production recipe or COV6 selection drifted")
    sha256(recipe["sha256"], "production recipe SHA-256")
    unit = exact(recipe["unit"], {"crate_name", "source", "working_directory_relative"}, "unit")
    if unit != {
        "crate_name": "ferric_qwen3_all_kernels_device_v1",
        "source": "src/lib.rs",
        "working_directory_relative": "device/qwen3-all-kernels-v1",
    }:
        fail("aggregate production unit drifted")
    worker = exact(
        recipe["worker"],
        {"byte_len", "llvm_build_identity", "sha256", "worker_build_identity"},
        "Worker V3 identity",
    )
    positive_u64(worker["byte_len"], "Worker V3 byte length")
    sha256(worker["sha256"], "Worker V3 SHA-256")
    worker_build_sha = (
        worker["worker_build_identity"].removeprefix("fe2o3-worker-v1-sha256-")
        if isinstance(worker["worker_build_identity"], str)
        else None
    )
    if (
        worker["llvm_build_identity"] != "7.2.4"
        or not isinstance(worker["worker_build_identity"], str)
        or not worker["worker_build_identity"].startswith("fe2o3-worker-v1-sha256-")
        or not isinstance(worker_build_sha, str)
        or SHA256.fullmatch(worker_build_sha) is None
        or len(set(worker_build_sha)) == 1
    ):
        fail("Worker V3 build identity drifted")

    inspection = exact(
        record["inspection"],
        {"authority", "format", "kernel_count", "kernels", "metadata_version", "ordering_claim", "target", "transcript_sha256"},
        "HSACO inspection",
    )
    if (
        inspection["authority"] != "descriptive-only"
        or inspection["format"] != "hsaco-v6"
        or inspection["kernel_count"] != len(KERNELS)
        or inspection["metadata_version"] != "1.2"
        or inspection["ordering_claim"] != "none"
        or inspection["target"] != TARGET
        or not isinstance(inspection["kernels"], list)
        or len(inspection["kernels"]) != len(KERNELS)
    ):
        fail("HSACO inspection target, COV6, count, or authority drifted")
    sha256(inspection["transcript_sha256"], "inspection transcript SHA-256")
    kernel_keys = {
        "explicit_argument_count", "group_segment_size_bytes", "hidden_argument_count",
        "kernarg_alignment_bytes", "kernarg_size_bytes", "name", "private_segment_size_bytes",
        "sgpr_count", "symbol", "vgpr_count", "wavefront_size",
    }
    for ordinal, (kernel, expected_name) in enumerate(zip(inspection["kernels"], KERNELS, strict=True)):
        item = exact(kernel, kernel_keys, "inspection kernel")
        if item["name"] != expected_name or item["symbol"] != f"{expected_name}.kd":
            fail(f"ordered aggregate kernel roster drifted at ordinal {ordinal}")
        for field in kernel_keys - {"name", "symbol"}:
            value = item[field]
            if not isinstance(value, int) or isinstance(value, bool) or value < 0 or value > 2**64 - 1:
                fail(f"inspection kernel {field} is outside u64")
        if item["wavefront_size"] != 64 or item["kernarg_alignment_bytes"] == 0:
            fail("inspection kernel ABI facts drifted")

    projection_wrapper = exact(
        record["source_pin_observation"], {"adapter_execution", "adapter_prebinding", "projection"}, "source-pin observation"
    )
    projection = exact(
        projection_wrapper["projection"],
        {
            "authority", "authenticates_compiler_origin", "code_object_version", "format",
            "grants_launch_authority", "grants_load_authority", "grants_publication_authority",
            "grants_verifier_authority", "policy_kernel_symbols", "program_count", "source_pin", "target",
        },
        "source-pin projection",
    )
    if (
        projection["authority"] != "identity-observation-only"
        or projection["authenticates_compiler_origin"] is not False
        or projection["code_object_version"] != 6
        or projection["format"] != "ferric.m1-all-kernels-worker-v3-source-pin.v1"
        or projection["grants_launch_authority"] is not False
        or projection["grants_load_authority"] is not False
        or projection["grants_publication_authority"] is not False
        or projection["grants_verifier_authority"] is not False
        or projection["policy_kernel_symbols"] != list(KERNELS)
        or projection["program_count"] != len(KERNELS)
        or projection["target"] != TARGET
    ):
        fail("source-pin projection scope, order, target, COV6, or nonauthority drifted")
    source_pin = exact(
        projection["source_pin"],
        {
            "compiler_handoff_length", "compiler_handoff_sha256", "compiler_module_length",
            "compiler_module_sha256", "symbol_manifest_length", "symbol_manifest_sha256",
        },
        "aggregate source pin",
    )
    for field, value in source_pin.items():
        if field.endswith("_length"):
            positive_u64(value, f"source-pin {field}")
        else:
            sha256(value, f"source-pin {field}")

    prebinding = exact(
        projection_wrapper["adapter_prebinding"],
        {"binding_git_blob", "binding_sha256", "binary_sha256", "binary_size_bytes", "name", "protocol", "source_closure_sha256", "source_files"},
        "source-pin adapter prebinding",
    )
    git_id(prebinding["binding_git_blob"], "adapter binding Git blob")
    for field in ("binding_sha256", "binary_sha256", "source_closure_sha256"):
        sha256(prebinding[field], f"adapter prebinding {field}")
    positive_u64(prebinding["binary_size_bytes"], "adapter binary size")
    adapter_files = source_file_records(prebinding["source_files"], ADAPTER_FILES, "adapter source")
    if (
        prebinding["name"] != "ferric-qwen3-all-kernels-worker-v3-source-pin-v1"
        or prebinding["protocol"] != "ferric.m1-all-kernels-worker-v3-source-pin.v1"
        or prebinding["source_closure_sha256"] != hashlib.sha256(compact_bytes(adapter_files)).hexdigest()
    ):
        fail("source-pin adapter prebinding drifted")
    adapter_execution = exact(
        projection_wrapper["adapter_execution"], {"envelope_sha256", "output_sha256"}, "adapter execution"
    )
    for field in adapter_execution:
        sha256(adapter_execution[field], f"adapter execution {field}")

    worker_records = exact(
        record["observed_worker_v3_records"],
        {
            "checksummed_claim", "declared_finalization_identity_sha256",
            "declared_finalized_output_identity_sha256", "declared_publication_identity_sha256",
            "receipt_checksum_observation", "shallow_worker_v3_binding_observation",
            "typed_current_publication_reacquisition", "typed_durable_record_decoding",
        },
        "observed Worker V3 records",
    )
    for field in (
        "declared_finalization_identity_sha256", "declared_finalized_output_identity_sha256",
        "declared_publication_identity_sha256",
    ):
        sha256(worker_records[field], f"Worker V3 {field}")
    if (
        worker_records["typed_current_publication_reacquisition"] is not False
        or worker_records["typed_durable_record_decoding"] is not False
    ):
        fail("observational record promoted typed publication authority")
    claim = exact(
        worker_records["checksummed_claim"], {"backend_receipt_sha256", "sha256", "size_bytes"}, "checksummed claim"
    )
    for field in ("backend_receipt_sha256", "sha256"):
        sha256(claim[field], f"claim {field}")
    positive_u64(claim["size_bytes"], "claim size")
    readiness = exact(
        worker_records["receipt_checksum_observation"],
        {"backend_receipt_sha256", "claim_sha256", "claim_size_bytes", "envelope_sha256", "envelope_size_bytes", "receipt_identity_sha256"},
        "load-readiness checksum observation",
    )
    for field in ("backend_receipt_sha256", "claim_sha256", "envelope_sha256", "receipt_identity_sha256"):
        sha256(readiness[field], f"load-readiness {field}")
    positive_u64(readiness["claim_size_bytes"], "load-readiness claim size")
    positive_u64(readiness["envelope_size_bytes"], "load-readiness envelope size")
    if (
        readiness["backend_receipt_sha256"] != claim["backend_receipt_sha256"]
        or readiness["claim_sha256"] != claim["sha256"]
        or readiness["claim_size_bytes"] != claim["size_bytes"]
        or adapter_execution["envelope_sha256"] != readiness["envelope_sha256"]
    ):
        fail("claim, envelope, adapter, or readiness identity cross-link drifted")
    binding = exact(
        worker_records["shallow_worker_v3_binding_observation"],
        {
            "compiler_handoff_sha256", "finalization_sha256", "finalized_output_sha256",
            "finalized_output_size_bytes", "publication_intent_sha256", "raw_inspection_sha256",
            "raw_output_sha256", "raw_output_size_bytes", "source_evidence_sha256",
        },
        "shallow Worker V3 binding",
    )
    for field, value in binding.items():
        if field.endswith("_sha256"):
            sha256(value, f"Worker V3 binding {field}")
        else:
            positive_u64(value, f"Worker V3 binding {field}")
    if (
        binding["finalized_output_sha256"] != artifact_sha
        or binding["finalized_output_size_bytes"] != artifact_size
    ):
        fail("finalized artifact or Worker V3 binding cross-link drifted")

    custody = record["custody_records"]
    expected_kinds = {
        ".codegen-generation-v1", ".fe2o3-artifacts.lock", ".fe2o3-attempts-v1",
        ".fe2o3-owned-v1", "artifact", "claim", "consumed", "envelope", "publication", "receipt",
    }
    if not isinstance(custody, list) or len(custody) != len(expected_kinds):
        fail("artifact custody roster length drifted")
    by_kind: dict[str, dict[str, Any]] = {}
    previous_path = ""
    for item in custody:
        entry = exact(item, {"kind", "path", "sha256", "size_bytes"}, "custody record")
        if not isinstance(entry["kind"], str) or entry["kind"] in by_kind:
            fail("artifact custody kind is invalid or duplicated")
        if not isinstance(entry["path"], str) or entry["path"] <= previous_path:
            fail("artifact custody paths are not in exact lexical order")
        previous_path = entry["path"]
        sha256(entry["sha256"], "custody SHA-256")
        if entry["kind"] == ".fe2o3-artifacts.lock":
            if entry["size_bytes"] != 0:
                fail("artifact lock custody snapshot is not empty")
        else:
            positive_u64(entry["size_bytes"], "custody size")
        by_kind[entry["kind"]] = entry
    if set(by_kind) != expected_kinds:
        fail("artifact custody kinds drifted")
    for fixed in (
        ".codegen-generation-v1",
        ".fe2o3-artifacts.lock",
        ".fe2o3-attempts-v1",
        ".fe2o3-owned-v1",
    ):
        if by_kind[fixed]["path"] != fixed:
            fail(f"fixed artifact custody path drifted: {fixed}")
    readiness_paths = {
        kind: READINESS_PATH.fullmatch(by_kind[kind]["path"])
        for kind in ("claim", "envelope", "receipt")
    }
    readiness_namespaces = {
        match.group(1) for match in readiness_paths.values() if match is not None
    }
    if (
        by_kind["artifact"]["path"] != artifact["path"]
        or by_kind["artifact"]["sha256"] != artifact_sha
        or by_kind["artifact"]["size_bytes"] != artifact_size
        or by_kind["claim"]["sha256"] != claim["sha256"]
        or by_kind["claim"]["size_bytes"] != claim["size_bytes"]
        or by_kind["envelope"]["sha256"] != readiness["envelope_sha256"]
        or by_kind["envelope"]["size_bytes"] != readiness["envelope_size_bytes"]
        or any(
            match is None or match.group(2) != kind
            for kind, match in readiness_paths.items()
        )
        or len(readiness_namespaces) != 1
        or PUBLICATION_PATH.fullmatch(by_kind["publication"]["path"]) is None
        or CONSUMED_PATH.fullmatch(by_kind["consumed"]["path"]) is None
    ):
        fail("artifact, claim, or envelope custody cross-link drifted")


def load_and_validate(path: Path) -> tuple[dict[str, Any], bytes, os.stat_result]:
    record, raw, metadata = load(path)
    validate(record)
    return record, raw, metadata


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate-protected-worker-v3-all-kernels-build.py RECORD")
    _, raw, _ = load_and_validate(Path(sys.argv[1]))
    print(
        "PASS: canonical aggregate protected Worker V3 build record "
        f"sha256={hashlib.sha256(raw).hexdigest()}"
    )


if __name__ == "__main__":
    main()
