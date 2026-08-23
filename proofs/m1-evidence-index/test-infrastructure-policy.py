#!/usr/bin/env python3
"""Hostile policy tests for the static M1 evidence-infrastructure preflight."""

from __future__ import annotations

import ast
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Callable


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "proofs/m1-evidence-index/check-infrastructure.py"
INDEX_CHECKER = Path("proofs/check-m1-evidence-index.py")
ARTIFACT_VALIDATOR = Path("proofs/m1/evidence/validate-artifact-identity.py")
NEGATIVE_VALIDATOR = Path("proofs/m1/evidence/validate-negative-mutation.py")
TCB_VALIDATOR = Path("proofs/m1/evidence/validate-tcb-report.py")
RECEIPT_VALIDATOR = Path("proofs/m1/evidence/validate-qualification-receipt.py")


def invoke(repo: Path) -> tuple[bool, str]:
    result = subprocess.run(
        [sys.executable, "-I", str(CHECKER), str(repo)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=30,
        env={"PATH": os.environ.get("PATH", "")},
    )
    return result.returncode == 0, result.stdout


def replace_once(path: Path, old: str, new: str) -> None:
    source = path.read_text(encoding="utf-8")
    if source.count(old) != 1:
        raise AssertionError(f"fixture anchor is not unique in {path}: {old!r}")
    path.write_text(source.replace(old, new, 1), encoding="utf-8")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def repin_validator(fixture: Path, relative: Path) -> None:
    original_digest = sha256(ROOT / relative)
    replacement_digest = sha256(fixture / relative)
    replace_once(
        fixture / INDEX_CHECKER,
        f'"{original_digest}"',
        f'"{replacement_digest}"',
    )


def remove_registry_row(path: Path, evidence_kind: str) -> None:
    source = path.read_text(encoding="utf-8")
    tree = ast.parse(source, filename=str(path))
    registry: ast.Dict | None = None
    for node in tree.body:
        if (
            isinstance(node, ast.Assign)
            and len(node.targets) == 1
            and isinstance(node.targets[0], ast.Name)
            and node.targets[0].id == "TRUSTED_VALIDATORS"
            and isinstance(node.value, ast.Dict)
        ):
            registry = node.value
            break
    if registry is None:
        raise AssertionError("fixture trusted-validator registry is unavailable")
    for key, value in zip(registry.keys, registry.values, strict=True):
        if isinstance(key, ast.Constant) and key.value == evidence_kind:
            lines = source.splitlines(keepends=True)
            del lines[key.lineno - 1 : value.end_lineno]
            path.write_text("".join(lines), encoding="utf-8")
            return
    raise AssertionError(f"fixture registry row is unavailable: {evidence_kind}")


def reset_path(fixture: Path, relative: Path) -> None:
    target = fixture / relative
    if target.is_symlink() or target.is_file():
        target.unlink()
    elif target.exists():
        shutil.rmtree(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / relative, target)


def main() -> None:
    with tempfile.TemporaryDirectory(
        prefix="ferric-m1-infrastructure-policy."
    ) as scratch:
        fixture = Path(scratch) / "ferric"
        shutil.copytree(
            ROOT,
            fixture,
            ignore=shutil.ignore_patterns(
                ".git",
                ".ruff_cache",
                "target",
                "__pycache__",
                "*.pyc",
                "*.receipt",
            ),
        )

        passed, output = invoke(fixture)
        if not passed or "external closure remains absent" not in output:
            raise AssertionError(f"baseline infrastructure preflight failed:\n{output}")
        print("PASS: baseline M1 evidence-infrastructure preflight")

        cases: list[tuple[str, str, tuple[Path, ...], Callable[[Path], None]]] = []

        def add(
            name: str,
            marker: str,
            touched: tuple[Path, ...],
            mutate: Callable[[Path], None],
        ) -> None:
            cases.append((name, marker, touched, mutate))

        add(
            "missing index checker",
            "M1 evidence-index checker is unavailable",
            (INDEX_CHECKER,),
            lambda repo: (repo / INDEX_CHECKER).unlink(),
        )
        add(
            "missing validator",
            "trusted artifact-identity validator is unavailable",
            (ARTIFACT_VALIDATOR,),
            lambda repo: (repo / ARTIFACT_VALIDATOR).unlink(),
        )

        def symlink_validator(repo: Path) -> None:
            path = repo / ARTIFACT_VALIDATOR
            path.unlink()
            path.symlink_to("validate-canonical-structure.py")

        add(
            "symlink validator",
            "must be a regular nonsymlink file",
            (ARTIFACT_VALIDATOR,),
            symlink_validator,
        )
        add(
            "validator source substitution",
            "trusted validator source identity mismatch",
            (ARTIFACT_VALIDATOR,),
            lambda repo: (repo / ARTIFACT_VALIDATOR).write_text(
                "raise SystemExit(0)\n", encoding="utf-8"
            ),
        )

        def remove_source_pin(repo: Path) -> None:
            digest = sha256(repo / ARTIFACT_VALIDATOR)
            replace_once(repo / INDEX_CHECKER, f'"{digest}"', "None")

        add(
            "unpinned validator",
            "trusted validator source pin is absent or invalid",
            (INDEX_CHECKER,),
            remove_source_pin,
        )
        add(
            "validator registry omission",
            "trusted-validator registry is incomplete or reordered",
            (INDEX_CHECKER,),
            lambda repo: remove_registry_row(repo / INDEX_CHECKER, "artifact-identity"),
        )

        def drift_protocol(repo: Path) -> None:
            replace_once(
                repo / ARTIFACT_VALIDATOR,
                'PROTOCOL = "ferric.m1-validator.artifact-identity.v1"',
                'PROTOCOL = "ferric.m1-validator.artifact-identity-alt.v1"',
            )
            repin_validator(repo, ARTIFACT_VALIDATOR)

        add(
            "validator protocol drift",
            "trusted validator protocol mismatch",
            (INDEX_CHECKER, ARTIFACT_VALIDATOR),
            drift_protocol,
        )

        def widen_negative_support(repo: Path) -> None:
            replace_once(
                repo / NEGATIVE_VALIDATOR,
                'OBLIGATION_CLASSES = ("Assurance",)',
                'OBLIGATION_CLASSES = ("Assurance", "Roadmap")',
            )
            repin_validator(repo, NEGATIVE_VALIDATOR)

        add(
            "validator binding-class widening",
            "trusted validator obligation-class support drifted: negative-mutation",
            (INDEX_CHECKER, NEGATIVE_VALIDATOR),
            widen_negative_support,
        )

        def remove_artifact_support(repo: Path) -> None:
            replace_once(
                repo / ARTIFACT_VALIDATOR,
                'OBLIGATION_CLASSES = ("Assurance", "Roadmap")',
                'OBLIGATION_CLASSES = ("Assurance",)',
            )
            repin_validator(repo, ARTIFACT_VALIDATOR)

        add(
            "validator binding-class removal",
            "trusted validator obligation-class support drifted: artifact-identity",
            (INDEX_CHECKER, ARTIFACT_VALIDATOR),
            remove_artifact_support,
        )

        def drift_artifact_registry(repo: Path) -> None:
            replace_once(
                repo / INDEX_CHECKER,
                '"artifact-identity": "ArtifactIdentityReport"',
                '"artifact-identity-substitute": "ArtifactIdentityReport"',
            )

        add(
            "artifact-kind registry drift",
            "artifact-kind registry is incomplete or reordered",
            (INDEX_CHECKER,),
            drift_artifact_registry,
        )

        def drift_tcb_mirror(repo: Path) -> None:
            path = repo / TCB_VALIDATOR
            source = path.read_text(encoding="utf-8")
            marker = 'VALIDATOR_SPECS = (\n    (\n        "artifact-identity",'
            replacement = (
                'VALIDATOR_SPECS = (\n    (\n        "artifact-identity-substitute",'
            )
            if source.count(marker) != 1:
                raise AssertionError("TCB validator registry anchor drifted")
            path.write_text(source.replace(marker, replacement, 1), encoding="utf-8")
            repin_validator(repo, TCB_VALIDATOR)

        add(
            "TCB validator mirror drift",
            "TCB-report trusted-validator registry drifted",
            (INDEX_CHECKER, TCB_VALIDATOR),
            drift_tcb_mirror,
        )

        def drift_tcb_binding_classes(repo: Path) -> None:
            replace_once(
                repo / TCB_VALIDATOR,
                '("negative-mutation", ("Assurance",)),',
                '("negative-mutation", ("Assurance", "Roadmap")),',
            )
            repin_validator(repo, TCB_VALIDATOR)

        add(
            "TCB binding-class mirror drift",
            "TCB-report evidence-kind binding-class roster drifted",
            (INDEX_CHECKER, TCB_VALIDATOR),
            drift_tcb_binding_classes,
        )

        def drift_receipt_mirror(repo: Path) -> None:
            replace_once(
                repo / RECEIPT_VALIDATOR,
                'VALIDATOR_IDS = (\n    "artifact-identity",',
                'VALIDATOR_IDS = (\n    "artifact-identity-substitute",',
            )
            repin_validator(repo, RECEIPT_VALIDATOR)

        add(
            "receipt validator mirror drift",
            "qualification-receipt trusted-validator roster drifted",
            (INDEX_CHECKER, RECEIPT_VALIDATOR),
            drift_receipt_mirror,
        )

        def drift_receipt_binding_classes(repo: Path) -> None:
            replace_once(
                repo / RECEIPT_VALIDATOR,
                '("negative-mutation", ("Assurance",)),',
                '("negative-mutation", ("Assurance", "Roadmap")),',
            )
            repin_validator(repo, RECEIPT_VALIDATOR)

        add(
            "receipt binding-class mirror drift",
            "qualification-receipt evidence-kind binding-class roster drifted",
            (INDEX_CHECKER, RECEIPT_VALIDATOR),
            drift_receipt_binding_classes,
        )

        def drift_receipt_gate(repo: Path) -> None:
            replace_once(
                repo / RECEIPT_VALIDATOR,
                'GATE_IDS = (\n    "evidence-index",',
                'GATE_IDS = (\n    "evidence-index-substitute",',
            )
            repin_validator(repo, RECEIPT_VALIDATOR)

        add(
            "receipt gate drift",
            "qualification-receipt gate roster drifted",
            (INDEX_CHECKER, RECEIPT_VALIDATOR),
            drift_receipt_gate,
        )

        for name, marker, touched, mutate in cases:
            for relative in touched:
                reset_path(fixture, relative)
            mutate(fixture)
            passed, output = invoke(fixture)
            if passed or marker not in output:
                raise AssertionError(
                    f"hostile {name} did not fail closed with {marker!r}:\n{output}"
                )
            print(f"PASS: hostile {name}")
            for relative in touched:
                reset_path(fixture, relative)

        passed, output = invoke(fixture)
        if not passed:
            raise AssertionError(f"restored infrastructure preflight failed:\n{output}")
        print(f"PASS: {len(cases)} hostile M1 evidence-infrastructure fixtures")


if __name__ == "__main__":
    main()
