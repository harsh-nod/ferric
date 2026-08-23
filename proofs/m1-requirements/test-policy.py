#!/usr/bin/env python3
"""Exercise hostile mutations against the M1 requirements checker."""

from __future__ import annotations

import copy
import json
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Callable
from pathlib import Path
from typing import Any, NoReturn


JsonObject = dict[str, Any]
JsonMutation = Callable[[JsonObject], None]
FixtureMutation = Callable[[Path], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def write_canonical(path: Path, value: JsonObject) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def manifest_mutation(mutation: JsonMutation) -> FixtureMutation:
    def apply(fixture: Path) -> None:
        path = fixture / "proofs/M1_REQUIREMENTS.json"
        value = json.loads(path.read_text(encoding="utf-8"))
        mutation(value)
        write_canonical(path, value)

    return apply


def requirement(value: JsonObject, identifier: str) -> JsonObject:
    return next(
        record for record in value["roadmap_requirements"] if record["id"] == identifier
    )


def property_record(value: JsonObject, name: str) -> JsonObject:
    return next(
        record for record in value["assurance_properties"] if record["name"] == name
    )


def path_record(value: JsonObject, identifier: str) -> JsonObject:
    return next(
        record for record in value["path_obligations"] if record["id"] == identifier
    )


def profile(value: JsonObject, identifier: str) -> JsonObject:
    return next(
        record for record in value["evidence_profiles"] if record["id"] == identifier
    )


def binding_classes(value: JsonObject, kind: str) -> JsonObject:
    return next(
        record
        for record in value["evidence_kind_binding_classes"]
        if record["kind"] == kind
    )


def copy_fixture(repo: Path, root: Path, name: str) -> Path:
    fixture = root / name
    (fixture / "docs").mkdir(parents=True)
    (fixture / "proofs").mkdir()
    shutil.copy2(repo / "docs/ROADMAP.md", fixture / "docs/ROADMAP.md")
    shutil.copy2(
        repo / "docs/M1_PROPERTY_CONTRACT.md",
        fixture / "docs/M1_PROPERTY_CONTRACT.md",
    )
    shutil.copy2(
        repo / "proofs/M1_REQUIREMENTS.json",
        fixture / "proofs/M1_REQUIREMENTS.json",
    )
    return fixture


def expect_rejected(
    checker: Path,
    repo: Path,
    root: Path,
    name: str,
    expected: str,
    mutation: FixtureMutation,
) -> None:
    fixture = copy_fixture(repo, root, name)
    mutation(fixture)
    result = subprocess.run(
        [sys.executable, "-I", str(checker), str(fixture)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if result.returncode == 0 or expected not in result.stdout:
        fail(
            f"{name} was not rejected with {expected!r} "
            f"(status={result.returncode})\n{result.stdout}"
        )


def main() -> None:
    if len(sys.argv) != 2:
        fail(f"usage: {sys.argv[0]} REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    checker = repo / "proofs/check-m1-requirements.py"
    baseline = subprocess.run(
        [sys.executable, "-I", str(checker), str(repo)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if baseline.returncode != 0:
        fail(f"baseline M1 requirements check failed\n{baseline.stdout}")

    cases: list[tuple[str, str, FixtureMutation]] = []

    def remove_requirement(value: JsonObject) -> None:
        value["roadmap_requirements"].pop()

    cases.append(
        (
            "requirement-omission",
            "M1 roadmap requirement roster has 32 records, expected 33",
            manifest_mutation(remove_requirement),
        )
    )

    def duplicate_requirement(value: JsonObject) -> None:
        value["roadmap_requirements"].append(
            copy.deepcopy(value["roadmap_requirements"][-1])
        )

    cases.append(
        (
            "requirement-duplicate",
            "M1 roadmap requirement roster has 34 records, expected 33",
            manifest_mutation(duplicate_requirement),
        )
    )

    def close_requirement(value: JsonObject) -> None:
        requirement(value, "m1.r01")["obligation_state"] = "Closed"

    cases.append(
        (
            "requirement-closed",
            "M1 roadmap requirement must remain Open: m1.r01",
            manifest_mutation(close_requirement),
        )
    )

    def unknown_property(value: JsonObject) -> None:
        requirement(value, "m1.r01")["assurance_properties"].append("unknown")

    cases.append(
        (
            "unknown-property-reference",
            "M1 roadmap requirement references an unknown property: m1.r01",
            manifest_mutation(unknown_property),
        )
    )

    def duplicate_property_reference(value: JsonObject) -> None:
        properties = requirement(value, "m1.r01")["assurance_properties"]
        properties.append(properties[0])

    cases.append(
        (
            "duplicate-property-reference",
            "requirement m1.r01 properties contains a duplicate reference",
            manifest_mutation(duplicate_property_reference),
        )
    )

    def unknown_path(value: JsonObject) -> None:
        requirement(value, "m1.r01")["path_obligations"].append("unknown-path")

    cases.append(
        (
            "unknown-path-reference",
            "M1 roadmap requirement references an unknown path obligation: m1.r01",
            manifest_mutation(unknown_path),
        )
    )

    def duplicate_path_reference(value: JsonObject) -> None:
        paths = requirement(value, "m1.r01")["path_obligations"]
        paths.append(paths[0])

    cases.append(
        (
            "duplicate-path-reference",
            "requirement m1.r01 path obligations contains a duplicate reference",
            manifest_mutation(duplicate_path_reference),
        )
    )

    def unknown_profile(value: JsonObject) -> None:
        requirement(value, "m1.r01")["evidence_profiles"].append("unknown")

    cases.append(
        (
            "unknown-evidence-profile",
            "M1 roadmap requirement references an unknown evidence profile: m1.r01",
            manifest_mutation(unknown_profile),
        )
    )

    def unknown_evidence_kind(value: JsonObject) -> None:
        profile(value, "admission")["kinds"].append("unknown-evidence")

    cases.append(
        (
            "unknown-evidence-kind",
            "M1 evidence profile references an unknown evidence kind: admission",
            manifest_mutation(unknown_evidence_kind),
        )
    )

    cases.append(
        (
            "binding-class-roster-omission",
            "M1 evidence-kind binding-class roster is incomplete",
            manifest_mutation(
                lambda value: value["evidence_kind_binding_classes"].pop()
            ),
        )
    )
    cases.append(
        (
            "Assurance-only-binding-class-widening",
            "M1 evidence-kind binding-class roster drifted",
            manifest_mutation(
                lambda value: binding_classes(value, "negative-mutation")[
                    "classes"
                ].append("Roadmap")
            ),
        )
    )
    cases.append(
        (
            "global-kind-binding-class-injection",
            "global M1 evidence kind has an obligation binding class",
            manifest_mutation(
                lambda value: binding_classes(value, "tcb-report")["classes"].append(
                    "Assurance"
                )
            ),
        )
    )
    cases.append(
        (
            "non-global-kind-empty-binding-class",
            "non-global M1 evidence kind has no binding class",
            manifest_mutation(
                lambda value: binding_classes(value, "artifact-identity").__setitem__(
                    "classes", []
                )
            ),
        )
    )

    def weaken_status(value: JsonObject) -> None:
        property_record(value, "model_bundle_well_formed")[
            "required_status_at_closure"
        ] = "Checked"

    cases.append(
        (
            "weakened-status",
            "M1 assurance property roster drifted or status weakened: model_bundle_well_formed",
            manifest_mutation(weaken_status),
        )
    )

    def unknown_status(value: JsonObject) -> None:
        property_record(value, "target_conforming")["required_status_at_closure"] = (
            "Unknown"
        )

    cases.append(
        (
            "unknown-status",
            "M1 assurance property roster drifted or status weakened: target_conforming",
            manifest_mutation(unknown_status),
        )
    )

    def duplicate_property_record(value: JsonObject) -> None:
        value["assurance_properties"][-1] = copy.deepcopy(
            value["assurance_properties"][0]
        )

    cases.append(
        (
            "duplicate-property-record",
            "duplicate M1 assurance property name: model_bundle_well_formed",
            manifest_mutation(duplicate_property_record),
        )
    )

    def duplicate_path_record(value: JsonObject) -> None:
        value["path_obligations"][-1] = copy.deepcopy(value["path_obligations"][0])

    cases.append(
        (
            "duplicate-path-record",
            "duplicate M1 path obligation id: adversarial-bench",
            manifest_mutation(duplicate_path_record),
        )
    )

    def close_property(value: JsonObject) -> None:
        property_record(value, "artifact_authenticated")["obligation_state"] = (
            "Validated"
        )

    cases.append(
        (
            "property-closed",
            "M1 assurance property must remain Open: artifact_authenticated",
            manifest_mutation(close_property),
        )
    )

    def drift_boundary(value: JsonObject) -> None:
        property_record(value, "machine_refined")["boundary"] += " Altered."

    cases.append(
        (
            "property-boundary-drift",
            "M1 assurance property roster drifted or status weakened: machine_refined",
            manifest_mutation(drift_boundary),
        )
    )

    def close_path(value: JsonObject) -> None:
        path_record(value, "bundle-parser")["obligation_state"] = "Implemented"

    cases.append(
        (
            "path-closed",
            "M1 path obligation must remain Open: bundle-parser",
            manifest_mutation(close_path),
        )
    )

    def drift_path(value: JsonObject) -> None:
        path_record(value, "bundle-parser")["path"] = "../outside.rs"

    cases.append(
        (
            "path-target-drift",
            "unsafe M1 path obligation bundle-parser",
            manifest_mutation(drift_path),
        )
    )

    def inject_evidence(value: JsonObject) -> None:
        requirement(value, "m1.r01")["evidence"] = []

    cases.append(
        (
            "evidence-injection",
            "M1 requirements must not contain evidence or closure fields",
            manifest_mutation(inject_evidence),
        )
    )

    def inject_receipt(value: JsonObject) -> None:
        value["receipt"] = "forged"

    cases.append(
        (
            "receipt-injection",
            "M1 requirements must not contain evidence or closure fields",
            manifest_mutation(inject_receipt),
        )
    )

    def drift_m0_contracts(value: JsonObject) -> None:
        value["m0_contracts_commit"] = "0" * 40

    cases.append(
        (
            "m0-contract-pin-drift",
            "M1 inherited M0 proof-contract pin drifted",
            manifest_mutation(drift_m0_contracts),
        )
    )

    def drift_m1_upstream(value: JsonObject) -> None:
        value["m1_upstream_base_commit"] = "0" * 40

    cases.append(
        (
            "m1-upstream-pin-drift",
            "M1 fe2o3 upstream base commit drifted",
            manifest_mutation(drift_m1_upstream),
        )
    )

    def drift_m1_upstream_tree(value: JsonObject) -> None:
        value["m1_upstream_base_tree"] = "0" * 40

    cases.append(
        (
            "m1-upstream-tree-drift",
            "M1 fe2o3 upstream base tree drifted",
            manifest_mutation(drift_m1_upstream_tree),
        )
    )

    def claim_future_path_exists(value: JsonObject) -> None:
        path_record(value, "fe2o3-batch")["availability"] = "ExistingFoundation"

    cases.append(
        (
            "future-path-existence-claim",
            "M1 path obligation availability drifted: fe2o3-batch",
            manifest_mutation(claim_future_path_exists),
        )
    )

    def duplicate_json_key(fixture: Path) -> None:
        path = fixture / "proofs/M1_REQUIREMENTS.json"
        source = path.read_text(encoding="utf-8")
        needle = '  "format": "ferric.m1-requirements.v1",\n'
        if source.count(needle) != 1:
            fail("duplicate-key fixture could not find format field")
        path.write_text(source.replace(needle, needle + needle, 1), encoding="utf-8")

    cases.append(
        (
            "duplicate-json-key",
            "duplicate JSON key: format",
            duplicate_json_key,
        )
    )

    def noncanonical_json(fixture: Path) -> None:
        path = fixture / "proofs/M1_REQUIREMENTS.json"
        path.write_text(path.read_text(encoding="utf-8") + "\n", encoding="utf-8")

    cases.append(
        (
            "noncanonical-json",
            "M1 requirements manifest is not canonical JSON",
            noncanonical_json,
        )
    )

    def close_roadmap(fixture: Path) -> None:
        path = fixture / "docs/ROADMAP.md"
        source = path.read_text(encoding="utf-8")
        needle = "- [ ] Define bounded canonical Ferric deployment bundles."
        if source.count(needle) != 1:
            fail("roadmap fixture could not find M1 requirement")
        path.write_text(
            source.replace(needle, needle.replace("[ ]", "[x]")), encoding="utf-8"
        )

    cases.append(
        (
            "roadmap-premature-closure",
            "M1 roadmap checklist drifted or contains a closed requirement",
            close_roadmap,
        )
    )

    def weaken_documentation(fixture: Path) -> None:
        path = fixture / "docs/M1_PROPERTY_CONTRACT.md"
        source = path.read_text(encoding="utf-8")
        needle = "| `model_bundle_well_formed` | `Extension:model_bundle_well_formed` | `Proved` | `Open` |"
        if source.count(needle) != 1:
            fail("documentation fixture could not find assurance row")
        path.write_text(
            source.replace(needle, needle.replace("`Proved`", "`Checked`")),
            encoding="utf-8",
        )

    cases.append(
        (
            "documentation-status-weakening",
            "M1 assurance documentation contains a weakened or closed row: model_bundle_well_formed",
            weaken_documentation,
        )
    )

    with tempfile.TemporaryDirectory(
        prefix="ferric-m1-requirements-policy."
    ) as scratch:
        root = Path(scratch)
        for name, expected, mutation in cases:
            expect_rejected(checker, repo, root, name, expected, mutation)

    print(f"PASS: M1 hostile requirements policy ({len(cases)} fixtures)")


if __name__ == "__main__":
    main()
