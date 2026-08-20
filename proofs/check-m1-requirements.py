#!/usr/bin/env python3
"""Validate the canonical, requirements-only M1 evidence scaffold."""

from __future__ import annotations

import hashlib
import json
import re
import stat
import sys
from pathlib import Path
from typing import Any, NoReturn


FORMAT = "ferric.m1-requirements.v1"
M0_CONTRACTS_COMMIT = "a6fa86b5ccf8f0438925cfec8f48a5d713874da3"
M1_UPSTREAM_BASE_COMMIT = "5d095d5663f7d158385603f867f001d1eb22d539"
M1_UPSTREAM_BASE_TREE = "f6a187be6365fb8e2cb12671d163cee41af3b24f"
OPEN = "Open"
M1_REQUIREMENT_COUNT = 33

EVIDENCE_KINDS = (
    "artifact-identity",
    "canonical-structure-check",
    "external-contract",
    "fe2o3-contract",
    "hardware-test",
    "independent-validator",
    "negative-mutation",
    "performance-gate",
    "tcb-report",
    "unsupported-rationale",
    "verus-theorem",
)
EVIDENCE_PROFILE_IDS = (
    "admission",
    "authentication",
    "composition",
    "kernel",
    "nonclaim",
    "qualification",
    "runtime",
)
EXISTING_FOUNDATIONS = {
    "fe2o3-aql-foundation": "crates/fe2o3-aql/src/lib.rs",
    "fe2o3-kfd-memory-foundation": "crates/fe2o3-kfd/src/memory.rs",
    "fe2o3-kfd-queue-foundation": "crates/fe2o3-kfd/src/queue_submit.rs",
}
REQUIRED_FUTURE_TARGETS = {
    "fe2o3-batch": ("fe2o3", "crates/fe2o3-service-host/src/batch.rs"),
}
ALLOWED_CLOSURE_STATUSES = {"Proved", "Unsupported", "Validated"}
ALLOWED_AVAILABILITY = {"ExistingFoundation", "RequiredFuture"}
ALLOWED_REPOSITORIES = {"fe2o3", "ferric"}
FORBIDDEN_KEYS = {"actual_status", "evidence", "receipt", "satisfaction"}
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_PROPERTY = re.compile(r"[a-z][a-z0-9_]*\Z")
SAFE_PATH = re.compile(r"[A-Za-z0-9_./-]+\Z")


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def regular_file(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {path}: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"{description} must be a regular file: {path}")


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def load_manifest(path: Path) -> dict[str, Any]:
    regular_file(path, "M1 requirements manifest")
    try:
        source = path.read_text(encoding="utf-8")
        value = json.loads(source, object_pairs_hook=unique_object)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse M1 requirements manifest: {error}")
    if not isinstance(value, dict):
        fail("M1 requirements manifest must be an object")
    canonical = json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    if source != canonical:
        fail("M1 requirements manifest is not canonical JSON")
    return value


def exact_keys(value: dict[str, Any], expected: set[str], description: str) -> None:
    if set(value) != expected:
        fail(
            f"{description} has unexpected keys "
            f"(missing={sorted(expected - set(value))}, extra={sorted(set(value) - expected)})"
        )


def string_list(value: Any, description: str) -> tuple[str, ...]:
    if (
        not isinstance(value, list)
        or not value
        or not all(isinstance(item, str) for item in value)
    ):
        fail(f"{description} must be a nonempty string array")
    if len(value) != len(set(value)):
        fail(f"{description} contains a duplicate reference")
    return tuple(value)


def safe_path(value: str, description: str) -> None:
    path = Path(value)
    if not SAFE_PATH.fullmatch(value) or path.is_absolute() or ".." in path.parts:
        fail(f"unsafe {description}: {value!r}")


def unquote(value: str) -> str:
    if len(value) >= 2 and value.startswith("`") and value.endswith("`"):
        return value[1:-1]
    return value


