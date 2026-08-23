#!/usr/bin/env python3
"""Validate one canonical M1 selected-function positive Verus run artifact."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys
from typing import Any, NoReturn


PROTOCOL = "ferric.m1-validator.verus-theorem.v1"
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REGISTRY_FORMAT = "format=FERRIC-M1-POSITIVE-THEOREMS-V1"
RUN_FORMAT = "FERRIC-M1-POSITIVE-RUN-V1"
RESULT_FORMAT = "FERRIC-M1-POSITIVE-RESULT-V1"
THEOREM_FORMAT = "FERRIC-M1-POSITIVE-THEOREM-V1"
COMPILE_FORMAT = "FERRIC-M1-POSITIVE-COMPILE-V1"
VERUS_FORMAT = "FERRIC-M1-POSITIVE-VERUS-V1"
SUMMARY_FORMAT = "FERRIC-M1-POSITIVE-OUTPUT-V1"
COMMON_SHA256 = "b4ee8e7c362f28506a87a4c7620249950c61a3eb34fbddd963961f45a78092c2"
VERUS_COMMIT = "b677dd5a766f25f56e9aa1e32621aa4e53304b47"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
DECIMAL = re.compile(r"0|[1-9][0-9]*\Z")
MAX_CONTROL_BYTES = 1_000_000
MAX_TRANSCRIPT_BYTES = 20_000_000
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {"source.fe2o3": "fe2o3", "source.ferric": "ferric"}
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"

ARTIFACT_KEYS = {"id", "kind", "path", "sha256", "size_bytes"}
BINDING_KEYS = {
    "artifact_id",
    "binding_sha256",
    "evidence_kind",
    "id",
    "obligation_class",
    "obligation_id",
    "path_id",
    "profile_id",
    "source_identity_id",
    "statement_sha256",
    "tcb_ids",
}
SOURCE_KEYS = {
    "base_commit",
    "commit",
    "id",
    "repository",
    "source_closure_artifact_id",
    "source_closure_sha256",
    "tree",
}
TCB_KEYS = {"artifact_id", "id", "identity_sha256", "kind"}
PATH_KEYS = {"availability", "id", "path", "repository", "source_identity_id"}

RUN_KEYS = (
    "FORMAT",
    "FERRIC_COMMIT",
    "FERRIC_TREE",
    "FERRIC_SOURCE_CLOSURE_SHA256",
    "VERUS_VERSION",
    "VERUS_SHA256",
    "VERUS_CLOSURE_MANIFEST_SHA256",
    "VERUS_CLOSURE_SHA256",
    "VERIFIED_MODULES_SHA256",
    "REGISTRY_SHA256",
    "RUNNER_SHA256",
    "AUTHORITY",
    "NONCLAIM",
)
RESULT_KEYS = (
    "FORMAT",
    "THEOREM",
    "RUN_IDENTITY_SHA256",
    "ACTIVE_FOUNDATIONS_SHA256",
    "SELECTED_FOUNDATIONS_SHA256",
    "VERUS_CLOSURE_TRANSCRIPT_SHA256",
    "THEOREM_RECORD",
    "THEOREM_RECORD_SHA256",
    "THEOREM_RECORD_SIZE",
    "COMPILE_TRANSCRIPT",
    "COMPILE_TRANSCRIPT_SHA256",
    "COMPILE_TRANSCRIPT_SIZE",
    "COMPILE_EXIT_STATUS",
    "VERUS_SUMMARY",
    "VERUS_SUMMARY_SHA256",
    "VERUS_SUMMARY_SIZE",
    "VERUS_TRANSCRIPT",
    "VERUS_TRANSCRIPT_SHA256",
    "VERUS_TRANSCRIPT_SIZE",
    "VERUS_EXIT_STATUS",
    "RESULT",
)
THEOREM_KEYS = (
    "FORMAT",
    "THEOREM",
    "FOUNDATION",
    "OPEN_ASSURANCE_PROPERTY",
    "OPEN_PATH_OBLIGATION",
    "VERUS_PACKAGE",
    "VERUS_SOURCE",
    "VERUS_MODULE",
    "VERUS_FUNCTION",
    "COMPILER_PATH",
    "VERIFIED_MODULES_SHA256",
    "SOURCE_SHA256",
    "FUNCTION_SOURCE_IDENTITY_SHA256",
    "CARGO_CHECK",
    "VERUS_RESULT",
)
SUMMARY_KEYS = (
    "FORMAT",
    "COMPILER_PATH",
    "TRANSCRIPT_SHA256",
    "VERIFIED_COUNT",
    "DETAILS_COUNT",
    "IS_VERIFYING_ENTIRE_CRATE",
    "ENCOUNTERED_ERROR",
    "ENCOUNTERED_VIR_ERROR",
    "ERRORS",
    "RESULT",
)
ROOT_KEYS = {"func-details", "verification-results", "verus"}
VERIFICATION_KEYS = {
    "encountered-error",
    "encountered-vir-error",
    "errors",
    "is-verifying-entire-crate",
    "verified",
}
VERUS_KEYS = {"commit", "platform", "profile", "toolchain", "version"}
PLATFORM_KEYS = {"arch", "os"}
DETAIL_KEYS = {"failed_proof_notes", "obligation_proof_notes"}
EXPECTED_ROWS = (
    (
        "batching-publish-once",
        "continuous-batching",
        "scheduler_refined",
        "batching-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "batching_publish_once_theorem",
    ),
    (
        "batching-request-routing",
        "continuous-batching",
        "scheduler_refined",
        "scheduler-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "batching_request_routing_theorem",
    ),
    (
        "graph-operator-order",
        "exact-graph-plan",
        "graph_refined",
        "graph-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "graph_operator_order_theorem",
    ),
    (
        "graph-role-step-count",
        "exact-graph-plan",
        "graph_refined",
        "graph-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "graph_role_step_count_theorem",
    ),
    (
        "isolation-other-request-frame",
        "continuous-batching",
        "request_isolated",
        "isolation-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "isolation_other_request_frame_theorem",
    ),
    (
        "kv-release-generation",
        "logical-paged-kv",
        "kv_refined",
        "kv-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "kv_release_generation_theorem",
    ),
    (
        "kv-rollback-retirement",
        "logical-paged-kv",
        "kv_refined",
        "kv-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "kv_rollback_retirement_theorem",
    ),
    (
        "kv-write-prefix",
        "logical-paged-kv",
        "kv_refined",
        "kv-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "kv_write_prefix_theorem",
    ),
    (
        "model-bundle-composition",
        "model-bundle-composition",
        "model_bundle_well_formed",
        "model-bundle-proof",
        "ferric-m1-proof",
        "proofs/m1/model_bundle.rs",
        "model_bundle",
        "model_bundle_well_formed_composition_theorem",
    ),
    (
        "publication-phase-transition",
        "step-plan-publication",
        "graph_refined",
        "graph-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "publication_phase_transition_theorem",
    ),
    (
        "publication-plan-identity",
        "step-plan-publication",
        "graph_refined",
        "graph-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "publication_plan_identity_theorem",
    ),
    (
        "speculative-accepted-count-binding",
        "speculative-step-composition",
        "rollback_refined",
        "speculation-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "speculative_accepted_count_binding_theorem",
    ),
    (
        "speculative-atomic-failure-frame",
        "speculative-step-composition",
        "request_isolated",
        "isolation-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "speculative_atomic_failure_frame_theorem",
    ),
)
EXPECTED_PACKAGES = frozenset(row[4] for row in EXPECTED_ROWS)


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    try:
        return digest_bytes(path.read_bytes())
    except OSError as error:
        fail(f"cannot hash {path}: {error}")


def load_common(repo: Path) -> Any:
    path = repo / "proofs/m1/evidence/validate-negative-mutation.py"
    if digest_file(path) != COMMON_SHA256:
        fail("trusted M1 validator common source identity drifted")
    spec = importlib.util.spec_from_file_location("ferric_m1_validator_common", path)
    if spec is None or spec.loader is None:
        fail("cannot load trusted M1 validator common source")
    sys.dont_write_bytecode = True
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def require_name(value: Any, description: str) -> str:
    if not isinstance(value, str) or SAFE_NAME.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_sha256(value: Any, description: str) -> str:
    if not isinstance(value, str) or SHA256.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_decimal(value: str, description: str) -> int:
    if DECIMAL.fullmatch(value) is None:
        fail(f"malformed {description}")
    return int(value)


def canonical_digest(value: dict[str, Any]) -> str:
    payload = json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    return digest_bytes(payload.encode("ascii"))


def exact_object(value: Any, keys: set[str], description: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        fail(f"{description} fields drifted")
    return value


def validate_context(
    repo: Path, common: Any, context: dict[str, Any]
) -> tuple[Path, dict[str, Any], dict[str, Any], dict[str, Any]]:
    if context["format"] != INDEX_FORMAT:
        fail("Verus-theorem context index format drifted")
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = common.load_canonical_json(requirements_path, "M1 requirements")
    if context["requirements_sha256"] != digest_file(requirements_path):
        fail("Verus-theorem context requirements identity drifted")

    artifact = common.exact_keys(context["artifact"], ARTIFACT_KEYS, "artifact context")
    if artifact["kind"] != "TheoremTranscript":
        fail("Verus-theorem artifact kind drifted")
    require_name(artifact["id"], "artifact id")
    artifact_digest = require_sha256(artifact["sha256"], "artifact SHA-256")
    if not isinstance(artifact["size_bytes"], int) or artifact["size_bytes"] <= 0:
        fail("Verus-theorem artifact size is invalid")
    if not isinstance(context["artifact_absolute_path"], str):
        fail("Verus-theorem artifact absolute path is invalid")
    artifact_path = Path(context["artifact_absolute_path"])
    if not artifact_path.is_absolute():
        fail("Verus-theorem artifact path must be absolute")
    if (
        not isinstance(artifact["path"], str)
        or not artifact["path"]
        or Path(artifact["path"]).is_absolute()
        or ".." in Path(artifact["path"]).parts
        or Path(artifact["path"]).name != artifact_path.name
    ):
        fail("Verus-theorem artifact relative path escaped or drifted")
    artifact_bytes = common.read_bounded(
        artifact_path, MAX_CONTROL_BYTES, "Verus-theorem result artifact"
    )
    if (
        len(artifact_bytes) != artifact["size_bytes"]
        or digest_bytes(artifact_bytes) != artifact_digest
    ):
        fail("Verus-theorem artifact bytes do not match their context identity")

    binding = common.exact_keys(context["binding"], BINDING_KEYS, "binding context")
    for key in (
        "id",
        "artifact_id",
        "obligation_id",
        "path_id",
        "profile_id",
        "source_identity_id",
    ):
        require_name(binding[key], f"binding {key}")
    require_sha256(binding["binding_sha256"], "binding SHA-256")
    require_sha256(binding["statement_sha256"], "binding statement SHA-256")
    if not isinstance(binding["tcb_ids"], list):
        fail("Verus-theorem binding TCB roster drifted")
    if (
        context["subject"] != f"binding:{binding['id']}"
        or binding["artifact_id"] != artifact["id"]
        or binding["evidence_kind"] != "verus-theorem"
        or binding["obligation_class"] != "Assurance"
        or binding["source_identity_id"] != "source.ferric"
        or tuple(binding["tcb_ids"]) != TCB_IDS
    ):
        fail("Verus-theorem binding context drifted")
    payload = {key: value for key, value in binding.items() if key != "binding_sha256"}
    if binding["binding_sha256"] != canonical_digest(payload):
        fail("Verus-theorem binding identity mismatch")

    properties = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    property_record = properties.get(binding["obligation_id"])
    if property_record is None or property_record["obligation_state"] != "Open":
        fail("Verus-theorem binding does not name an Open assurance property")
    if binding["profile_id"] not in property_record["evidence_profiles"]:
        fail("Verus-theorem binding profile is not assigned to its property")
    profiles = {
        record["id"]: record["kinds"] for record in requirements["evidence_profiles"]
    }
    if "verus-theorem" not in profiles.get(binding["profile_id"], []):
        fail("Verus-theorem binding profile does not require theorem evidence")
    if binding["statement_sha256"] != digest_bytes(
        property_record["boundary"].encode("utf-8")
    ):
        fail("Verus-theorem binding statement identity drifted")

    resolution = common.exact_keys(
        context["path_resolution"], PATH_KEYS, "path context"
    )
    paths = {record["id"]: record for record in requirements["path_obligations"]}
    expected_path = paths.get(binding["path_id"])
    if (
        expected_path is None
        or resolution["id"] != binding["path_id"]
        or resolution["availability"] != expected_path["availability"]
        or resolution["path"] != expected_path["path"]
        or resolution["repository"] != "ferric"
        or resolution["source_identity_id"] != "source.ferric"
    ):
        fail("Verus-theorem path context drifted")

    sources = context["sources"]
    if not isinstance(sources, list) or len(sources) != 2:
        fail("Verus-theorem source context roster drifted")
    source_map: dict[str, dict[str, Any]] = {}
    for source in sources:
        record = common.exact_keys(source, SOURCE_KEYS, "source context")
        identifier = record["id"]
        if not isinstance(identifier, str) or identifier not in SOURCE_REPOSITORIES:
            fail("unknown Verus-theorem source context")
        if identifier in source_map:
            fail("duplicate Verus-theorem source context")
        common.require_git_id(record["commit"], f"{identifier} commit")
        common.require_git_id(record["tree"], f"{identifier} tree")
        require_sha256(record["source_closure_sha256"], f"{identifier} source closure")
        require_name(
            record["source_closure_artifact_id"], f"{identifier} closure artifact"
        )
        if record["repository"] != SOURCE_REPOSITORIES[identifier]:
            fail(f"Verus-theorem source repository drifted: {identifier}")
        source_map[identifier] = record
    if tuple(source_map) != SOURCE_IDS:
        fail("Verus-theorem source context order drifted")
    ferric_source = source_map["source.ferric"]
    if ferric_source["base_commit"] != FERRIC_BASE_COMMIT:
        fail("Verus-theorem Ferric source base identity drifted")
    if (
        source_map["source.fe2o3"]["base_commit"]
        != requirements["m1_upstream_base_commit"]
    ):
        fail("Verus-theorem fe2o3 source base identity drifted")
    actual_commit, actual_tree, actual_closure = common.qualified_source_identity(repo)
    if (
        ferric_source["commit"] != actual_commit
        or ferric_source["tree"] != actual_tree
        or ferric_source["source_closure_sha256"] != actual_closure
    ):
        fail("Verus-theorem Ferric context is not the qualified source identity")

    tcb = context["tcb"]
    if not isinstance(tcb, list) or len(tcb) != len(TCB_IDS):
        fail("Verus-theorem TCB roster drifted")
    for record, expected_id in zip(tcb, TCB_IDS, strict=True):
        common.exact_keys(record, TCB_KEYS, "TCB context")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("Verus-theorem TCB order or identity drifted")
        require_name(record["artifact_id"], f"{expected_id} artifact")
        require_sha256(record["identity_sha256"], f"{expected_id} identity")
    return artifact_path, requirements, binding, ferric_source


def registry_rows(repo: Path, common: Any) -> tuple[bytes, list[tuple[str, ...]]]:
    path = repo / "proofs/m1/theorem/REQUIRED_FOUNDATIONS"
    raw = common.read_bounded(path, MAX_CONTROL_BYTES, "M1 theorem registry")
    if not raw.endswith(b"\n") or raw.endswith(b"\n\n"):
        fail("M1 theorem registry must have one trailing newline")
    try:
        lines = raw.decode("ascii").splitlines()
    except UnicodeDecodeError as error:
        fail(f"M1 theorem registry is not ASCII: {error}")
    if not lines or lines[0] != REGISTRY_FORMAT:
        fail("M1 theorem registry format drifted")
    rows: list[tuple[str, ...]] = []
    for line in lines[1:]:
        if not line.startswith("theorem="):
            fail("M1 theorem registry contains a malformed row")
        fields = tuple(line.removeprefix("theorem=").split("|"))
        if len(fields) != 8:
            fail("M1 theorem registry row width drifted")
        rows.append(fields)
    if tuple(rows) != EXPECTED_ROWS:
        fail("M1 theorem registry row identity or order drifted")
    active = "".join("|".join(row) + "\n" for row in rows).encode("ascii")
    return active, rows


def exact_run_files(
    run_dir: Path, selected: list[tuple[str, ...]], common: Any
) -> None:
    expected = {
        "RUN_IDENTITY",
        "active-foundations",
        "selected-foundations",
        "verus-closure.transcript",
    }
    expected.update(f"{row[4]}.compile.transcript" for row in selected)
    for row in selected:
        name = row[0]
        expected.update(
            {
                f"{name}.result",
                f"{name}.theorem",
                f"{name}.verus.summary",
                f"{name}.verus.transcript",
            }
        )
    try:
        entries = list(run_dir.iterdir())
    except OSError as error:
        fail(f"cannot enumerate Verus-theorem run directory: {error}")
    if {entry.name for entry in entries} != expected or len(entries) != len(expected):
        fail("Verus-theorem run file roster is incomplete or contains extras")
    for entry in entries:
        common.regular_file(entry, f"Verus-theorem run member {entry.name}")


def validate_run_identity(
    repo: Path,
    run_dir: Path,
    ferric_source: dict[str, Any],
    common: Any,
) -> tuple[dict[str, str], bytes]:
    path = run_dir / "RUN_IDENTITY"
    values = common.parse_kv(path, RUN_KEYS, "Verus-theorem run identity")
    if values["FORMAT"] != RUN_FORMAT:
        fail("Verus-theorem run format drifted")
    if (
        values["FERRIC_COMMIT"] != ferric_source["commit"]
        or values["FERRIC_TREE"] != ferric_source["tree"]
        or values["FERRIC_SOURCE_CLOSURE_SHA256"]
        != ferric_source["source_closure_sha256"]
    ):
        fail("Verus-theorem run source commit, tree, or closure drifted")
    for key in RUN_KEYS[3:11]:
        if key != "VERUS_VERSION":
            require_sha256(values[key], f"run {key}")
    version = (repo / "proofs/verus/VERUS_VERSION").read_text(encoding="ascii")
    verus_sha = (repo / "proofs/verus/VERUS_SHA256").read_text(encoding="ascii")
    if (
        version != values["VERUS_VERSION"] + "\n"
        or verus_sha != values["VERUS_SHA256"] + "\n"
    ):
        fail("Verus-theorem run Verus release identity drifted")
    manifest = common.read_bounded(
        repo / "proofs/verus/VERUS_CLOSURE_MANIFEST",
        MAX_CONTROL_BYTES,
        "Verus closure manifest",
    )
    if values["VERUS_CLOSURE_MANIFEST_SHA256"] != digest_bytes(manifest):
        fail("Verus-theorem closure-manifest identity drifted")
    closure_values = [
        line.removeprefix(b"closure-sha256=").decode("ascii")
        for line in manifest.splitlines()
        if line.startswith(b"closure-sha256=")
    ]
    if closure_values != [values["VERUS_CLOSURE_SHA256"]]:
        fail("Verus-theorem compiler closure identity drifted")
    identities = {
        "VERIFIED_MODULES_SHA256": repo / "proofs/VERIFIED_MODULES",
        "REGISTRY_SHA256": repo / "proofs/m1/theorem/REQUIRED_FOUNDATIONS",
        "RUNNER_SHA256": repo / "proofs/m1/theorem/run-same-source.sh",
    }
    if any(values[key] != digest_file(file) for key, file in identities.items()):
        fail("Verus-theorem coverage, registry, or runner identity drifted")
    if (
        values["AUTHORITY"] != "direct-verus-foundation-success-only"
        or values["NONCLAIM"] != "no-m1-property-path-or-roadmap-closure"
    ):
        fail("Verus-theorem authority boundary drifted")

    closure = common.read_bounded(
        run_dir / "verus-closure.transcript",
        MAX_CONTROL_BYTES,
        "Verus closure transcript",
    )
    fields = dict(
        line.decode("ascii").split("=", 1)
        for line in manifest.splitlines()
        if b"=" in line
    )
    expected = (
        f"PASS: pinned Verus release closure matched "
        f"({fields['file-count']} files, {fields['total-bytes']} bytes)\n"
    ).encode("ascii")
    if closure != expected:
        fail("Verus-theorem compiler closure transcript drifted")
    return values, closure


def companion(
    run_dir: Path,
    expected_name: str,
    recorded_name: str,
    recorded_size: str,
    recorded_digest: str,
    description: str,
    limit: int,
    common: Any,
) -> tuple[Path, bytes]:
    if recorded_name != expected_name or Path(recorded_name).name != recorded_name:
        fail(f"{description} path escaped or drifted")
    size = require_decimal(recorded_size, f"{description} size")
    if size <= 0:
        fail(f"{description} size must be positive")
    require_sha256(recorded_digest, f"{description} SHA-256")
    path = run_dir / recorded_name
    raw = common.read_bounded(path, limit, description)
    if len(raw) != size or digest_bytes(raw) != recorded_digest:
        fail(f"{description} byte identity mismatch")
    return path, raw


def validate_compile(raw: bytes, package: str, common: Any) -> None:
    body = common.transcript_parts(
        raw,
        (
            f"FORMAT={COMPILE_FORMAT}",
            f"CARGO_PACKAGE={package}",
            "COMMAND=cargo-check-locked-all-targets",
        ),
        "Verus-theorem compile transcript",
    )
    if len(body.encode("utf-8")) < 150:
        fail("Verus-theorem compile transcript is implausibly short")
    if re.search(rf"\b(?:Checking|Compiling) {re.escape(package)} v", body) is None:
        fail("Verus-theorem compile transcript did not compile its package")
    if "Finished `dev` profile" not in body:
        fail("Verus-theorem compile transcript has no successful terminal result")
    prohibited = ("error:", "could not compile", "timed out", "timeout", "FAIL:")
    if any(value.lower() in body.lower() for value in prohibited):
        fail("Verus-theorem ordinary compilation was not clean")


def structured_objects(transcript: str, common: Any) -> list[dict[str, Any]]:
    decoder = json.JSONDecoder(object_pairs_hook=common.reject_duplicate_key)
    objects: list[dict[str, Any]] = []
    cursor = 0
    while cursor < len(transcript):
        opening = transcript.find("{", cursor)
        if opening < 0:
            break
        try:
            value, end = decoder.raw_decode(transcript, opening)
        except json.JSONDecodeError:
            cursor = opening + 1
            continue
        if isinstance(value, dict) and "verification-results" in value:
            objects.append(value)
        cursor = end
    return objects


def rust_toolchain(repo: Path) -> str:
    source = (repo / "rust-toolchain.toml").read_text(encoding="ascii")
    matches = re.findall(r'^channel = "([^"]+)"$', source, re.MULTILINE)
    if len(matches) != 1:
        fail("Verus-theorem Rust toolchain identity drifted")
    return matches[0] + "-x86_64-unknown-linux-gnu"


def validate_verus(
    repo: Path, raw: bytes, row: tuple[str, ...], common: Any
) -> tuple[int, int]:
    name, _, _, _, package, _, module, function = row
    body = common.transcript_parts(
        raw,
        (
            f"FORMAT={VERUS_FORMAT}",
            f"THEOREM={name}",
            f"VERUS_PACKAGE={package}",
            f"VERUS_MODULE={module}",
            f"VERUS_FUNCTION={function}",
            "COMMAND=cargo-verus-build-lib-locked-release-no-cheating-output-json-exact-function",
        ),
        f"{name} Verus transcript",
    )
    if len(body.encode("utf-8")) < 500:
        fail(f"Verus-theorem transcript is implausibly short: {name}")
    if body.count(f"note: verifying module {module} (selected functions)") != 1:
        fail(f"Verus-theorem selected-module diagnostic drifted: {name}")
    if re.search(rf"^\s+Compiling {re.escape(package)} v", body, re.MULTILINE) is None:
        fail(f"Verus-theorem did not compile its selected package: {name}")
    if "Finished `release` profile" not in body:
        fail(f"Verus-theorem transcript has no successful terminal result: {name}")
    prohibited = (
        "error:",
        "could not compile",
        "timed out",
        "timeout",
        "could not find module",
        "could not find function",
        "available modules are",
        "assume/admit not allowed",
        "admit not allowed",
        "no space left",
        "permission denied",
        "failed to get",
        "connection refused",
        "panicked at",
        "unknown option",
    )
    if any(value in body.lower() for value in prohibited):
        fail(
            f"Verus-theorem transcript contains a proof or infrastructure error: {name}"
        )

    objects = structured_objects(body, common)
    if len(objects) != 1:
        fail(
            f"Verus-theorem transcript has {len(objects)} structured root results: {name}"
        )
    root = exact_object(objects[0], ROOT_KEYS, "structured Verus root")
    result = exact_object(
        root["verification-results"],
        VERIFICATION_KEYS,
        "structured verification result",
    )
    details = root["func-details"]
    verus = exact_object(root["verus"], VERUS_KEYS, "structured Verus identity")
    platform = exact_object(verus["platform"], PLATFORM_KEYS, "Verus platform")
    verified = result["verified"]
    if (
        result["is-verifying-entire-crate"] is not False
        or result["encountered-error"] is not False
        or result["encountered-vir-error"] is not False
        or result["errors"] != 0
        or not isinstance(result["errors"], int)
        or isinstance(result["errors"], bool)
        or not isinstance(verified, int)
        or isinstance(verified, bool)
        or verified != 1
    ):
        fail(f"Verus-theorem structured result is not an exact success: {name}")
    compiler_path = f"{package.replace('-', '_')}::{module}::{function}"
    if not isinstance(details, dict) or compiler_path not in details:
        fail(f"Verus-theorem selected function is absent from func-details: {name}")
    for detail_path, detail_value in details.items():
        if not isinstance(detail_path, str) or not detail_path:
            fail(f"Verus-theorem function detail path is invalid: {name}")
        detail = exact_object(detail_value, DETAIL_KEYS, "structured function detail")
        if detail["failed_proof_notes"] != [] or detail["obligation_proof_notes"] != []:
            fail(f"Verus-theorem function detail has unresolved proof notes: {name}")
    expected_version = (
        (repo / "proofs/verus/VERUS_VERSION").read_text(encoding="ascii").strip()
    )
    if (
        verus["commit"] != VERUS_COMMIT
        or verus["version"] != expected_version
        or verus["profile"] != "release"
        or verus["toolchain"] != rust_toolchain(repo)
        or platform != {"arch": "x86_64", "os": "linux"}
    ):
        fail(f"Verus-theorem structured tool identity drifted: {name}")
    return verified, len(details)


def validate_theorem_record(
    repo: Path,
    path: Path,
    row: tuple[str, ...],
    coverage_sha: str,
    common: Any,
) -> None:
    values = common.parse_kv(path, THEOREM_KEYS, f"{row[0]} theorem record")
    name, foundation, property_name, path_id, package, source, module, function = row
    compiler_path = f"{package.replace('-', '_')}::{module}::{function}"
    source_sha = digest_file(repo / source)
    identity = digest_bytes(
        f"FERRIC-M1-THEOREM-SOURCE-IDENTITY-V1|{source_sha}|{compiler_path}\n".encode(
            "ascii"
        )
    )
    expected = {
        "FORMAT": THEOREM_FORMAT,
        "THEOREM": name,
        "FOUNDATION": foundation,
        "OPEN_ASSURANCE_PROPERTY": property_name,
        "OPEN_PATH_OBLIGATION": path_id,
        "VERUS_PACKAGE": package,
        "VERUS_SOURCE": source,
        "VERUS_MODULE": module,
        "VERUS_FUNCTION": function,
        "COMPILER_PATH": compiler_path,
        "VERIFIED_MODULES_SHA256": coverage_sha,
        "SOURCE_SHA256": source_sha,
        "FUNCTION_SOURCE_IDENTITY_SHA256": identity,
        "CARGO_CHECK": "passed",
        "VERUS_RESULT": "proved",
    }
    if values != expected:
        fail(f"Verus-theorem source/function binding drifted: {name}")
    manifest = (repo / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8")
    module_record = f"module={package}|{source}|{package.replace('-', '_')}::{module}\n"
    function_record = f"verified={package}|{source}|{compiler_path}\n"
    if manifest.count(module_record) != 1 or manifest.count(function_record) != 1:
        fail(f"Verus-theorem compiler-rooted coverage binding drifted: {name}")


def validate_result(
    repo: Path,
    run_dir: Path,
    row: tuple[str, ...],
    run_identity: bytes,
    active: bytes,
    selected: bytes,
    closure: bytes,
    compile_raw: bytes,
    common: Any,
) -> None:
    name, _, _, _, package, _, _, _ = row
    result = common.parse_kv(
        run_dir / f"{name}.result", RESULT_KEYS, f"{name} result record"
    )
    expected = {
        "FORMAT": RESULT_FORMAT,
        "THEOREM": name,
        "RUN_IDENTITY_SHA256": digest_bytes(run_identity),
        "ACTIVE_FOUNDATIONS_SHA256": digest_bytes(active),
        "SELECTED_FOUNDATIONS_SHA256": digest_bytes(selected),
        "VERUS_CLOSURE_TRANSCRIPT_SHA256": digest_bytes(closure),
        "COMPILE_EXIT_STATUS": "0",
        "VERUS_EXIT_STATUS": "0",
        "RESULT": "proved",
    }
    for key, value in expected.items():
        if result[key] != value:
            fail(f"Verus-theorem result binding drifted: {name}/{key}")
    theorem_path, _ = companion(
        run_dir,
        f"{name}.theorem",
        result["THEOREM_RECORD"],
        result["THEOREM_RECORD_SIZE"],
        result["THEOREM_RECORD_SHA256"],
        f"{name} theorem record",
        MAX_CONTROL_BYTES,
        common,
    )
    _, recorded_compile = companion(
        run_dir,
        f"{package}.compile.transcript",
        result["COMPILE_TRANSCRIPT"],
        result["COMPILE_TRANSCRIPT_SIZE"],
        result["COMPILE_TRANSCRIPT_SHA256"],
        f"{name} compile transcript",
        MAX_TRANSCRIPT_BYTES,
        common,
    )
    summary_path, _ = companion(
        run_dir,
        f"{name}.verus.summary",
        result["VERUS_SUMMARY"],
        result["VERUS_SUMMARY_SIZE"],
        result["VERUS_SUMMARY_SHA256"],
        f"{name} Verus summary",
        MAX_CONTROL_BYTES,
        common,
    )
    _, verus_raw = companion(
        run_dir,
        f"{name}.verus.transcript",
        result["VERUS_TRANSCRIPT"],
        result["VERUS_TRANSCRIPT_SIZE"],
        result["VERUS_TRANSCRIPT_SHA256"],
        f"{name} Verus transcript",
        MAX_TRANSCRIPT_BYTES,
        common,
    )
    if recorded_compile != compile_raw:
        fail(f"Verus-theorem compile transcript substitution: {name}")
    coverage_sha = digest_file(repo / "proofs/VERIFIED_MODULES")
    validate_theorem_record(repo, theorem_path, row, coverage_sha, common)
    verified, details = validate_verus(repo, verus_raw, row, common)
    summary = common.parse_kv(summary_path, SUMMARY_KEYS, f"{name} Verus summary")
    compiler_path = f"{package.replace('-', '_')}::{row[6]}::{row[7]}"
    if summary != {
        "FORMAT": SUMMARY_FORMAT,
        "COMPILER_PATH": compiler_path,
        "TRANSCRIPT_SHA256": digest_bytes(verus_raw),
        "VERIFIED_COUNT": str(verified),
        "DETAILS_COUNT": str(details),
        "IS_VERIFYING_ENTIRE_CRATE": "false",
        "ENCOUNTERED_ERROR": "false",
        "ENCOUNTERED_VIR_ERROR": "false",
        "ERRORS": "0",
        "RESULT": "success",
    }:
        fail(f"Verus-theorem summary does not match structured output-json: {name}")


def validate_run(
    repo: Path,
    artifact_path: Path,
    binding: dict[str, Any],
    ferric_source: dict[str, Any],
    common: Any,
) -> None:
    run_dir = artifact_path.parent
    active_expected, rows = registry_rows(repo, common)
    active = common.read_bounded(
        run_dir / "active-foundations", MAX_CONTROL_BYTES, "active theorem roster"
    )
    if active != active_expected:
        fail("Verus-theorem active registry does not match qualified source")
    selected = common.read_bounded(
        run_dir / "selected-foundations", MAX_CONTROL_BYTES, "selected theorem roster"
    )
    try:
        selected_lines = selected.decode("ascii").splitlines()
    except UnicodeDecodeError as error:
        fail(f"selected theorem roster is not ASCII: {error}")
    if not selected.endswith(b"\n") or selected.endswith(b"\n\n") or not selected_lines:
        fail("selected theorem roster is empty or has trailing data")
    row_by_line = {"|".join(row): row for row in rows}
    if len(selected_lines) != len(set(selected_lines)) or any(
        line not in row_by_line for line in selected_lines
    ):
        fail("selected theorem roster has duplicate or unknown rows")
    selected_names = {row_by_line[line][0] for line in selected_lines}
    expected_lines = ["|".join(row) for row in rows if row[0] in selected_names]
    if selected_lines != expected_lines:
        fail("selected theorem roster is reordered or noncanonical")
    selected_rows = [row_by_line[line] for line in selected_lines]
    binding_rows = [
        row
        for row in rows
        if row[2] == binding["obligation_id"] and row[3] == binding["path_id"]
    ]
    if not binding_rows or selected_rows != binding_rows:
        fail("selected theorem roster is incomplete for its bound property and path")
    exact_run_files(run_dir, selected_rows, common)

    _, closure = validate_run_identity(repo, run_dir, ferric_source, common)
    run_identity = common.read_bounded(
        run_dir / "RUN_IDENTITY", MAX_CONTROL_BYTES, "Verus-theorem run identity"
    )
    packages = {row[4] for row in selected_rows}
    if not packages <= EXPECTED_PACKAGES:
        fail("selected theorem package roster drifted")
    compile_by_package: dict[str, bytes] = {}
    for package in sorted(packages):
        compile_raw = common.read_bounded(
            run_dir / f"{package}.compile.transcript",
            MAX_TRANSCRIPT_BYTES,
            f"{package} Verus-theorem compile transcript",
        )
        validate_compile(compile_raw, package, common)
        compile_by_package[package] = compile_raw
    for row in selected_rows:
        validate_result(
            repo,
            run_dir,
            row,
            run_identity,
            active,
            selected,
            closure,
            compile_by_package[row[4]],
            common,
        )
    primary = artifact_path.name.removesuffix(".result")
    if artifact_path.name != f"{primary}.result" or primary not in {
        row[0] for row in selected_rows
    }:
        fail("Verus-theorem artifact does not name one selected result")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail(f"usage: {sys.argv[0]} {PROTOCOL}")
    repo = Path.cwd().resolve(strict=True)
    common = load_common(repo)
    context, payload = common.load_stdin_context()
    artifact_path, _, binding, ferric_source = validate_context(repo, common, context)
    validate_run(repo, artifact_path, binding, ferric_source, common)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(payload)}"
    )


if __name__ == "__main__":
    main()
