#!/usr/bin/env python3
"""Validate the finite M1 foundation-mutation registry."""

from __future__ import annotations

import json
from pathlib import Path
import re
import stat
import subprocess
import sys
from typing import NoReturn


FORMAT = "format=FERRIC-M1-NEGATIVE-FOUNDATIONS-V1"
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_TARGET = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*\Z")
SAFE_CLAUSE = re.compile(r"[a-z][a-z0-9-]*\Z")
MARKERS = {"assertion", "postcondition"}

# This exact roster is the fail-closed requirement. Registry rows associate an
# Open M1 obligation with an existing direct-Verus foundation; they do not
# claim that the future path obligation has been implemented or closed.
EXPECTED = {
    "artifact-manifest-commitment-digest": (
        "manifest-commitment-authentication", "artifact_authenticated",
        "bundle-auth", "ferric-build", "crates/ferric-build/src/auth.rs",
        "artifact-manifest-commitment-digest.py", "assertion", "auth",
        "validate_manifest_commitment_verified",
        "canonical-manifest-digest-binding",
    ),
    "batching-publish-once": (
        "continuous-batching", "scheduler_refined", "batching-proof",
        "ferric-spec", "crates/ferric-spec/src/continuous_batching.rs",
        "batching-publish-once.py", "postcondition",
        "continuous_batching", "apply_continuous_publish_step",
        "exact-once-completion-publication",
    ),
    "batching-request-routing": (
        "continuous-batching", "scheduler_refined", "scheduler-proof",
        "ferric-spec", "crates/ferric-spec/src/continuous_batching.rs",
        "batching-request-routing.py", "postcondition",
        "continuous_batching", "apply_continuous_batch_step",
        "stale-generation-rejection",
    ),
    "graph-operator-order": (
        "exact-graph-plan", "graph_refined", "graph-proof", "ferric-spec",
        "crates/ferric-spec/src/graph.rs", "graph-operator-order.py",
        "postcondition", "graph", "expected_step",
        "exact-layer-operator-order",
    ),
    "graph-role-step-count": (
        "exact-graph-plan", "graph_refined", "graph-proof", "ferric-spec",
        "crates/ferric-spec/src/graph.rs", "graph-role-step-count.py",
        "postcondition", "graph", "plan_step_count",
        "exact-role-step-count",
    ),
    "isolation-other-request-frame": (
        "continuous-batching", "request_isolated", "isolation-proof",
        "ferric-spec", "crates/ferric-spec/src/continuous_batching.rs",
        "isolation-other-request-frame.py", "postcondition",
        "continuous_batching", "apply_continuous_batch_step",
        "other-request-frame",
    ),
    "kv-release-generation": (
        "logical-paged-kv", "kv_refined", "kv-proof", "ferric-spec",
        "crates/ferric-spec/src/paged_kv_refinement.rs",
        "kv-release-generation.py", "postcondition",
        "paged_kv_refinement", "release_retired_page",
        "released-generation-advance",
    ),
    "kv-rollback-retirement": (
        "logical-paged-kv", "kv_refined", "kv-proof", "ferric-spec",
        "crates/ferric-spec/src/paged_kv_refinement.rs",
        "kv-rollback-retirement.py", "postcondition",
        "paged_kv_refinement", "rollback_physical_token",
        "retired-tail-prefix",
    ),
    "kv-terminal-release-exact-epoch": (
        "terminal-page-lifetime-release", "lifetime_safe", "kv-proof",
        "ferric-spec", "crates/ferric-spec/src/request_isolation.rs",
        "kv-terminal-release-exact-epoch.py", "assertion",
        "request_isolation", "release_isolated_page",
        "exact-quiescent-epoch-match",
    ),
    "kv-write-prefix": (
        "logical-paged-kv", "kv_refined", "kv-proof", "ferric-spec",
        "crates/ferric-spec/src/paged_kv_refinement.rs", "kv-write-prefix.py",
        "postcondition", "paged_kv_refinement",
        "write_physical_token", "initialized-prefix-advance",
    ),
    "model-bundle-record-binding": (
        "model-bundle-composition", "model_bundle_well_formed",
        "model-bundle-proof", "ferric-build",
        "crates/ferric-build/src/auth.rs", "model-bundle-record-binding.py",
        "postcondition", "auth", "admission_records_equal",
        "retained-record-equality",
    ),
    "operator-declared-profile-effect": (
        "k1-k7-operator-composition", "operator_refined",
        "kernel-contract-proof", "ferric-engine",
        "crates/ferric-engine/src/operation_kernel_plan.rs",
        "operator-declared-profile-effect.py", "assertion",
        "operation_kernel_plan", "select_declared_operator_certificate",
        "profile-identity-presence",
    ),
    "publication-phase-transition": (
        "step-plan-publication", "graph_refined", "graph-proof",
        "ferric-spec", "crates/ferric-spec/src/step_plan_publication.rs",
        "publication-phase-transition.py", "assertion",
        "step_plan_publication", "publish_reserved_delta",
        "validated-to-published-transition",
    ),
    "publication-plan-identity": (
        "step-plan-publication", "graph_refined", "graph-proof",
        "ferric-spec", "crates/ferric-spec/src/step_plan_publication.rs",
        "publication-plan-identity.py", "postcondition",
        "step_plan_publication", "validate_step_plan",
        "exact-plan-identity",
    ),
    "sampler-lowest-id-publication": (
        "deterministic-sampler-composition", "sampler_refined",
        "speculation-proof", "ferric-spec",
        "crates/ferric-spec/src/m1_completion.rs",
        "sampler-lowest-id-publication.py", "assertion",
        "m1_completion", "select_lowest_argmax",
        "lowest-token-id-tie-breaking",
    ),
    "speculative-accepted-count-binding": (
        "speculative-step-composition", "rollback_refined", "speculation-proof",
        "ferric-spec", "crates/ferric-spec/src/speculative_step_composition.rs",
        "speculative-accepted-count-binding.py", "assertion",
        "speculative_step_composition", "settle_and_publish_speculative_step",
        "publication-kv-accepted-count",
    ),
    "speculative-atomic-failure-frame": (
        "speculative-step-composition", "request_isolated", "isolation-proof",
        "ferric-spec", "crates/ferric-spec/src/speculative_step_composition.rs",
        "speculative-atomic-failure-frame.py", "postcondition",
        "speculative_step_composition", "settle_and_publish_speculative_step",
        "atomic-preflight-failure-frame",
    ),
    "target-catalog-processor-features": (
        "kernel-catalog-target-conformance", "target_conforming",
        "identity-closure", "ferric-kernels",
        "crates/ferric-kernels/src/validation.rs",
        "target-catalog-processor-features.py", "postcondition",
        "validation", "validate_kernel_catalog_input",
        "exact-processor-target-rejection",
    ),
}


