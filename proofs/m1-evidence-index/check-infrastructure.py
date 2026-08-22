#!/usr/bin/env python3
"""Fail-closed preflight for the reusable M1 evidence infrastructure."""

from __future__ import annotations

import ast
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
from pathlib import Path, PurePosixPath
from typing import Any, NoReturn


INDEX_CHECKER = "proofs/check-m1-evidence-index.py"
REQUIREMENTS = "proofs/M1_REQUIREMENTS.json"
TCB_VALIDATOR_ID = "tcb-report"
RECEIPT_VALIDATOR_ID = "qualification-receipt"
EXPECTED_INDEX_FORMAT = "ferric.m1-evidence-index.v1"
EXPECTED_TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
EXPECTED_SOURCE_IDS = ("source.fe2o3", "source.ferric")
EXPECTED_GATE_IDS = (
    "evidence-index",
    "hardware",
    "performance",
    "proof",
    "quality",
    "source-closure",
    "validators",
)
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
PROTOCOL = re.compile(r"ferric\.m1-validator\.[a-z0-9.-]+\.v1\Z")
ARTIFACT_KIND = re.compile(r"[A-Z][A-Za-z0-9]+\Z")
MAX_JSON_BYTES = 2_000_000
MAX_PYTHON_BYTES = 4_000_000


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def regular_file(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {path}: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"{description} must be a regular nonsymlink file: {path}")


def read_bounded(path: Path, limit: int, description: str) -> bytes:
    regular_file(path, description)
    try:
        with path.open("rb") as source:
            value = source.read(limit + 1)
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    if not value or len(value) > limit:
        fail(f"{description} is empty or oversized")
    return value


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON key in M1 requirements: {key}")
        result[key] = value
    return result


def load_requirements(repo: Path) -> dict[str, Any]:
    path = repo / REQUIREMENTS
    raw = read_bounded(path, MAX_JSON_BYTES, "M1 requirements manifest")
    try:
        source = raw.decode("utf-8")
        value = json.loads(source, object_pairs_hook=unique_object)
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse M1 requirements manifest: {error}")
    if not isinstance(value, dict):
        fail("M1 requirements manifest must be an object")
    canonical = json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    if source != canonical:
        fail("M1 requirements manifest is not canonical JSON")
    return value


def validate_requirements(repo: Path) -> None:
    checker = repo / "proofs/check-m1-requirements.py"
    regular_file(checker, "M1 requirements checker")
    try:
        result = subprocess.run(
            [sys.executable, "-I", str(checker), str(repo)],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=30,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"M1 requirements checker could not run: {error}")
    if result.returncode != 0:
        fail(f"M1 requirements are invalid:\n{result.stdout}")


def parse_python(path: Path, description: str) -> tuple[bytes, ast.Module]:
    raw = read_bounded(path, MAX_PYTHON_BYTES, description)
    try:
        tree = ast.parse(raw, filename=str(path))
    except SyntaxError as error:
        fail(f"cannot parse {description}: {error}")
    return raw, tree


def literal_assignments(
    tree: ast.Module, names: set[str], description: str
) -> dict[str, Any]:
    assignments: dict[str, list[ast.AST]] = {name: [] for name in names}
    for node in tree.body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1:
            target = node.targets[0]
            if isinstance(target, ast.Name) and target.id in assignments:
                assignments[target.id].append(node.value)
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            if node.target.id in assignments and node.value is not None:
                assignments[node.target.id].append(node.value)
    result: dict[str, Any] = {}
    for name, values in assignments.items():
        if len(values) != 1:
            fail(f"{description} must define exactly one literal {name}")
        try:
            result[name] = ast.literal_eval(values[0])
        except (
            ValueError,
            TypeError,
            SyntaxError,
            MemoryError,
            RecursionError,
        ) as error:
            fail(f"{description} {name} is not literal data: {error}")
    return result


def safe_validator_path(repo: Path, value: Any, evidence_kind: str) -> Path:
    if not isinstance(value, str):
        fail(f"trusted validator path is not a string: {evidence_kind}")
    relative = PurePosixPath(value)
    expected_name = f"validate-{evidence_kind}.py"
    if evidence_kind == "canonical-structure-check":
        expected_name = "validate-canonical-structure.py"
    elif evidence_kind == "hardware-test":
        expected_name = "validate-hardware-transcript.py"
    elif evidence_kind == "performance-gate":
        expected_name = "validate-performance-report.py"
    if (
        relative.is_absolute()
        or ".." in relative.parts
        or relative.parent.as_posix() != "proofs/m1/evidence"
        or relative.name != expected_name
    ):
        fail(f"trusted validator path is noncanonical: {evidence_kind}: {value!r}")
    return repo / Path(*relative.parts)


