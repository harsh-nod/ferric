#!/usr/bin/env python3
"""Validate Ferric's exact protected Worker V3 SwiGLU build record."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import stat
import sys
from typing import Any, NoReturn


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
SOURCE_COMMIT = "57f6cfdf4b3f5177a556159d1e548b25b63a1541"
SOURCE_TREE = "c7ca664c20a9d30d955161b5f3f924f203cf4770"
PROVIDER_COMMIT = "06c74c64506f15883d64c5ab2ca476561909181d"
PROVIDER_TREE = "4dd036b683c4e8c0fbb3068cb70d59eef4a482bd"
COMPILER_COMMIT = "21e4c10609a7b44687153fc3484d1156b4eb4def"
COMPILER_TREE = "b8727d7cbcb640461329654937a66a63f15fe514"
ARTIFACT_SHA256 = "57ecb86b40db136237e65a5fae04c955f2c92fe3347c085ec5c806984fc6afa7"
CONFIG_SHA256 = "da719f9a29860c407eeccb5d0a51a7cfde692dee5066a846801254f5bbc3412e"
CARGO_FE2O3_SHA256 = "5f92afe883dbc572797aa2a2254a95be16973dc372099196bcbe162b68ee7d96"
RUSTC_WRAPPER_SHA256 = "eaa277f44be0de79dd122e6b858ac50ef1d521b831175d0aeafc93729ec064c0"
BACKEND_SHA256 = "dc2f55784cd9d88ddb5dd7a4c97a8a5fac1cd30f810332c065f01c23d85f8eae"
COMPILER_CLOSURE = {
    "cargo_binding_trampoline_sha256": "21ef8b92cc1d17450b78949736de609f42977f47967f6549e28517f2722bb7f6",
    "cargo_executable_sha256": "c9ad606cb1dbb4a65aa27c80be88ed61eb2b811b6450eeec6794f60ed78b94a3",
    "cargo_fe2o3_binding_wrapper_sha256": CARGO_FE2O3_SHA256,
    "codegen_backend_sha256": BACKEND_SHA256,
    "identity_sha256": "97664a82bf361020647e36634e90afa30ccc4958c85b2da62baaa01303d75ef8",
    "rustc_executable_sha256": "08dfef109ad22d90556dbd2f964543cd93843dcd75a2e9792c173667392a1950",
    "rustc_runtime_tree_sha256": "ec3968ae686e8872ded5e5de84466d5bcaa2cc3b22bf2298176c4c52da903b35",
    "transition_protocol_version": 1,
}
CLAIM_SHA256 = "401b5b2b54190e7bd0e0115da9aa85b17187631e9c9ee2057bf4655c456083e0"
ENVELOPE_SHA256 = "093b45da9da3b6859553345aa38e5789aad4949b725e33e4e4d6620045455ed1"
RECEIPT_SHA256 = "2f82eb86387b180c1860fb8a4ea013e489036d994ae8e4a2a054cab922677158"
PUBLICATION_RECORD_SHA256 = "dade9685c28493a4761f78ad997e155c992da88b61a501a771752b17b1e25ae0"
READINESS_KEY = "058b9498ba96b0d6969ed60bb6599da860d0c0e4528e48b36bb8ef14b17014a8"
PUBLICATION_FILE_ID = "ece47d41f4765c917623310efdb48a607c007cbdb95c58b4c283b0b78b96f3d4"
BACKEND_RECEIPT_SHA256 = "476b3aaa82b575312956bad8ba237981e8be70186a5e337ba7970cd308c5e6be"
READINESS_RECEIPT_IDENTITY = "d323a22995cc8c663a02618c0f3f735ed6bd50d281bd70e5f0c2b402c5a1668f"
FINALIZATION_IDENTITY = "cbd8637d801d1e0a7ab077766bc979082e5423bf80f4ab106ae7b0ec359225dd"
PUBLICATION_IDENTITY = "3de815844502e69df1393df47e77569673d44c35ae4ef262019b8461c3999b1b"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
MAX_RECORD_BYTES = 64_000

TOP_KEYS = {
    "artifact",
    "authority",
    "compiler",
    "custody_records",
    "established_claims",
    "excluded_claims",
    "format",
    "inspection",
    "milestone",
    "nonclaim",
    "production_recipe",
    "publication",
    "release_entrypoint",
    "source",
    "target",
}


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def exact(value: Any, keys: set[str], description: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        fail(f"{description} fields drifted")
    return value


def require_sha(value: Any, description: str) -> str:
    if not isinstance(value, str) or SHA256.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def load(path: Path) -> tuple[dict[str, Any], bytes]:
    try:
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
            fail("protected-build record must be a regular nonsymlink file")
        if metadata.st_size <= 0 or metadata.st_size > MAX_RECORD_BYTES:
            fail("protected-build record size is outside policy")
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"cannot load protected-build record: {error}")
    if raw != canonical_bytes(value):
        fail("protected-build record is not canonical ASCII JSON")
    return exact(value, TOP_KEYS, "protected-build record"), raw


def validate(record: dict[str, Any]) -> None:
    if (
        record["format"] != FORMAT
        or record["authority"] != AUTHORITY
        or record["nonclaim"] != NONCLAIM
        or record["milestone"] != "M1"
        or record["target"] != TARGET
        or record["established_claims"] != ESTABLISHED
        or record["excluded_claims"] != EXCLUDED
        or record["release_entrypoint"]
        != ["cargo-fe2o3", "authority", "release", "build", "--locked"]
    ):
        fail("protected-build scope or nonclaim drifted")

    artifact = exact(record["artifact"], {"path", "sha256", "size_bytes"}, "artifact")
    if (
        artifact["sha256"] != ARTIFACT_SHA256
        or artifact["size_bytes"] != 14_192
        or artifact["path"] != f".fe2o3-link-artifact-v1-{ARTIFACT_SHA256}.bin"
    ):
        fail("finalized HSACO identity drifted")

    source = exact(
        record["source"],
        {"commit", "device_files", "device_provider_commit", "device_provider_tree", "tree"},
        "source",
    )
    if (
        source["commit"] != SOURCE_COMMIT
        or source["tree"] != SOURCE_TREE
        or source["device_provider_commit"] != PROVIDER_COMMIT
        or source["device_provider_tree"] != PROVIDER_TREE
    ):
        fail("Ferric or device-provider source identity drifted")
    expected_files = [
        ("device/qwen3-swiglu-v1/Cargo.lock", "82d8c4322af5f7d16ef9a6fb5309f99c8266a2d834373393dd8e8ad22f21330d", 21_905),
        ("device/qwen3-swiglu-v1/Cargo.toml", "13ee597339bcfc636af9e493eb98ed88650bd99c71f8cfe7f7551ca24f4597d1", 819),
        ("device/qwen3-swiglu-v1/src/lib.rs", "8fbbd8464e2f6b66ca43f284d1f894eba88be1eb84913bcd00360a6b7a20239f", 7_961),
    ]
    if not isinstance(source["device_files"], list):
        fail("Ferric device source file roster is not a list")
    for item in source["device_files"]:
        exact(item, {"path", "sha256", "size_bytes"}, "Ferric device source file")
    if [
        (item["path"], item["sha256"], item["size_bytes"])
        for item in source["device_files"]
    ] != expected_files:
        fail("Ferric device source file roster drifted")

    compiler = exact(
        record["compiler"],
        {"cargo_fe2o3_sha256", "closure", "codegen_backend_sha256", "commit", "rustc_wrapper_sha256", "tree"},
        "compiler",
    )
    if (
        compiler["commit"] != COMPILER_COMMIT
        or compiler["tree"] != COMPILER_TREE
        or compiler["cargo_fe2o3_sha256"] != CARGO_FE2O3_SHA256
        or compiler["rustc_wrapper_sha256"] != RUSTC_WRAPPER_SHA256
        or compiler["codegen_backend_sha256"] != BACKEND_SHA256
    ):
        fail("compiler source or image identity drifted")
    closure = exact(
        compiler["closure"],
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
    for key, value in closure.items():
        if key.endswith("sha256"):
            require_sha(value, f"compiler closure {key}")
    if closure != COMPILER_CLOSURE:
        fail("compiler closure identity drifted")

    recipe = exact(
        record["production_recipe"],
        {"candidate_output_max_bytes", "limits", "link_options", "sha256", "unit", "worker"},
        "production recipe",
    )
    if (
        recipe["sha256"] != CONFIG_SHA256
        or recipe["candidate_output_max_bytes"] != 4_194_304
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
        fail("production recipe identity drifted")
    unit = exact(
        recipe["unit"],
        {"crate_name", "source", "working_directory_relative"},
        "production recipe unit",
    )
    worker = exact(
        recipe["worker"],
        {"byte_len", "llvm_build_identity", "sha256", "worker_build_identity"},
        "production recipe worker",
    )
    if (
        unit.get("crate_name") != "ferric_qwen3_swiglu_device_v1"
        or unit.get("source") != "src/lib.rs"
        or unit.get("working_directory_relative") != "device/qwen3-swiglu-v1"
        or worker.get("sha256") != "81638afe105f006b86404fc7b34ea7bbd1051618cf59a9a16a46fd6e7a0af335"
        or worker.get("byte_len") != 107_389_072
        or worker.get("llvm_build_identity") != "7.2.4"
        or worker.get("worker_build_identity")
        != "fe2o3-worker-v1-sha256-cd00c8e74d3ba6228a9f04640eee2c8930607d4b51eb64627625c92ac028b186"
    ):
        fail("production recipe unit or worker drifted")

    custody = record["custody_records"]
    if not isinstance(custody, list) or len(custody) != 10:
        fail("custody record roster is incomplete")
    by_kind = {item.get("kind"): item for item in custody if isinstance(item, dict)}
    if len(by_kind) != 10:
        fail("custody record kinds are not unique")
    for item in custody:
        exact(item, {"kind", "path", "sha256", "size_bytes"}, "custody record")
        require_sha(item["sha256"], "custody record digest")
    expected_custody = [
        (".codegen-generation-v1", ".codegen-generation-v1", "cb68ae6e82e63d97a3cdc3b64ee2694763fce79dfdd8cd288edbfbc9c64447da", 116),
        (".fe2o3-artifacts.lock", ".fe2o3-artifacts.lock", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", 0),
        (".fe2o3-attempts-v1", ".fe2o3-attempts-v1", "0e6d519a3dc5842bb9515f16262701071b9c7011a7f032268fe0a85988bdef52", 1_182),
        ("consumed", ".fe2o3-compiler-module-handoff-v3-fcee247fc0617e5ac5b0786457f5a39712e61d907b9a28b67c583b9247d0962a/attempt-a2ff973015ea54e95c769c8d6313e2dccb8a4fc4cbc9c037a5958cf81c56f867/consumed", "63bde890dd3bb32033740d3f68cd80891e877cee6fab7a64dc96da2a04381ea4", 323),
        ("artifact", f".fe2o3-link-artifact-v1-{ARTIFACT_SHA256}.bin", ARTIFACT_SHA256, 14_192),
        ("publication", f".fe2o3-link-publication-v1-{PUBLICATION_FILE_ID}.record", PUBLICATION_RECORD_SHA256, 581),
        (".fe2o3-owned-v1", ".fe2o3-owned-v1", "e2bb78e46d846cb86c1a9335a2b3f25ee778106af7b17c98594d4709c1d0bcc0", 31),
        ("claim", f".fe2o3-worker-v3-load-readiness-v1-{READINESS_KEY}.claim", CLAIM_SHA256, 1_219),
        ("envelope", f".fe2o3-worker-v3-load-readiness-v1-{READINESS_KEY}.envelope", ENVELOPE_SHA256, 1_100_878),
        ("receipt", f".fe2o3-worker-v3-load-readiness-v1-{READINESS_KEY}.receipt", RECEIPT_SHA256, 356),
    ]
    if [
        (item["kind"], item["path"], item["sha256"], item["size_bytes"])
        for item in custody
    ] != expected_custody:
        fail("exact custody record roster drifted")

    inspection = exact(
        record["inspection"],
        {"authority", "format", "kernel", "metadata_version", "target", "transcript_sha256"},
        "inspection",
    )
    kernel = inspection["kernel"]
    if (
        inspection.get("authority") != "descriptive-only"
        or inspection.get("format") != "hsaco-v6"
        or inspection.get("metadata_version") != "1.2"
        or inspection.get("target") != TARGET
        or inspection.get("transcript_sha256") != "cbf4a5253e996a7d8702e95fa52fd6c19afebaf08a1a9790c910f64d6a4d8e71"
        or kernel
        != {
            "explicit_argument_count": 6,
            "group_segment_size_bytes": 0,
            "hidden_argument_count": 13,
            "kernarg_alignment_bytes": 8,
            "kernarg_size_bytes": 304,
            "name": "qwen3_swiglu_bf16_f32_v1",
            "private_segment_size_bytes": 0,
            "sgpr_count": 84,
            "symbol": "qwen3_swiglu_bf16_f32_v1.kd",
            "vgpr_count": 11,
            "wavefront_size": 64,
        }
    ):
        fail("descriptive HSACO inspection drifted")

    publication = exact(
        record["publication"],
        {
            "claim",
            "finalization_identity_sha256",
            "finalized_output_identity_sha256",
            "load_readiness",
            "publication_identity_sha256",
            "worker_v3_binding",
        },
        "publication",
    )
    claim = exact(
        publication["claim"],
        {"backend_receipt_sha256", "sha256", "size_bytes"},
        "published claim",
    )
    readiness = exact(
        publication["load_readiness"],
        {
            "backend_receipt_sha256",
            "claim_sha256",
            "claim_size_bytes",
            "envelope_sha256",
            "envelope_size_bytes",
            "receipt_identity_sha256",
        },
        "load-readiness publication",
    )
    binding = exact(
        publication["worker_v3_binding"],
        {
            "compiler_handoff_sha256",
            "finalization_sha256",
            "finalized_output_sha256",
            "finalized_output_size_bytes",
            "publication_intent_sha256",
            "raw_inspection_sha256",
            "raw_output_sha256",
            "raw_output_size_bytes",
            "source_evidence_sha256",
        },
        "Worker V3 binding",
    )
    if (
        claim.get("sha256") != CLAIM_SHA256
        or claim.get("size_bytes") != 1_219
        or claim.get("backend_receipt_sha256") != BACKEND_RECEIPT_SHA256
        or readiness
        != {
            "backend_receipt_sha256": BACKEND_RECEIPT_SHA256,
            "claim_sha256": CLAIM_SHA256,
            "claim_size_bytes": 1_219,
            "envelope_sha256": ENVELOPE_SHA256,
            "envelope_size_bytes": 1_100_878,
            "receipt_identity_sha256": READINESS_RECEIPT_IDENTITY,
        }
        or binding
        != {
            "compiler_handoff_sha256": "de561a1eb2b66a1b85b05e6bda06c5e545c17d642fd0aa23f0a2458fef532b12",
            "finalization_sha256": "37aa965af2c771fcd4c13f635660d25961509d37d0a0572efdb9ec569f53f896",
            "finalized_output_sha256": ARTIFACT_SHA256,
            "finalized_output_size_bytes": 14_192,
            "publication_intent_sha256": "61db6ef6f80e89dc6ac571f99edc5728edc0a3def3c4ad1d117787d4ef743565",
            "raw_inspection_sha256": "0397e40dc360f47c3b301c3b7aa8a1ce5342f862b7de8c0909c185179d49523c",
            "raw_output_sha256": "af9dc3b58ff454dd78253cabbdd1bc2f114e1add2a16c995befbec5a3d50e2b2",
            "raw_output_size_bytes": 14_192,
            "source_evidence_sha256": "1ce1b7a5c834a14f0334ba75522e9f0aec31ce6761d4516ec36d45c72bfd839f",
        }
        or publication["finalization_identity_sha256"] != FINALIZATION_IDENTITY
        or publication["finalized_output_identity_sha256"] != ARTIFACT_SHA256
        or publication["publication_identity_sha256"] != PUBLICATION_IDENTITY
    ):
        fail("publication claim/readiness/finalized-output cross-link drifted")
    for key in (
        "finalization_identity_sha256",
        "finalized_output_identity_sha256",
        "publication_identity_sha256",
    ):
        require_sha(publication.get(key), f"publication {key}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: validate-protected-worker-v3-build.py RECORD")
    record, raw = load(Path(sys.argv[1]))
    validate(record)
    print(f"PASS: protected Worker V3 build record sha256={hashlib.sha256(raw).hexdigest()}")


if __name__ == "__main__":
    main()