def documentation_roster(repo: Path) -> list[tuple[str, str, str, str, str]]:
    path = repo / "docs/M1_PROPERTY_CONTRACT.md"
    regular_file(path, "M1 property contract")
    try:
        source = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        fail(f"cannot read M1 property contract: {error}")
    begin = "<!-- BEGIN M1 ASSURANCE ROSTER -->"
    end = "<!-- END M1 ASSURANCE ROSTER -->"
    if source.count(begin) != 1 or source.count(end) != 1:
        fail("M1 property contract has no unique assurance roster")
    table = source.split(begin, 1)[1].split(end, 1)[0]
    rows: list[tuple[str, str, str, str, str]] = []
    for line in table.splitlines():
        if not line.startswith("| `"):
            continue
        fields = [field.strip() for field in line.strip().strip("|").split("|")]
        if len(fields) != 5:
            fail(f"malformed M1 assurance documentation row: {line}")
        name, kind, status, state = (unquote(field) for field in fields[:4])
        boundary = fields[4]
        if not SAFE_PROPERTY.fullmatch(name) or not boundary:
            fail(f"malformed M1 assurance documentation row: {line}")
        if status not in ALLOWED_CLOSURE_STATUSES or state != OPEN:
            fail(
                f"M1 assurance documentation contains a weakened or closed row: {name}"
            )
        rows.append((name, kind, status, state, boundary))
    names = [name for name, *_rest in rows]
    if not rows or len(names) != len(set(names)):
        fail("M1 property contract contains a missing or duplicate assurance row")
    normalized = " ".join(source.split())
    required_text = (
        "No M1 evidence is recorded by this document or manifest.",
        "Every M1 implementation obligation remains `Open`.",
        "`machine_refined` remains an `Unsupported` target status",
    )
    for text in required_text:
        if text not in normalized:
            fail(f"M1 property contract is missing required non-claim: {text}")
    return rows


def roadmap_titles(repo: Path) -> tuple[str, ...]:
    path = repo / "docs/ROADMAP.md"
    regular_file(path, "roadmap")
    try:
        source = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        fail(f"cannot read roadmap: {error}")
    try:
        m1 = source.split("## M1:", 1)[1].split("## M2:", 1)[0]
    except IndexError:
        fail("roadmap has no unique M1 section")
    rows = tuple(re.findall(r"^- \[ \] (.+)$", m1, flags=re.MULTILINE))
    if re.search(r"^- \[[xX]\] ", m1, flags=re.MULTILINE):
        fail("M1 roadmap checklist drifted or contains a closed requirement")
    if len(rows) != M1_REQUIREMENT_COUNT:
        fail(
            f"M1 roadmap has {len(rows)} open requirements, expected {M1_REQUIREMENT_COUNT}"
        )
    return rows


