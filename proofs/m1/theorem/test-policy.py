#!/usr/bin/env python3
"""Exercise hostile structural changes against the M1 theorem registry."""

from __future__ import annotations

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Callable, NoReturn


REGISTRY = Path("proofs/m1/theorem/REQUIRED_FOUNDATIONS")
FixtureMutation = Callable[[Path], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def copy_fixture(repo: Path, destination: Path) -> None:
    for relative in (
        "docs/ROADMAP.md",
        "docs/M1_PROPERTY_CONTRACT.md",
        "proofs/check-m1-requirements.py",
        "proofs/M1_REQUIREMENTS.json",
        "proofs/VERIFIED_MODULES",
        "proofs/m1/theorem/check-registry.py",
        str(REGISTRY),
        "proofs/m1/model_bundle.rs",
        "crates/ferric-spec/src/continuous_batching.rs",
        "crates/ferric-spec/src/graph.rs",
        "crates/ferric-spec/src/m1_foundation_theorems.rs",
        "crates/ferric-spec/src/paged_kv_refinement.rs",
        "crates/ferric-spec/src/speculative_step_composition.rs",
        "crates/ferric-spec/src/step_plan_publication.rs",
    ):
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(repo / relative, target)


def run_checker(fixture: Path, output: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "-I",
            str(fixture / "proofs/m1/theorem/check-registry.py"),
            str(fixture),
            str(fixture / REGISTRY),
            str(output),
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )


def mutate_registry(fixture: Path, mutation: Callable[[list[str]], None]) -> None:
    path = fixture / REGISTRY
    lines = path.read_text(encoding="ascii").splitlines()
    mutation(lines)
    path.write_text("\n".join(lines) + "\n", encoding="ascii")


def replace_field(lines: list[str], row: int, field: int, value: str) -> None:
    prefix, record = lines[row].split("=", 1)
    fields = record.split("|")
    fields[field] = value
    lines[row] = prefix + "=" + "|".join(fields)


def registry_row(lines: list[str], name: str) -> int:
    matches = [
        position
        for position, line in enumerate(lines)
        if line.startswith(f"theorem={name}|")
    ]
    if len(matches) != 1:
        fail(f"fixture theorem row is not unique: {name}")
    return matches[0]


def expect_rejected(
    repo: Path,
    root: Path,
    name: str,
    expected: str,
    mutation: FixtureMutation,
) -> None:
    fixture = root / name
    copy_fixture(repo, fixture)
    mutation(fixture)
    result = run_checker(fixture, fixture / "active")
    if result.returncode == 0 or expected not in result.stdout:
        fail(
            f"{name} was not rejected with {expected!r} "
            f"(status={result.returncode})\n{result.stdout}"
        )
    shutil.rmtree(fixture)


def main() -> None:
    if len(sys.argv) != 2:
        fail(f"usage: {sys.argv[0]} REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-theorem-registry.") as scratch:
        root = Path(scratch)
        baseline = root / "baseline"
        copy_fixture(repo, baseline)
        active = root / "baseline.active"
        result = run_checker(baseline, active)
        if result.returncode != 0:
            fail(f"baseline theorem registry failed\n{result.stdout}")
        if len(active.read_text(encoding="ascii").splitlines()) != 13:
            fail("baseline theorem registry did not select exactly thirteen rows")

        cases: list[tuple[str, str, FixtureMutation]] = [
            (
                "format-drift",
                "unsupported M1 positive-theorem registry",
                lambda fixture: mutate_registry(
                    fixture,
                    lambda lines: lines.__setitem__(
                        0, "format=FERRIC-M1-POSITIVE-THEOREMS-V0"
                    ),
                ),
            ),
            (
                "row-omission",
                "positive theorem roster drifted",
                lambda fixture: mutate_registry(fixture, lambda lines: lines.pop()),
            ),
            (
                "unknown-row",
                "unknown M1 positive theorem",
                lambda fixture: mutate_registry(
                    fixture,
                    lambda lines: lines.append(
                        lines[1].replace("batching-publish-once", "unknown-row")
                    ),
                ),
            ),
            (
                "duplicate-row",
                "duplicate M1 positive theorem",
                lambda fixture: mutate_registry(
                    fixture, lambda lines: lines.append(lines[1])
                ),
            ),
            (
                "row-order",
                "registry is not sorted",
                lambda fixture: mutate_registry(
                    fixture,
                    lambda lines: lines.__setitem__(slice(1, 3), [lines[2], lines[1]]),
                ),
            ),
            (
                "field-omission",
                "malformed M1 positive-theorem record",
                lambda fixture: mutate_registry(
                    fixture,
                    lambda lines: lines.__setitem__(1, lines[1].rsplit("|", 1)[0]),
                ),
            ),
            (
                "binding-drift",
                "positive theorem binding drifted",
                lambda fixture: mutate_registry(
                    fixture,
                    lambda lines: replace_field(lines, 1, 7, "wrong_function"),
                ),
            ),
            (
                "speculative-binding-drift",
                "positive theorem binding drifted",
                lambda fixture: mutate_registry(
                    fixture,
                    lambda lines: replace_field(
                        lines,
                        registry_row(lines, "speculative-accepted-count-binding"),
                        7,
                        "exact_draft_tokens",
                    ),
                ),
            ),
            (
                "unsafe-source",
                "unsafe theorem source path",
                lambda fixture: mutate_registry(
                    fixture,
                    lambda lines: replace_field(lines, 1, 5, "../source.rs"),
                ),
            ),
            (
                "missing-source",
                "M1 theorem source is unavailable",
                lambda fixture: (
                    fixture / "crates/ferric-spec/src/m1_foundation_theorems.rs"
                ).unlink(),
            ),
            (
                "missing-model-bundle-source",
                "M1 theorem source is unavailable",
                lambda fixture: (
                    fixture / "proofs/m1/model_bundle.rs"
                ).unlink(),
            ),
            (
                "missing-module-coverage",
                "theorem module path is not inventoried",
                lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                    "\n".join(
                        line
                        for line in (fixture / "proofs/VERIFIED_MODULES")
                        .read_text(encoding="utf-8")
                        .splitlines()
                        if line
                        != "module=ferric-spec|crates/ferric-spec/src/m1_foundation_theorems.rs|ferric_spec::m1_foundation_theorems"
                    )
                    + "\n",
                    encoding="utf-8",
                ),
            ),
            (
                "missing-function-coverage",
                "function path is not directly verified",
                lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                    "\n".join(
                        line
                        for line in (fixture / "proofs/VERIFIED_MODULES")
                        .read_text(encoding="utf-8")
                        .splitlines()
                        if line
                        != "verified=ferric-spec|crates/ferric-spec/src/m1_foundation_theorems.rs|ferric_spec::m1_foundation_theorems::batching_publish_once_theorem"
                    )
                    + "\n",
                    encoding="utf-8",
                ),
            ),
            (
                "missing-model-bundle-module-coverage",
                "theorem module path is not inventoried",
                lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                    "\n".join(
                        line
                        for line in (fixture / "proofs/VERIFIED_MODULES")
                        .read_text(encoding="utf-8")
                        .splitlines()
                        if line
                        != "module=ferric-m1-proof|proofs/m1/model_bundle.rs|ferric_m1_proof::model_bundle"
                    )
                    + "\n",
                    encoding="utf-8",
                ),
            ),
            (
                "missing-model-bundle-function-coverage",
                "function path is not directly verified",
                lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                    "\n".join(
                        line
                        for line in (fixture / "proofs/VERIFIED_MODULES")
                        .read_text(encoding="utf-8")
                        .splitlines()
                        if line
                        != "verified=ferric-m1-proof|proofs/m1/model_bundle.rs|ferric_m1_proof::model_bundle::model_bundle_well_formed_composition_theorem"
                    )
                    + "\n",
                    encoding="utf-8",
                ),
            ),
            (
                "missing-speculative-function-coverage",
                "function path is not directly verified",
                lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                    "\n".join(
                        line
                        for line in (fixture / "proofs/VERIFIED_MODULES")
                        .read_text(encoding="utf-8")
                        .splitlines()
                        if line
                        != "verified=ferric-spec|crates/ferric-spec/src/m1_foundation_theorems.rs|ferric_spec::m1_foundation_theorems::speculative_accepted_count_binding_theorem"
                    )
                    + "\n",
                    encoding="utf-8",
                ),
            ),
        ]

        def close_property(fixture: Path) -> None:
            path = fixture / "proofs/M1_REQUIREMENTS.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            next(
                row
                for row in value["assurance_properties"]
                if row["name"] == "scheduler_refined"
            )["obligation_state"] = "Proved"
            path.write_text(
                json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )

        cases.append(("premature-property-closure", "must remain Open", close_property))

        def close_speculative_property(fixture: Path) -> None:
            path = fixture / "proofs/M1_REQUIREMENTS.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            next(
                row
                for row in value["assurance_properties"]
                if row["name"] == "rollback_refined"
            )["obligation_state"] = "Proved"
            path.write_text(
                json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )

        cases.append(
            (
                "premature-speculative-property-closure",
                "must remain Open",
                close_speculative_property,
            )
        )
        for name, expected, mutation in cases:
            expect_rejected(repo, root, name, expected, mutation)

        occupied = root / "occupied"
        copy_fixture(repo, occupied)
        output = occupied / "active"
        output.write_text("occupied\n", encoding="ascii")
        result = run_checker(occupied, output)
        if result.returncode == 0 or "output already exists" not in result.stdout:
            fail("pre-existing theorem active output was not rejected")

    print(
        f"PASS: M1 positive-theorem registry policy ({len(cases) + 1} hostile fixtures)"
    )


if __name__ == "__main__":
    main()
