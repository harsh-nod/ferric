#!/usr/bin/env python3
"""Exercise hostile structural mutations against the M1 negative registry."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Callable, NoReturn


FixtureMutation = Callable[[Path], None]
REGISTRY = Path("proofs/m1/negative/REQUIRED_FOUNDATIONS")


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def write_canonical(path: Path, value: dict) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def copy_fixture(repo: Path, destination: Path) -> None:
    (destination / "docs").mkdir(parents=True)
    shutil.copy2(repo / "docs/ROADMAP.md", destination / "docs/ROADMAP.md")
    shutil.copy2(
        repo / "docs/M1_PROPERTY_CONTRACT.md",
        destination / "docs/M1_PROPERTY_CONTRACT.md",
    )
    (destination / "proofs/m1/negative").mkdir(parents=True)
    for relative in (
        "proofs/check-m1-requirements.py",
        "proofs/M1_REQUIREMENTS.json",
        "proofs/VERIFIED_MODULES",
        "proofs/m1/negative/check-registry.py",
        str(REGISTRY),
    ):
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(repo / relative, target)
    shutil.copytree(
        repo / "proofs/m1/negative/components",
        destination / "proofs/m1/negative/components",
    )
    for relative in (
        "crates/ferric-build/src/auth.rs",
        "crates/ferric-engine/src/operation_kernel_plan.rs",
        "crates/ferric-kernels/src/validation.rs",
        "crates/ferric-spec/src/continuous_batching.rs",
        "crates/ferric-spec/src/graph.rs",
        "crates/ferric-spec/src/m1_completion.rs",
        "crates/ferric-spec/src/paged_kv_refinement.rs",
        "crates/ferric-spec/src/request_isolation.rs",
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
            str(fixture / "proofs/m1/negative/check-registry.py"),
            str(fixture),
            str(fixture / REGISTRY),
            str(output),
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )


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


def mutate_registry(fixture: Path, mutation: Callable[[list[str]], None]) -> None:
    path = fixture / REGISTRY
    lines = path.read_text(encoding="utf-8").splitlines()
    mutation(lines)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def replace_field(lines: list[str], row: int, field: int, value: str) -> None:
    prefix, record = lines[row].split("=", 1)
    fields = record.split("|")
    fields[field] = value
    lines[row] = prefix + "=" + "|".join(fields)


def registry_row(lines: list[str], name: str) -> int:
    matches = [
        position
        for position, line in enumerate(lines)
        if line.startswith(f"mutation={name}|")
    ]
    if len(matches) != 1:
        fail(f"fixture mutation row is not unique: {name}")
    return matches[0]


def verify_current_mutators(repo: Path, root: Path, active: Path) -> int:
    count = 0
    for line in active.read_text(encoding="utf-8").splitlines():
        fields = line.split("|")
        if len(fields) != 11:
            fail("baseline active registry emitted a malformed row")
        name, _, _, _, _, source, mutator, _, _, _, clause = fields
        fixture = root / f"mutator-{name}"
        source_path = fixture / source
        source_path.parent.mkdir(parents=True)
        shutil.copy2(repo / source, source_path)
        before = hashlib.sha256(source_path.read_bytes()).hexdigest()
        result = subprocess.run(
            [
                sys.executable,
                "-I",
                str(repo / "proofs/m1/negative/components" / mutator),
                str(fixture),
            ],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        prefix = f"MUTATED_SOURCE={source}\nMUTATION={name}\nCLAUSE={clause}\n"
        anchor_line = result.stdout.removeprefix(prefix).removesuffix("\n")
        anchor = anchor_line.removeprefix("ANCHOR_SHA256=")
        if (
            result.returncode != 0
            or not result.stdout.startswith(prefix)
            or not anchor_line.startswith("ANCHOR_SHA256=")
            or len(anchor) != 64
            or any(character not in "0123456789abcdef" for character in anchor)
        ):
            fail(f"{name} did not apply its exact current anchor\n{result.stdout}")
        after = hashlib.sha256(source_path.read_bytes()).hexdigest()
        if before == after:
            fail(f"{name} did not change its declared source")
        repeated = subprocess.run(
            [
                sys.executable,
                "-I",
                str(repo / "proofs/m1/negative/components" / mutator),
                str(fixture),
            ],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        if repeated.returncode == 0 or "anchor drifted" not in repeated.stdout:
            fail(f"{name} did not reject its consumed anchor")
        shutil.rmtree(fixture)
        count += 1
    return count


def main() -> None:
    if len(sys.argv) != 2:
        fail(f"usage: {sys.argv[0]} REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)

    with tempfile.TemporaryDirectory(prefix="ferric-m1-negative-policy.") as scratch:
        root = Path(scratch)
        baseline = root / "baseline"
        copy_fixture(repo, baseline)
        active = root / "baseline.active"
        result = run_checker(baseline, active)
        if result.returncode != 0:
            fail(f"baseline M1 negative registry check failed\n{result.stdout}")
        rows = active.read_text(encoding="utf-8").splitlines()
        if len(rows) != 18:
            fail(f"baseline selected {len(rows)} M1 mutations instead of 18")
        mutator_count = verify_current_mutators(repo, root, active)

        cases: list[tuple[str, str, FixtureMutation]] = []
        cases.append((
            "format-drift", "unsupported M1 foundation-mutation registry",
            lambda fixture: mutate_registry(
                fixture, lambda lines: lines.__setitem__(0, "format=FERRIC-M1-NEGATIVE-FOUNDATIONS-V0")
            ),
        ))
        cases.append((
            "row-omission", "M1 foundation mutation roster drifted",
            lambda fixture: mutate_registry(fixture, lambda lines: lines.pop()),
        ))
        cases.append((
            "unknown-row", "unknown M1 foundation mutation",
            lambda fixture: mutate_registry(
                fixture,
                lambda lines: lines.append(
                    lines[1]
                    .replace("artifact-manifest-commitment-digest", "unknown-row")
                    .replace("canonical-manifest-digest-binding", "unknown-clause")
                ),
            ),
        ))
        cases.append((
            "duplicate-row", "duplicate M1 foundation mutation",
            lambda fixture: mutate_registry(fixture, lambda lines: lines.append(lines[1])),
        ))
        cases.append((
            "row-order", "registry is not sorted",
            lambda fixture: mutate_registry(
                fixture, lambda lines: lines.__setitem__(slice(1, 3), [lines[2], lines[1]])
            ),
        ))
        cases.append((
            "field-omission", "malformed M1 foundation-mutation record",
            lambda fixture: mutate_registry(
                fixture, lambda lines: lines.__setitem__(1, lines[1].rsplit("|", 1)[0])
            ),
        ))
        for name, field, value in (
            ("foundation-drift", 1, "wrong-foundation"),
            ("property-drift", 2, "kv_refined"),
            ("path-drift", 3, "kv-proof"),
            ("package-drift", 4, "ferric-engine"),
            ("source-drift", 5, "crates/ferric-spec/src/graph.rs"),
            ("mutator-drift", 6, "graph-operator-order.py"),
            ("marker-drift", 7, "accepted"),
            ("module-drift", 8, "ferric_spec::graph"),
            ("function-drift", 9, "expected_step"),
            ("clause-drift", 10, "weakened-clause"),
        ):
            expected = (
                "unknown M1 proof-failure marker"
                if name == "marker-drift"
                else "M1 foundation mutation binding drifted"
            )
            cases.append((
                name,
                expected,
                lambda fixture, field=field, value=value: mutate_registry(
                    fixture, lambda lines: replace_field(lines, 1, field, value)
                ),
            ))
        cases.append((
            "speculative-binding-drift", "M1 foundation mutation binding drifted",
            lambda fixture: mutate_registry(
                fixture,
                lambda lines: replace_field(
                    lines,
                    registry_row(lines, "speculative-accepted-count-binding"),
                    10,
                    "wrong-accepted-count-clause",
                ),
            ),
        ))
        cases.append((
            "sampler-binding-drift", "M1 foundation mutation binding drifted",
            lambda fixture: mutate_registry(
                fixture,
                lambda lines: replace_field(
                    lines,
                    registry_row(lines, "sampler-lowest-id-publication"),
                    10,
                    "last-token-id-tie-breaking",
                ),
            ),
        ))
        cases.append((
            "lifetime-binding-drift", "M1 foundation mutation binding drifted",
            lambda fixture: mutate_registry(
                fixture,
                lambda lines: replace_field(
                    lines,
                    registry_row(lines, "kv-terminal-release-exact-epoch"),
                    10,
                    "wrong-exact-quiescent-epoch-clause",
                ),
            ),
        ))
        cases.append((
            "target-binding-drift", "M1 foundation mutation binding drifted",
            lambda fixture: mutate_registry(
                fixture,
                lambda lines: replace_field(
                    lines,
                    registry_row(lines, "target-catalog-processor-features"),
                    10,
                    "wrong-target-clause",
                ),
            ),
        ))
        cases.append((
            "operator-binding-drift", "M1 foundation mutation binding drifted",
            lambda fixture: mutate_registry(
                fixture,
                lambda lines: replace_field(
                    lines,
                    registry_row(lines, "operator-declared-profile-effect"),
                    9,
                    "bind_declared_operation_kernel_plan",
                ),
            ),
        ))
        cases.append((
            "unsafe-source", "unsafe foundation source path",
            lambda fixture: mutate_registry(
                fixture, lambda lines: replace_field(lines, 1, 5, "../continuous_batching.rs")
            ),
        ))
        cases.append((
            "missing-mutator", "M1 foundation mutator is unavailable",
            lambda fixture: (fixture / "proofs/m1/negative/components/batching-publish-once.py").unlink(),
        ))
        cases.append((
            "missing-speculative-mutator", "M1 foundation mutator is unavailable",
            lambda fixture: (
                fixture
                / "proofs/m1/negative/components/speculative-accepted-count-binding.py"
            ).unlink(),
        ))
        cases.append((
            "missing-sampler-mutator", "M1 foundation mutator is unavailable",
            lambda fixture: (
                fixture
                / "proofs/m1/negative/components/sampler-lowest-id-publication.py"
            ).unlink(),
        ))
        cases.append((
            "missing-lifetime-mutator", "M1 foundation mutator is unavailable",
            lambda fixture: (
                fixture
                / "proofs/m1/negative/components/kv-terminal-release-exact-epoch.py"
            ).unlink(),
        ))
        cases.append((
            "missing-model-bundle-mutator", "M1 foundation mutator is unavailable",
            lambda fixture: (
                fixture
                / "proofs/m1/negative/components/model-bundle-record-binding.py"
            ).unlink(),
        ))
        cases.append((
            "missing-artifact-auth-mutator", "M1 foundation mutator is unavailable",
            lambda fixture: (
                fixture
                / "proofs/m1/negative/components/artifact-manifest-commitment-digest.py"
            ).unlink(),
        ))
        cases.append((
            "missing-target-mutator", "M1 foundation mutator is unavailable",
            lambda fixture: (
                fixture
                / "proofs/m1/negative/components/target-catalog-processor-features.py"
            ).unlink(),
        ))
        cases.append((
            "missing-operator-mutator", "M1 foundation mutator is unavailable",
            lambda fixture: (
                fixture
                / "proofs/m1/negative/components/operator-declared-profile-effect.py"
            ).unlink(),
        ))
        cases.append((
            "missing-source", "M1 foundation source is unavailable",
            lambda fixture: (fixture / "crates/ferric-spec/src/continuous_batching.rs").unlink(),
        ))
        cases.append((
            "missing-speculative-source", "M1 foundation source is unavailable",
            lambda fixture: (
                fixture / "crates/ferric-spec/src/speculative_step_composition.rs"
            ).unlink(),
        ))
        cases.append((
            "missing-sampler-source", "M1 foundation source is unavailable",
            lambda fixture: (
                fixture / "crates/ferric-spec/src/m1_completion.rs"
            ).unlink(),
        ))
        cases.append((
            "missing-lifetime-source", "M1 foundation source is unavailable",
            lambda fixture: (
                fixture / "crates/ferric-spec/src/request_isolation.rs"
            ).unlink(),
        ))
        cases.append((
            "missing-model-bundle-source", "M1 foundation source is unavailable",
            lambda fixture: (fixture / "crates/ferric-build/src/auth.rs").unlink(),
        ))
        cases.append((
            "missing-target-source", "M1 foundation source is unavailable",
            lambda fixture: (fixture / "crates/ferric-kernels/src/validation.rs").unlink(),
        ))
        cases.append((
            "missing-operator-source", "M1 foundation source is unavailable",
            lambda fixture: (
                fixture / "crates/ferric-engine/src/operation_kernel_plan.rs"
            ).unlink(),
        ))
        cases.append((
            "missing-module-record", "compiler module path is not inventoried",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "module=ferric-spec|crates/ferric-spec/src/continuous_batching.rs|ferric_spec::continuous_batching"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-function-record", "compiler function path is not directly verified",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "verified=ferric-spec|crates/ferric-spec/src/continuous_batching.rs|ferric_spec::continuous_batching::apply_continuous_publish_step"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-model-bundle-module-record",
            "compiler module path is not inventoried",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "module=ferric-build|crates/ferric-build/src/auth.rs|ferric_build::auth"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-model-bundle-function-record",
            "compiler function path is not directly verified",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "verified=ferric-build|crates/ferric-build/src/auth.rs|ferric_build::auth::admission_records_equal"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-artifact-auth-function-record",
            "compiler function path is not directly verified",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "verified=ferric-build|crates/ferric-build/src/auth.rs|ferric_build::auth::validate_manifest_commitment_verified"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-target-module-record",
            "compiler module path is not inventoried",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "module=ferric-kernels|crates/ferric-kernels/src/validation.rs|ferric_kernels::validation"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-target-function-record",
            "compiler function path is not directly verified",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "verified=ferric-kernels|crates/ferric-kernels/src/validation.rs|ferric_kernels::validation::validate_kernel_catalog_input"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-operator-function-record",
            "compiler function path is not directly verified",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "verified=ferric-engine|crates/ferric-engine/src/operation_kernel_plan.rs|ferric_engine::operation_kernel_plan::select_declared_operator_certificate"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-speculative-function-record",
            "compiler function path is not directly verified",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "verified=ferric-spec|crates/ferric-spec/src/speculative_step_composition.rs|ferric_spec::speculative_step_composition::settle_and_publish_speculative_step"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-sampler-function-record",
            "compiler function path is not directly verified",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "verified=ferric-spec|crates/ferric-spec/src/m1_completion.rs|ferric_spec::m1_completion::select_lowest_argmax"
                ) + "\n",
                encoding="utf-8",
            ),
        ))
        cases.append((
            "missing-lifetime-function-record",
            "compiler function path is not directly verified",
            lambda fixture: (fixture / "proofs/VERIFIED_MODULES").write_text(
                "\n".join(
                    line for line in (fixture / "proofs/VERIFIED_MODULES").read_text(encoding="utf-8").splitlines()
                    if line != "verified=ferric-spec|crates/ferric-spec/src/request_isolation.rs|ferric_spec::request_isolation::release_isolated_page"
                ) + "\n",
                encoding="utf-8",
            ),
        ))

        def close_property(fixture: Path) -> None:
            path = fixture / "proofs/M1_REQUIREMENTS.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            next(row for row in value["assurance_properties"] if row["name"] == "scheduler_refined")["obligation_state"] = "Proved"
            write_canonical(path, value)

        cases.append(("premature-property-closure", "must remain Open", close_property))

        def close_speculative_property(fixture: Path) -> None:
            path = fixture / "proofs/M1_REQUIREMENTS.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            next(
                row
                for row in value["assurance_properties"]
                if row["name"] == "rollback_refined"
            )["obligation_state"] = "Proved"
            write_canonical(path, value)

        cases.append((
            "premature-speculative-property-closure",
            "must remain Open",
            close_speculative_property,
        ))

        for name, expected, mutation in cases:
            expect_rejected(repo, root, name, expected, mutation)

        existing = root / "existing-output"
        copy_fixture(repo, existing)
        output = existing / "active"
        output.write_text("occupied\n", encoding="utf-8")
        result = run_checker(existing, output)
        if result.returncode == 0 or "output already exists" not in result.stdout:
            fail("pre-existing active output was not rejected")

    print(
        f"PASS: M1 foundation-mutation policy ({len(cases) + 1} hostile fixtures, "
        f"{mutator_count} exact current anchors)"
    )


if __name__ == "__main__":
    main()