def fail(message: str) -> NoReturn:
    raise SystemExit(f"FAIL: {message}")


def regular_file(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {path}: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"{description} must be a regular file: {path}")


def safe_relative(value: str, description: str) -> Path:
    path = Path(value)
    if not value or path.is_absolute() or ".." in path.parts or "|" in value:
        fail(f"unsafe {description}: {value!r}")
    return path


def validate_open_requirements(repo: Path) -> tuple[dict[str, dict], dict[str, dict]]:
    checker = repo / "proofs/check-m1-requirements.py"
    regular_file(checker, "M1 requirements checker")
    checked = subprocess.run(
        [sys.executable, "-I", str(checker), str(repo)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if checked.returncode != 0:
        fail(f"open M1 requirements check failed: {checked.stdout.strip()}")
    manifest_path = repo / "proofs/M1_REQUIREMENTS.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    properties = {row["name"]: row for row in manifest["assurance_properties"]}
    paths = {row["id"]: row for row in manifest["path_obligations"]}
    return properties, paths


def coverage(repo: Path) -> tuple[set[tuple[str, str, str]], set[tuple[str, str, str]]]:
    path = repo / "proofs/VERIFIED_MODULES"
    regular_file(path, "compiler-rooted proof coverage manifest")
    modules: set[tuple[str, str, str]] = set()
    verified: set[tuple[str, str, str]] = set()
    for line in path.read_text(encoding="utf-8").splitlines()[1:]:
        if line.startswith("module="):
            fields = tuple(line.removeprefix("module=").split("|"))
            if len(fields) != 3 or fields in modules:
                fail("malformed or duplicate compiler module record")
            modules.add(fields)
        elif line.startswith("verified="):
            fields = tuple(line.removeprefix("verified=").split("|"))
            if len(fields) != 3 or fields in verified:
                fail("malformed or duplicate directly verified body record")
            verified.add(fields)
    return modules, verified


def main() -> None:
    if len(sys.argv) != 4:
        fail("usage: check-registry.py REPO REGISTRY ACTIVE_OUTPUT")
    repo = Path(sys.argv[1]).resolve(strict=True)
    registry = Path(sys.argv[2])
    output = Path(sys.argv[3])
    regular_file(registry, "M1 foundation-mutation registry")
    if output.exists() or output.is_symlink():
        fail(f"active M1 registry output already exists: {output}")

    properties, paths = validate_open_requirements(repo)
    modules, verified = coverage(repo)
    lines = registry.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != FORMAT:
        fail("unsupported M1 foundation-mutation registry")
    if len(lines) == 1:
        fail("M1 foundation-mutation registry selected no mutations")

    rows: list[tuple[str, ...]] = []
    names: set[str] = set()
    mutators: set[str] = set()
    clauses: set[tuple[str, str]] = set()
    for line in lines[1:]:
        if not line or not line.startswith("mutation="):
            fail(f"malformed M1 foundation-mutation record: {line}")
        fields = tuple(line.removeprefix("mutation=").split("|"))
        if len(fields) != 11:
            fail(f"malformed M1 foundation-mutation record: {line}")
        (
            name, foundation, property_name, path_id, package, source, mutator,
            marker, module, function, clause,
        ) = fields
        for value, description in (
            (name, "mutation name"), (foundation, "foundation name"),
            (property_name, "assurance property"), (path_id, "path obligation"),
            (package, "package name"), (mutator, "mutator name"),
        ):
            if SAFE_NAME.fullmatch(value) is None:
                fail(f"unsafe M1 {description}: {value!r}")
        if marker not in MARKERS:
            fail(f"unknown M1 proof-failure marker: {marker}")
        if SAFE_TARGET.fullmatch(module) is None or SAFE_TARGET.fullmatch(function) is None:
            fail(f"unsafe M1 compiler target: {module}::{function}")
        if SAFE_CLAUSE.fullmatch(clause) is None:
            fail(f"unsafe M1 contract clause: {clause!r}")
        source_path = safe_relative(source, "foundation source path")
        mutator_path = safe_relative(mutator, "foundation mutator path")
        if name in names:
            fail(f"duplicate M1 foundation mutation: {name}")
        if mutator in mutators:
            fail(f"duplicate M1 foundation mutator: {mutator}")
        if (foundation, clause) in clauses:
            fail(f"duplicate M1 foundation contract clause: {foundation}/{clause}")
        names.add(name)
        mutators.add(mutator)
        clauses.add((foundation, clause))

        expected = EXPECTED.get(name)
        if expected is None:
            fail(f"unknown M1 foundation mutation: {name}")
        if fields[1:] != expected:
            fail(f"M1 foundation mutation binding drifted: {name}")
        property_row = properties.get(property_name)
        if property_row is None or property_row["obligation_state"] != "Open":
            fail(f"M1 mutation property is absent or not Open: {property_name}")
        if path_id not in property_row["path_obligations"]:
            fail(f"M1 mutation path is not assigned to property: {name}")
        path_row = paths.get(path_id)
        if path_row is None or path_row["obligation_state"] != "Open":
            fail(f"M1 mutation path is absent or not Open: {path_id}")

        regular_file(repo / source_path, "M1 foundation source")
        regular_file(
            repo / "proofs/m1/negative/components" / mutator_path,
            "M1 foundation mutator",
        )
        inventory_module = f"{package.replace('-', '_')}::{module}"
        if (package, source, inventory_module) not in modules:
            fail(f"M1 compiler module path is not inventoried: {name}")
        compiler_path = f"{inventory_module}::{function}"
        if (package, source, compiler_path) not in verified:
            fail(f"M1 compiler function path is not directly verified: {name}")
        rows.append(fields)

    expected_names = set(EXPECTED)
    if names != expected_names:
        missing = sorted(expected_names - names)
        extra = sorted(names - expected_names)
        fail(f"M1 foundation mutation roster drifted: missing={missing}, extra={extra}")
    if [row[0] for row in rows] != sorted(names):
        fail("M1 foundation-mutation registry is not sorted")

    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("x", encoding="utf-8") as stream:
        for row in rows:
            stream.write("|".join(row) + "\n")


if __name__ == "__main__":
    main()