def protocol_from_validator(tree: ast.Module, evidence_kind: str) -> str:
    value = literal_assignments(tree, {"PROTOCOL"}, f"{evidence_kind} validator")
    protocol = value["PROTOCOL"]
    if not isinstance(protocol, str) or PROTOCOL.fullmatch(protocol) is None:
        fail(f"trusted validator protocol is invalid: {evidence_kind}")
    return protocol


def validate_tcb_mirror(
    tree: ast.Module,
    evidence_kinds: tuple[str, ...],
    validator_specs: tuple[tuple[str, str, str], ...],
) -> None:
    values = literal_assignments(
        tree,
        {"EVIDENCE_KINDS", "TCB_IDS", "VALIDATOR_SPECS"},
        "TCB-report validator",
    )
    if values["EVIDENCE_KINDS"] != evidence_kinds:
        fail("TCB-report evidence-kind roster drifted")
    if values["TCB_IDS"] != EXPECTED_TCB_IDS:
        fail("TCB-report trusted-boundary roster drifted")
    raw_specs = values["VALIDATOR_SPECS"]
    if not isinstance(raw_specs, tuple):
        fail("TCB-report validator registry is not a tuple")
    resolved: list[tuple[str, str, str]] = []
    own_protocol = protocol_from_validator(tree, TCB_VALIDATOR_ID)
    for record in raw_specs:
        if not isinstance(record, tuple) or len(record) != 3:
            fail("TCB-report validator registry contains a malformed row")
        kind, path, protocol = record
        if protocol == "__SELF_PROTOCOL__":
            protocol = own_protocol
        if not all(isinstance(item, str) for item in (kind, path, protocol)):
            fail("TCB-report validator registry contains a non-string field")
        resolved.append((kind, path, protocol))
    if tuple(resolved) != validator_specs:
        fail("TCB-report trusted-validator registry drifted")


def tcb_tree_with_literal_self_protocol(tree: ast.Module) -> ast.Module:
    class ReplaceSelfProtocol(ast.NodeTransformer):
        def visit_Name(self, node: ast.Name) -> ast.AST:
            if node.id == "PROTOCOL":
                return ast.copy_location(ast.Constant("__SELF_PROTOCOL__"), node)
            return node

    for node in tree.body:
        if isinstance(node, ast.Assign) and any(
            isinstance(target, ast.Name) and target.id == "VALIDATOR_SPECS"
            for target in node.targets
        ):
            node.value = ReplaceSelfProtocol().visit(node.value)
            return tree
    fail("TCB-report validator must define exactly one VALIDATOR_SPECS registry")


def validate_receipt_mirror(
    tree: ast.Module,
    evidence_kinds: tuple[str, ...],
    validator_ids: tuple[str, ...],
    artifact_kinds: dict[str, str],
) -> None:
    values = literal_assignments(
        tree,
        {
            "EVIDENCE_ARTIFACT_KINDS",
            "GATE_IDS",
            "SOURCE_IDS",
            "TCB_IDS",
            "VALIDATOR_IDS",
        },
        "qualification-receipt validator",
    )
    if values["VALIDATOR_IDS"] != validator_ids:
        fail("qualification-receipt trusted-validator roster drifted")
    if values["EVIDENCE_ARTIFACT_KINDS"] != artifact_kinds:
        fail("qualification-receipt artifact-kind registry drifted")
    if tuple(values["EVIDENCE_ARTIFACT_KINDS"]) != evidence_kinds:
        fail("qualification-receipt evidence-kind roster drifted")
    if values["TCB_IDS"] != EXPECTED_TCB_IDS:
        fail("qualification-receipt trusted-boundary roster drifted")
    if values["SOURCE_IDS"] != EXPECTED_SOURCE_IDS:
        fail("qualification-receipt source roster drifted")
    if values["GATE_IDS"] != EXPECTED_GATE_IDS:
        fail("qualification-receipt gate roster drifted")


