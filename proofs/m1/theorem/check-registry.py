#!/usr/bin/env python3
"""Validate the finite M1 positive-theorem foundation registry."""

from __future__ import annotations

import json
from pathlib import Path
import re
import stat
import subprocess
import sys
from typing import NoReturn


FORMAT = "format=FERRIC-M1-POSITIVE-THEOREMS-V1"
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_TARGET = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*\Z")

# These are existing direct-Verus foundations associated with still-Open M1
# paths. Registry membership does not implement or discharge those paths.
EXPECTED = {
    "batching-publish-once": (
        "continuous-batching",
        "scheduler_refined",
        "batching-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "batching_publish_once_theorem",
    ),
    "batching-request-routing": (
        "continuous-batching",
        "scheduler_refined",
        "scheduler-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "batching_request_routing_theorem",
    ),
    "graph-operator-order": (
        "exact-graph-plan",
        "graph_refined",
        "graph-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "graph_operator_order_theorem",
    ),
    "graph-role-step-count": (
        "exact-graph-plan",
        "graph_refined",
        "graph-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "graph_role_step_count_theorem",
    ),
    "isolation-other-request-frame": (
        "continuous-batching",
        "request_isolated",
        "isolation-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "isolation_other_request_frame_theorem",
    ),
    "kv-release-generation": (
        "logical-paged-kv",
        "kv_refined",
        "kv-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "kv_release_generation_theorem",
    ),
    "kv-rollback-retirement": (
        "logical-paged-kv",
        "kv_refined",
        "kv-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "kv_rollback_retirement_theorem",
    ),
    "kv-write-prefix": (
        "logical-paged-kv",
        "kv_refined",
        "kv-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "kv_write_prefix_theorem",
    ),
    "model-bundle-composition": (
        "model-bundle-composition",
        "model_bundle_well_formed",
        "model-bundle-proof",
        "ferric-m1-proof",
        "proofs/m1/model_bundle.rs",
        "model_bundle",
        "model_bundle_well_formed_composition_theorem",
    ),
    "publication-phase-transition": (
        "step-plan-publication",
        "graph_refined",
        "graph-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "publication_phase_transition_theorem",
    ),
    "publication-plan-identity": (
        "step-plan-publication",
        "graph_refined",
        "graph-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "publication_plan_identity_theorem",
    ),
    "speculative-accepted-count-binding": (
        "speculative-step-composition",
        "rollback_refined",
        "speculation-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "speculative_accepted_count_binding_theorem",
    ),
    "speculative-atomic-failure-frame": (
        "speculative-step-composition",
        "request_isolated",
        "isolation-proof",
        "ferric-spec",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "m1_foundation_theorems",
        "speculative_atomic_failure_frame_theorem",
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


def requirements(repo: Path) -> tuple[dict[str, dict], dict[str, dict]]:
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
    manifest = json.loads(
        (repo / "proofs/M1_REQUIREMENTS.json").read_text(encoding="utf-8")
    )
    properties = {row["name"]: row for row in manifest["assurance_properties"]}
    paths = {row["id"]: row for row in manifest["path_obligations"]}
    return properties, paths


def coverage(repo: Path) -> tuple[set[tuple[str, str, str]], set[tuple[str, str, str]]]:
    path = repo / "proofs/VERIFIED_MODULES"
    regular_file(path, "compiler-rooted proof coverage manifest")
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "format=FERRIC-VERIFIED-MODULES-V2":
        fail("unsupported compiler-rooted coverage manifest")
    modules: set[tuple[str, str, str]] = set()
    verified: set[tuple[str, str, str]] = set()
    for line in lines[1:]:
        target = None
        if line.startswith("module="):
            target = modules
            fields = tuple(line.removeprefix("module=").split("|"))
        elif line.startswith("verified="):
            target = verified
            fields = tuple(line.removeprefix("verified=").split("|"))
        else:
            continue
        if len(fields) != 3 or fields in target:
            fail("malformed or duplicate compiler-rooted coverage record")
        target.add(fields)
    return modules, verified


def main() -> None:
    if len(sys.argv) != 4:
        fail("usage: check-registry.py REPO REGISTRY ACTIVE_OUTPUT")
    repo = Path(sys.argv[1]).resolve(strict=True)
    registry = Path(sys.argv[2])
    output = Path(sys.argv[3])
    regular_file(registry, "M1 positive-theorem registry")
    if output.exists() or output.is_symlink():
        fail(f"active M1 theorem output already exists: {output}")

    properties, paths = requirements(repo)
    modules, verified = coverage(repo)
    lines = registry.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != FORMAT:
        fail("unsupported M1 positive-theorem registry")
    rows: list[tuple[str, ...]] = []
    names: set[str] = set()
    for line in lines[1:]:
        if not line.startswith("theorem="):
            fail(f"malformed M1 positive-theorem record: {line}")
        fields = tuple(line.removeprefix("theorem=").split("|"))
        if len(fields) != 8:
            fail(f"malformed M1 positive-theorem record: {line}")
        name, foundation, property_name, path_id, package, source, module, function = (
            fields
        )
        for value, description in (
            (name, "theorem name"),
            (foundation, "foundation name"),
            (property_name, "assurance property"),
            (path_id, "path obligation"),
            (package, "package name"),
        ):
            if SAFE_NAME.fullmatch(value) is None:
                fail(f"unsafe M1 {description}: {value!r}")
        if (
            SAFE_TARGET.fullmatch(module) is None
            or SAFE_TARGET.fullmatch(function) is None
        ):
            fail(f"unsafe M1 compiler target: {module}::{function}")
        source_path = safe_relative(source, "theorem source path")
        if name in names:
            fail(f"duplicate M1 positive theorem: {name}")
        names.add(name)
        expected = EXPECTED.get(name)
        if expected is None:
            fail(f"unknown M1 positive theorem: {name}")
        if fields[1:] != expected:
            fail(f"M1 positive theorem binding drifted: {name}")
        property_row = properties.get(property_name)
        if property_row is None or property_row["obligation_state"] != "Open":
            fail(f"M1 theorem property is absent or not Open: {property_name}")
        if path_id not in property_row["path_obligations"]:
            fail(f"M1 theorem path is not assigned to property: {name}")
        path_row = paths.get(path_id)
        if path_row is None or path_row["obligation_state"] != "Open":
            fail(f"M1 theorem path is absent or not Open: {path_id}")
        regular_file(repo / source_path, "M1 theorem source")
        compiler_module = f"{package.replace('-', '_')}::{module}"
        compiler_path = f"{compiler_module}::{function}"
        if (package, source, compiler_module) not in modules:
            fail(f"M1 theorem module path is not inventoried: {name}")
        if (package, source, compiler_path) not in verified:
            fail(f"M1 theorem function path is not directly verified: {name}")
        rows.append(fields)

    expected_names = set(EXPECTED)
    if names != expected_names:
        fail(
            "M1 positive theorem roster drifted: "
            f"missing={sorted(expected_names - names)}, "
            f"extra={sorted(names - expected_names)}"
        )
    if [row[0] for row in rows] != sorted(names):
        fail("M1 positive-theorem registry is not sorted")
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("x", encoding="utf-8") as stream:
        for row in rows:
            stream.write("|".join(row) + "\n")


if __name__ == "__main__":
    main()