def reject_evidence(value: Any, location: str = "manifest") -> None:
    if isinstance(value, dict):
        forbidden = FORBIDDEN_KEYS & set(value)
        if forbidden:
            fail(
                f"M1 requirements must not contain evidence or closure fields at {location}: {sorted(forbidden)}"
            )
        for key, item in value.items():
            reject_evidence(item, f"{location}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            reject_evidence(item, f"{location}[{index}]")


def validate_manifest(
    value: dict[str, Any],
    documented: list[tuple[str, str, str, str, str]],
    roadmap: tuple[str, ...],
) -> None:
    exact_keys(
        value,
        {
            "assurance_properties",
            "evidence_kinds",
            "evidence_profiles",
            "format",
            "m0_contracts_commit",
            "m1_upstream_base_commit",
            "m1_upstream_base_tree",
            "milestone",
            "path_obligations",
            "roadmap_requirements",
        },
        "M1 requirements manifest",
    )
    if value["format"] != FORMAT or value["milestone"] != "M1":
        fail("M1 requirements manifest authority identity drifted")
    if value["m0_contracts_commit"] != M0_CONTRACTS_COMMIT:
        fail("M1 inherited M0 proof-contract pin drifted")
    if value["m1_upstream_base_commit"] != M1_UPSTREAM_BASE_COMMIT:
        fail("M1 fe2o3 upstream base commit drifted")
    if value["m1_upstream_base_tree"] != M1_UPSTREAM_BASE_TREE:
        fail("M1 fe2o3 upstream base tree drifted")
    if value["evidence_kinds"] != list(EVIDENCE_KINDS):
        fail("M1 evidence-kind roster drifted")

    profiles = value["evidence_profiles"]
    if not isinstance(profiles, list):
        fail("M1 evidence-profile roster must be an array")
    profile_ids: set[str] = set()
    used_evidence_kinds: set[str] = set()
    for record in profiles:
        if not isinstance(record, dict):
            fail("M1 evidence-profile record must be an object")
        exact_keys(record, {"id", "kinds"}, "M1 evidence profile")
        identifier = record["id"]
        if not isinstance(identifier, str) or not SAFE_ID.fullmatch(identifier):
            fail(f"unsafe M1 evidence profile id: {identifier!r}")
        if identifier in profile_ids:
            fail(f"duplicate M1 evidence profile id: {identifier}")
        kinds = string_list(record["kinds"], f"M1 evidence profile {identifier} kinds")
        if not set(kinds) <= set(EVIDENCE_KINDS):
            fail(
                f"M1 evidence profile references an unknown evidence kind: {identifier}"
            )
        profile_ids.add(identifier)
        used_evidence_kinds.update(kinds)
    if tuple(record["id"] for record in profiles) != EVIDENCE_PROFILE_IDS:
        fail("M1 evidence-profile roster drifted")
    if used_evidence_kinds != set(EVIDENCE_KINDS):
        fail("not every M1 evidence kind resolves through an evidence profile")

    paths = value["path_obligations"]
    if not isinstance(paths, list) or not paths:
        fail("M1 path obligation roster must be a nonempty array")
    path_ids: set[str] = set()
    existing_foundations: dict[str, str] = {}
    for record in paths:
        if not isinstance(record, dict):
            fail("M1 path obligation must be an object")
        exact_keys(
            record,
            {"availability", "id", "obligation_state", "path", "repository"},
            "M1 path obligation",
        )
        identifier = record["id"]
        if not isinstance(identifier, str) or not SAFE_ID.fullmatch(identifier):
            fail(f"unsafe M1 path obligation id: {identifier!r}")
        if identifier in path_ids:
            fail(f"duplicate M1 path obligation id: {identifier}")
        if record["obligation_state"] != OPEN:
            fail(f"M1 path obligation must remain Open: {identifier}")
        if record["repository"] not in ALLOWED_REPOSITORIES:
            fail(f"M1 path obligation has an unknown repository: {identifier}")
        if record["availability"] not in ALLOWED_AVAILABILITY:
            fail(f"M1 path obligation has an unknown availability: {identifier}")
        safe_path(record["path"], f"M1 path obligation {identifier}")
        if record["availability"] == "ExistingFoundation":
            if record["repository"] != "fe2o3":
                fail(f"M1 existing foundation must be owned by fe2o3: {identifier}")
            existing_foundations[identifier] = record["path"]
        path_ids.add(identifier)
    if tuple(record["id"] for record in paths) != tuple(sorted(path_ids)):
        fail("M1 path obligation roster is not canonically ordered")
    by_path_id = {record["id"]: record for record in paths}
    for identifier, (repository, path) in REQUIRED_FUTURE_TARGETS.items():
        record = by_path_id.get(identifier)
        if (
            record is None
            or record["repository"] != repository
            or record["path"] != path
            or record["availability"] != "RequiredFuture"
        ):
            fail(f"M1 path obligation availability drifted: {identifier}")
    if existing_foundations != EXISTING_FOUNDATIONS:
        fail("M1 existing fe2o3 foundation roster drifted")

    properties = value["assurance_properties"]
    if not isinstance(properties, list) or len(properties) != len(documented):
        fail(
            f"M1 assurance roster has {len(properties) if isinstance(properties, list) else 'invalid'} records, expected {len(documented)}"
        )
    property_ids: set[str] = set()
    used_profiles: set[str] = set()
    used_paths: set[str] = set()
    for record, expected in zip(properties, documented, strict=True):
        if not isinstance(record, dict):
            fail("M1 assurance property must be an object")
        exact_keys(
            record,
            {
                "boundary",
                "evidence_profiles",
                "fe2o3_kind",
                "name",
                "obligation_state",
                "path_obligations",
                "required_status_at_closure",
            },
            "M1 assurance property",
        )
        name = record["name"]
        if name in property_ids:
            fail(f"duplicate M1 assurance property name: {name}")
        profiles_ref = string_list(
            record["evidence_profiles"], f"property {name} evidence profiles"
        )
        paths_ref = string_list(
            record["path_obligations"], f"property {name} path obligations"
        )
        if record["obligation_state"] != OPEN:
            fail(f"M1 assurance property must remain Open: {name}")
        actual = (
            name,
            record["fe2o3_kind"],
            record["required_status_at_closure"],
            record["obligation_state"],
            record["boundary"],
        )
        if actual != expected:
            fail(
                f"M1 assurance property roster drifted or status weakened: {expected[0]}"
            )
        if not set(profiles_ref) <= profile_ids:
            fail(
                f"M1 assurance property references an unknown evidence profile: {name}"
            )
        if not set(paths_ref) <= path_ids:
            fail(f"M1 assurance property references an unknown path obligation: {name}")
        property_ids.add(name)
        used_profiles.update(profiles_ref)
        used_paths.update(paths_ref)

    requirements = value["roadmap_requirements"]
    if not isinstance(requirements, list) or len(requirements) != len(roadmap):
        fail(
            f"M1 roadmap requirement roster has {len(requirements) if isinstance(requirements, list) else 'invalid'} records, expected {len(roadmap)}"
        )
    used_properties: set[str] = set()
    for index, (record, expected_title) in enumerate(
        zip(requirements, roadmap, strict=True), start=1
    ):
        expected_id = f"m1.r{index:02d}"
        if not isinstance(record, dict):
            fail("M1 roadmap requirement must be an object")
        exact_keys(
            record,
            {
                "assurance_properties",
                "evidence_profiles",
                "id",
                "obligation_state",
                "path_obligations",
                "title",
            },
            f"M1 roadmap requirement {expected_id}",
        )
        property_ref = string_list(
            record["assurance_properties"], f"requirement {expected_id} properties"
        )
        profile_ref = string_list(
            record["evidence_profiles"], f"requirement {expected_id} evidence profiles"
        )
        path_ref = string_list(
            record["path_obligations"], f"requirement {expected_id} path obligations"
        )
        if record["id"] != expected_id or record["title"] != expected_title:
            fail(f"M1 roadmap requirement identity or title drifted: {expected_id}")
        if record["obligation_state"] != OPEN:
            fail(f"M1 roadmap requirement must remain Open: {expected_id}")
        if not set(property_ref) <= property_ids:
            fail(
                f"M1 roadmap requirement references an unknown property: {expected_id}"
            )
        if not set(profile_ref) <= profile_ids:
            fail(
                f"M1 roadmap requirement references an unknown evidence profile: {expected_id}"
            )
        if not set(path_ref) <= path_ids:
            fail(
                f"M1 roadmap requirement references an unknown path obligation: {expected_id}"
            )
        used_properties.update(property_ref)
        used_profiles.update(profile_ref)
        used_paths.update(path_ref)

    if used_properties != property_ids:
        fail("not every M1 assurance property resolves from a roadmap requirement")
    if used_profiles != profile_ids:
        fail("not every M1 evidence profile resolves from an open obligation")
    if used_paths != path_ids:
        fail("not every M1 path resolves from an open obligation")


def main() -> None:
    if len(sys.argv) != 2:
        fail(f"usage: {sys.argv[0]} REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    manifest_path = repo / "proofs/M1_REQUIREMENTS.json"
    value = load_manifest(manifest_path)
    reject_evidence(value)
    documented = documentation_roster(repo)
    roadmap = roadmap_titles(repo)
    validate_manifest(value, documented, roadmap)
    digest = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    print(
        "PASS: M1 requirements remain open "
        f"({len(roadmap)} roadmap, {len(documented)} properties, "
        f"{len(value['path_obligations'])} paths, {len(EVIDENCE_KINDS)} evidence kinds, "
        f"sha256={digest})"
    )


if __name__ == "__main__":
    main()