def validate_infrastructure(repo: Path) -> None:
    try:
        repo = repo.resolve(strict=True)
    except OSError as error:
        fail(f"Ferric repository is unavailable: {error}")
    if not repo.is_dir():
        fail("Ferric repository must be a directory")

    validate_requirements(repo)
    requirements = load_requirements(repo)
    evidence_value = requirements.get("evidence_kinds")
    if (
        not isinstance(evidence_value, list)
        or not evidence_value
        or not all(isinstance(item, str) for item in evidence_value)
        or evidence_value != sorted(evidence_value)
        or len(evidence_value) != len(set(evidence_value))
    ):
        fail("M1 evidence-kind roster is not a unique canonical string array")
    evidence_kinds = tuple(evidence_value)

    checker_path = repo / INDEX_CHECKER
    _checker_raw, checker_tree = parse_python(checker_path, "M1 evidence-index checker")
    checker_values = literal_assignments(
        checker_tree,
        {
            "EVIDENCE_ARTIFACT_KINDS",
            "FORMAT",
            "SOURCE_IDS",
            "TCB_IDS",
            "TRUSTED_VALIDATORS",
        },
        "M1 evidence-index checker",
    )
    if checker_values["FORMAT"] != EXPECTED_INDEX_FORMAT:
        fail("M1 evidence-index format drifted")
    if checker_values["TCB_IDS"] != EXPECTED_TCB_IDS:
        fail("M1 evidence-index trusted-boundary roster drifted")
    if checker_values["SOURCE_IDS"] != EXPECTED_SOURCE_IDS:
        fail("M1 evidence-index source roster drifted")
    artifact_kinds = checker_values["EVIDENCE_ARTIFACT_KINDS"]
    if not isinstance(artifact_kinds, dict) or tuple(artifact_kinds) != evidence_kinds:
        fail("M1 evidence-index artifact-kind registry is incomplete or reordered")
    if not all(
        isinstance(value, str) and ARTIFACT_KIND.fullmatch(value) is not None
        for value in artifact_kinds.values()
    ):
        fail("M1 evidence-index artifact-kind registry contains an invalid kind")
    validators = checker_values["TRUSTED_VALIDATORS"]
    validator_ids = tuple(sorted((*evidence_kinds, RECEIPT_VALIDATOR_ID)))
    if not isinstance(validators, dict) or tuple(validators) != validator_ids:
        fail("M1 trusted-validator registry is incomplete or reordered")

    paths: set[str] = set()
    protocols: set[str] = set()
    source_pins: set[str] = set()
    validator_specs: list[tuple[str, str, str]] = []
    trees: dict[str, ast.Module] = {}
    for evidence_kind in validator_ids:
        record = validators[evidence_kind]
        if not isinstance(record, tuple) or len(record) != 3:
            fail(f"trusted validator registry row is malformed: {evidence_kind}")
        relative, protocol, source_pin = record
        path = safe_validator_path(repo, relative, evidence_kind)
        if not isinstance(protocol, str) or PROTOCOL.fullmatch(protocol) is None:
            fail(f"trusted validator registry protocol is invalid: {evidence_kind}")
        if (
            not isinstance(source_pin, str)
            or SHA256.fullmatch(source_pin) is None
            or len(set(source_pin)) == 1
        ):
            fail(f"trusted validator source pin is absent or invalid: {evidence_kind}")
        raw, tree = parse_python(path, f"trusted {evidence_kind} validator")
        if digest(raw) != source_pin:
            fail(f"trusted validator source identity mismatch: {evidence_kind}")
        if protocol_from_validator(tree, evidence_kind) != protocol:
            fail(f"trusted validator protocol mismatch: {evidence_kind}")
        if relative in paths or protocol in protocols or source_pin in source_pins:
            fail(f"trusted validator identity is reused: {evidence_kind}")
        paths.add(relative)
        protocols.add(protocol)
        source_pins.add(source_pin)
        trees[evidence_kind] = tree
        validator_specs.append((evidence_kind, relative, protocol))

    tcb_tree = tcb_tree_with_literal_self_protocol(trees[TCB_VALIDATOR_ID])
    validate_tcb_mirror(tcb_tree, evidence_kinds, tuple(validator_specs))
    validate_receipt_mirror(
        trees[RECEIPT_VALIDATOR_ID],
        evidence_kinds,
        validator_ids,
        artifact_kinds,
    )

    print(
        "PASS: M1 evidence infrastructure is internally pinned "
        f"({len(evidence_kinds)} evidence kinds, {len(validator_ids)} validators, "
        f"{len(EXPECTED_TCB_IDS)} TCB classes, "
        f"{len(EXPECTED_GATE_IDS)} receipt gates); "
        "external closure remains absent"
    )


def main() -> None:
    if len(sys.argv) != 2:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO")
    validate_infrastructure(Path(sys.argv[1]))


if __name__ == "__main__":
    main()
