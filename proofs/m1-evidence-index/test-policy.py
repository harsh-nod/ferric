#!/usr/bin/env python3
"""Hostile policy tests for the external M1 evidence index."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "proofs/check-m1-evidence-index.py"
REQUIREMENTS = ROOT / "proofs/M1_REQUIREMENTS.json"
FORMAT = "ferric.m1-evidence-index.v1"
FERRIC_BASE = "c5a86fd56c1c817664593df25c04bbed30e84971"
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
ARTIFACT_KINDS = {
    "artifact-identity": "ArtifactIdentityReport",
    "canonical-structure-check": "CheckerTranscript",
    "external-contract": "ContractDocument",
    "fe2o3-contract": "ContractDocument",
    "hardware-test": "HardwareTranscript",
    "independent-validator": "ValidatorTranscript",
    "negative-mutation": "MutationTranscript",
    "performance-gate": "PerformanceReport",
    "tcb-report": "TcbReport",
    "unsupported-rationale": "UnsupportedRationale",
    "verus-theorem": "TheoremTranscript",
}
EXCLUDED_DIRECTORIES = {".git", ".ruff_cache", "__pycache__", "target"}
EXCLUDED_SUFFIXES = {".pyc", ".receipt"}
FOUNDATION_REGISTRIES = {
    "negative-mutation": (
        Path("proofs/m1/negative/REQUIRED_FOUNDATIONS"),
        "mutation=",
        "MUTATION",
    ),
    "verus-theorem": (
        Path("proofs/m1/theorem/REQUIRED_FOUNDATIONS"),
        "theorem=",
        "THEOREM",
    ),
}


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_digest(value: dict[str, Any]) -> str:
    return sha256(
        json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
    )


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def git(repo: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        raise RuntimeError(f"git {' '.join(arguments)} failed: {result.stderr}")
    return result.stdout.strip()


def commit_fixture(repo: Path) -> None:
    git(repo, "init", "-q")
    git(repo, "config", "user.email", "m1-policy@example.invalid")
    git(repo, "config", "user.name", "M1 policy fixture")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "synthetic M1 policy fixture")


def source_closure(repo: Path) -> bytes:
    records: list[str] = []
    candidates = sorted(
        repo.rglob("*"), key=lambda path: path.relative_to(repo).as_posix()
    )
    for path in candidates:
        relative = path.relative_to(repo)
        if any(part in EXCLUDED_DIRECTORIES for part in relative.parts):
            continue
        if path.is_dir():
            continue
        if path.suffix in EXCLUDED_SUFFIXES:
            continue
        mode = stat.S_IMODE(path.stat().st_mode)
        content = path.read_bytes()
        records.append(
            f"{relative.as_posix()}|{mode:o}|{len(content)}|{sha256(content)}"
        )
    return ("\n".join(records) + "\n").encode("utf-8")


def foundation_rows(repo: Path) -> dict[str, list[tuple[str, ...]]]:
    result: dict[str, list[tuple[str, ...]]] = {}
    for evidence_kind, (
        relative,
        prefix,
        _selector_key,
    ) in FOUNDATION_REGISTRIES.items():
        lines = (repo / relative).read_text(encoding="ascii").splitlines()[1:]
        rows = [tuple(line.removeprefix(prefix).split("|")) for line in lines]
        if not rows or any(not line.startswith(prefix) for line in lines):
            raise AssertionError(f"fixture {evidence_kind} registry is malformed")
        result[evidence_kind] = rows
    return result


def load_checker() -> Any:
    spec = importlib.util.spec_from_file_location("m1_evidence_checker", CHECKER)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load M1 evidence checker")
    sys.dont_write_bytecode = True
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class Fixture:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.ferric = root / "ferric"
        self.fe2o3 = root / "fe2o3"
        self.evidence = root / "evidence"
        self.index_path = self.evidence / "M1_EVIDENCE_INDEX.json"
        self.requirements = json.loads(REQUIREMENTS.read_text(encoding="utf-8"))
        self.payloads: dict[str, bytes] = {}
        self.next_artifact = 0
        self._create_repositories()
        self.foundation_rows = foundation_rows(self.ferric)
        self.index = self._create_index()

    def _create_repositories(self) -> None:
        shutil.copytree(
            ROOT,
            self.ferric,
            ignore=shutil.ignore_patterns(
                ".git",
                ".ruff_cache",
                "target",
                "__pycache__",
                "*.pyc",
                "*.receipt",
            ),
        )
        self.fe2o3.mkdir()
        for record in self.requirements["path_obligations"]:
            repo = self.ferric if record["repository"] == "ferric" else self.fe2o3
            path = repo / record["path"]
            if not path.exists():
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(
                    f"synthetic test-only path {record['id']}\n", encoding="utf-8"
                )
        commit_fixture(self.ferric)
        commit_fixture(self.fe2o3)

    def add_artifact(
        self,
        kind: str,
        description: str,
        content: bytes,
        *,
        foundation_selector: str | None = None,
    ) -> str:
        identifier = f"artifact.{self.next_artifact:05d}"
        self.next_artifact += 1
        relative = (
            f"artifacts/{identifier}/{foundation_selector}.result"
            if foundation_selector is not None
            else f"artifacts/{identifier}.txt"
        )
        self.payloads[relative] = content
        self.artifacts.append(
            {
                "id": identifier,
                "kind": kind,
                "path": relative,
                "sha256": sha256(content),
                "size_bytes": len(content),
            }
        )
        return identifier

    def _create_index(self) -> dict[str, Any]:
        self.artifacts: list[dict[str, Any]] = []
        closures = {
            "source.fe2o3": source_closure(self.fe2o3),
            "source.ferric": source_closure(self.ferric),
        }
        sources: list[dict[str, Any]] = []
        for source_id, repo_name, repo in (
            ("source.fe2o3", "fe2o3", self.fe2o3),
            ("source.ferric", "ferric", self.ferric),
        ):
            closure = closures[source_id]
            artifact_id = self.add_artifact(
                "SourceClosure", f"source closure for {source_id}", closure
            )
            sources.append(
                {
                    "base_commit": (
                        self.requirements["m1_upstream_base_commit"]
                        if repo_name == "fe2o3"
                        else FERRIC_BASE
                    ),
                    "commit": git(repo, "rev-parse", "HEAD^{commit}"),
                    "id": source_id,
                    "repository": repo_name,
                    "source_closure_artifact_id": artifact_id,
                    "source_closure_sha256": sha256(closure),
                    "tree": git(repo, "rev-parse", "HEAD^{tree}"),
                }
            )

        tcb: list[dict[str, Any]] = []
        for identifier in TCB_IDS:
            artifact_id = self.add_artifact(
                "TcbReport",
                f"TCB report for {identifier}",
                f"synthetic test-only TCB report {identifier}\n".encode("ascii"),
            )
            artifact = self.artifacts[-1]
            tcb.append(
                {
                    "artifact_id": artifact_id,
                    "id": identifier,
                    "identity_sha256": artifact["sha256"],
                    "kind": TCB_KINDS[identifier],
                }
            )

        resolutions = [
            {
                "availability": record["availability"],
                "id": record["id"],
                "path": record["path"],
                "repository": record["repository"],
                "source_identity_id": f"source.{record['repository']}",
            }
            for record in self.requirements["path_obligations"]
        ]
        resolution_by_id = {record["id"]: record for record in resolutions}
        profiles = {
            record["id"]: record["kinds"]
            for record in self.requirements["evidence_profiles"]
        }
        binding_classes = {
            record["kind"]: record["classes"]
            for record in self.requirements["evidence_kind_binding_classes"]
        }
        specs: list[dict[str, Any]] = []
        for record in self.requirements["roadmap_requirements"]:
            specs.append(
                {
                    "class": "Roadmap",
                    "dependencies": record["assurance_properties"],
                    "id": record["id"],
                    "paths": record["path_obligations"],
                    "profiles": record["evidence_profiles"],
                    "statement": record["title"],
                    "status": "Closed",
                }
            )
        for record in self.requirements["assurance_properties"]:
            specs.append(
                {
                    "class": "Assurance",
                    "id": record["name"],
                    "paths": record["path_obligations"],
                    "profiles": record["evidence_profiles"],
                    "statement": record["boundary"],
                    "status": record["required_status_at_closure"],
                }
            )

        bindings: list[dict[str, Any]] = []
        grouped: dict[tuple[str, str], list[dict[str, Any]]] = {}
        next_binding = 0
        for spec in specs:
            key = (spec["class"], spec["id"])
            grouped[key] = []
            pairs = [
                (profile, kind)
                for profile in spec["profiles"]
                for kind in profiles[profile]
                if spec["class"] in binding_classes[kind]
            ]
            pair_paths: list[tuple[str, str, str]] = []
            for pair_index, (profile, kind) in enumerate(pairs):
                matching_paths = sorted(
                    {
                        row[3]
                        for row in self.foundation_rows.get(kind, [])
                        if row[2] == spec["id"] and row[3] in spec["paths"]
                    }
                )
                path_id = (
                    matching_paths[pair_index % len(matching_paths)]
                    if matching_paths
                    else spec["paths"][pair_index % len(spec["paths"])]
                )
                pair_paths.append((profile, kind, path_id))
            covered_paths = {path_id for _profile, _kind, path_id in pair_paths}
            for path_index, path_id in enumerate(spec["paths"]):
                if path_id not in covered_paths:
                    alternatives = [
                        (profile, kind)
                        for profile, kind in pairs
                        if kind not in FOUNDATION_REGISTRIES
                        and (profile, kind, path_id) not in pair_paths
                    ]
                    if not alternatives:
                        alternatives = [
                            (profile, kind)
                            for profile, kind in pairs
                            if (profile, kind, path_id) not in pair_paths
                        ]
                    profile, kind = alternatives[path_index % len(alternatives)]
                    pair_paths.append((profile, kind, path_id))
            for profile, kind, path_id in pair_paths:
                binding_id = f"binding.{next_binding:05d}"
                next_binding += 1
                selector_rows = [
                    row
                    for row in self.foundation_rows.get(kind, [])
                    if row[2] == spec["id"] and row[3] == path_id
                ]
                selector = selector_rows[0][0] if selector_rows else None
                if kind in FOUNDATION_REGISTRIES:
                    selector_key = FOUNDATION_REGISTRIES[kind][2]
                    selected = selector or (
                        f"missing-{spec['id'].replace('_', '-')}-{path_id}"
                    )
                    content = f"{selector_key}={selected}\n".encode("ascii")
                else:
                    content = (
                        f"synthetic test-only {kind} artifact for "
                        f"{spec['class']} {spec['id']} {profile}\n"
                    ).encode("utf-8")
                artifact_id = self.add_artifact(
                    ARTIFACT_KINDS[kind],
                    f"{kind} for {spec['class']} {spec['id']}",
                    content,
                    foundation_selector=(
                        selector or f"missing-{spec['id'].replace('_', '-')}-{path_id}"
                        if kind in FOUNDATION_REGISTRIES
                        else None
                    ),
                )
                binding = {
                    "artifact_id": artifact_id,
                    "evidence_kind": kind,
                    "id": binding_id,
                    "obligation_class": spec["class"],
                    "obligation_id": spec["id"],
                    "path_id": path_id,
                    "profile_id": profile,
                    "source_identity_id": resolution_by_id[path_id][
                        "source_identity_id"
                    ],
                    "statement_sha256": sha256(spec["statement"].encode("utf-8")),
                    "tcb_ids": list(TCB_IDS),
                }
                binding["binding_sha256"] = canonical_digest(binding)
                bindings.append(binding)
                grouped[key].append(binding)

        obligations: list[dict[str, Any]] = []
        receipt_artifact_id = self.add_artifact(
            "QualificationReceipt",
            "canonical M1 qualification receipt",
            b"synthetic test-only canonical M1 receipt\n",
        )
        for spec in specs:
            key = (spec["class"], spec["id"])
            own = grouped[key]
            record: dict[str, Any] = {
                "closure_status": spec["status"],
                "evidence_binding_ids": sorted(item["id"] for item in own),
                "id": spec["id"],
                "obligation_class": spec["class"],
                "path_resolution_ids": spec["paths"],
                "statement_sha256": sha256(spec["statement"].encode("utf-8")),
                "tcb_ids": list(TCB_IDS),
            }
            if spec["class"] == "Roadmap":
                record["assurance_dependencies"] = spec["dependencies"]
                record["receipt_artifact_id"] = receipt_artifact_id
            elif spec["status"] == "Proved":
                record["proof_artifact_ids"] = sorted(
                    item["artifact_id"]
                    for item in own
                    if item["evidence_kind"] == "verus-theorem"
                )
                record["mutation_artifact_ids"] = sorted(
                    item["artifact_id"]
                    for item in own
                    if item["evidence_kind"] == "negative-mutation"
                )
            elif spec["status"] == "Validated":
                record["validator_artifact_ids"] = sorted(
                    item["artifact_id"]
                    for item in own
                    if item["evidence_kind"] == "independent-validator"
                )
                record["validator_tcb_ids"] = list(TCB_IDS)
            else:
                rationale_ids = [
                    item["artifact_id"]
                    for item in own
                    if item["evidence_kind"] == "unsupported-rationale"
                ]
                record["nonclaim_tcb_ids"] = list(TCB_IDS)
                record["rationale"] = spec["statement"]
                record["rationale_artifact_ids"] = rationale_ids
            obligations.append(record)

        return {
            "artifacts": sorted(self.artifacts, key=lambda record: record["id"]),
            "evidence_bindings": sorted(bindings, key=lambda record: record["id"]),
            "format": FORMAT,
            "obligations": obligations,
            "path_resolutions": resolutions,
            "requirements_sha256": sha256(REQUIREMENTS.read_bytes()),
            "sources": sources,
            "tcb": tcb,
        }

    def reset_evidence(self) -> None:
        shutil.rmtree(self.evidence, ignore_errors=True)
        for relative, content in self.payloads.items():
            path = self.evidence / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)


Mutation = Callable[[dict[str, Any], Fixture], None]


def artifact(index: dict[str, Any], identifier: str) -> dict[str, Any]:
    return next(record for record in index["artifacts"] if record["id"] == identifier)


def binding_of_kind(index: dict[str, Any], kind: str) -> dict[str, Any]:
    return next(
        record
        for record in index["evidence_bindings"]
        if record["evidence_kind"] == kind
    )


def foundation_binding(
    index: dict[str, Any], kind: str, property_name: str | None = None
) -> dict[str, Any]:
    return next(
        record
        for record in index["evidence_bindings"]
        if record["evidence_kind"] == kind
        and record["obligation_class"] == "Assurance"
        and (property_name is None or record["obligation_id"] == property_name)
    )


def obligation_with_status(index: dict[str, Any], status: str) -> dict[str, Any]:
    return next(
        record for record in index["obligations"] if record["closure_status"] == status
    )


def obligation(
    index: dict[str, Any], obligation_class: str, identifier: str
) -> dict[str, Any]:
    return next(
        record
        for record in index["obligations"]
        if record["obligation_class"] == obligation_class and record["id"] == identifier
    )


def recompute_binding(record: dict[str, Any]) -> None:
    payload = {key: value for key, value in record.items() if key != "binding_sha256"}
    record["binding_sha256"] = canonical_digest(payload)


def rewrite_foundation_artifact(
    value: dict[str, Any],
    fixture: Fixture,
    binding: dict[str, Any],
    selector: str,
    content: bytes,
) -> None:
    record = artifact(value, binding["artifact_id"])
    old_path = fixture.evidence / record["path"]
    old_path.unlink()
    relative = f"artifacts/{record['id']}/hostile/{selector}.result"
    path = fixture.evidence / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    record["path"] = relative
    record["sha256"] = sha256(content)
    record["size_bytes"] = len(content)


def fixture_validator(kind: str, context: dict[str, Any]) -> None:
    if kind not in {
        *ARTIFACT_KINDS,
        "qualification-receipt",
    }:
        raise AssertionError(f"unexpected synthetic validator kind: {kind}")
    path = Path(context["artifact_absolute_path"])
    if sha256(path.read_bytes()) != context["artifact"]["sha256"]:
        raise AssertionError("synthetic validator received the wrong artifact")


def invoke(
    checker: Any,
    fixture: Fixture,
    index_path: Path,
    validator: Callable[[str, dict[str, Any]], None] = fixture_validator,
) -> tuple[bool, str]:
    output = io.StringIO()
    try:
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            checker.validate_evidence_index(
                fixture.ferric,
                index_path,
                fixture.fe2o3,
                _test_only_validator=validator,
            )
    except SystemExit:
        return False, output.getvalue()
    return True, output.getvalue()


def run_hostile_case(
    checker: Any,
    fixture: Fixture,
    name: str,
    marker: str,
    mutate: Mutation,
    *,
    encoding: str = "canonical",
) -> None:
    fixture.reset_evidence()
    value = copy.deepcopy(fixture.index)
    mutate(value, fixture)
    if encoding == "canonical":
        write_json(fixture.index_path, value)
    elif encoding == "minified":
        fixture.index_path.write_text(json.dumps(value), encoding="utf-8")
    elif encoding == "duplicate-key":
        source = json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
        needle = f'  "format": "{FORMAT}",'
        source = source.replace(needle, f"{needle}\n{needle}", 1)
        fixture.index_path.write_text(source, encoding="utf-8")
    else:
        raise AssertionError(f"unknown hostile encoding: {encoding}")
    passed, output = invoke(checker, fixture, fixture.index_path)
    if passed or marker not in output:
        raise AssertionError(
            f"hostile case {name!r} did not fail with {marker!r}:\n{output}"
        )
    print(f"PASS: hostile {name}")


def main() -> None:
    checker = load_checker()
    with tempfile.TemporaryDirectory(prefix="ferric-m1-evidence-policy-") as temporary:
        fixture = Fixture(Path(temporary))
        fixture.reset_evidence()
        write_json(fixture.index_path, fixture.index)
        roadmap_forbidden = {
            "negative-mutation",
            "unsupported-rationale",
            "verus-theorem",
        }
        if any(
            record["obligation_class"] == "Roadmap"
            and record["evidence_kind"] in roadmap_forbidden
            for record in fixture.index["evidence_bindings"]
        ):
            raise AssertionError(
                "synthetic Roadmap closure contains Assurance-only evidence"
            )
        if any(
            record["evidence_kind"] == "tcb-report"
            for record in fixture.index["evidence_bindings"]
        ):
            raise AssertionError("synthetic closure binds globally scoped TCB evidence")
        for kind in roadmap_forbidden:
            if not any(
                record["obligation_class"] == "Assurance"
                and record["evidence_kind"] == kind
                for record in fixture.index["evidence_bindings"]
            ):
                raise AssertionError(f"synthetic Assurance closure lacks {kind}")
        passed, output = invoke(checker, fixture, fixture.index_path)
        if (
            not passed
            or "PASS: structurally complete synthetic M1 evidence index" not in output
        ):
            raise AssertionError(f"synthetic complete index was rejected:\n{output}")
        print("PASS: complete synthetic index through test-only validator harness")

        receipt_ids = {
            record["receipt_artifact_id"]
            for record in fixture.index["obligations"]
            if record["obligation_class"] == "Roadmap"
        }
        if len(receipt_ids) != 1:
            raise AssertionError("synthetic index lacks one canonical receipt")
        receipt_id = next(iter(receipt_ids))
        candidate = copy.deepcopy(fixture.index)
        candidate["artifacts"] = [
            record for record in candidate["artifacts"] if record["id"] != receipt_id
        ]
        for record in candidate["obligations"]:
            if record["obligation_class"] == "Roadmap":
                record["receipt_artifact_id"] = "artifact.qualification.m1"
        candidate_path = fixture.evidence / "candidate-index.json"
        write_json(candidate_path, candidate)
        for gate_id in checker.PRE_RECEIPT_GATE_IDS:
            gate_output = io.StringIO()
            try:
                with (
                    contextlib.redirect_stdout(gate_output),
                    contextlib.redirect_stderr(gate_output),
                ):
                    checker.validate_evidence_index(
                        fixture.ferric,
                        candidate_path,
                        fixture.fe2o3,
                        _test_only_validator=fixture_validator,
                        _pre_receipt_gate=gate_id,
                    )
            except SystemExit as error:
                raise AssertionError(
                    f"pre-receipt gate {gate_id} rejected the candidate:\n"
                    f"{gate_output.getvalue()}"
                ) from error
            expected = (
                f"PASS: {checker.PRE_RECEIPT_PROTOCOL} gate={gate_id} "
                f"candidate_sha256={sha256(candidate_path.read_bytes())}\n"
            )
            if gate_output.getvalue() != expected:
                raise AssertionError(
                    f"pre-receipt gate {gate_id} output drifted:\n"
                    f"{gate_output.getvalue()}"
                )
        print("PASS: all source-pinned pre-receipt gate protocols")

        intake_root = Path(temporary) / "separate-qualification-run"
        intake_root.mkdir()
        exported_candidate = intake_root / "candidate-index.json"
        shutil.copyfile(candidate_path, exported_candidate)
        gate_output = io.StringIO()
        with (
            contextlib.redirect_stdout(gate_output),
            contextlib.redirect_stderr(gate_output),
        ):
            checker.validate_evidence_index(
                fixture.ferric,
                exported_candidate,
                fixture.fe2o3,
                _test_only_validator=fixture_validator,
                _pre_receipt_gate="evidence-index",
                _pre_receipt_artifact_root=fixture.evidence,
            )
        if (
            f"gate=evidence-index candidate_sha256={sha256(candidate_path.read_bytes())}"
            not in gate_output.getvalue()
        ):
            raise AssertionError(
                "separate candidate/artifact roots were not admitted:\n"
                f"{gate_output.getvalue()}"
            )
        print("PASS: pre-receipt candidate uses an explicit artifact root")

        callback_count = 0

        def counting_validator(kind: str, context: dict[str, Any]) -> None:
            nonlocal callback_count
            callback_count += 1
            fixture_validator(kind, context)

        bypass_value = copy.deepcopy(fixture.index)
        bypass_binding = foundation_binding(bypass_value, "verus-theorem")
        rewrite_foundation_artifact(
            bypass_value,
            fixture,
            bypass_binding,
            "fake-unregistered-theorem",
            b"THEOREM=fake-unregistered-theorem\n",
        )
        write_json(fixture.index_path, bypass_value)
        passed, output = invoke(
            checker,
            fixture,
            fixture.index_path,
            counting_validator,
        )
        if (
            passed
            or "foundation selector is not in the checked registry" not in output
            or callback_count != 0
        ):
            raise AssertionError(
                "test-only callback ran before foundation reachability failed:\n"
                f"callbacks={callback_count}\n{output}"
            )
        print("PASS: foundation reachability cannot be bypassed by test callback")
        fixture.reset_evidence()
        write_json(fixture.index_path, fixture.index)

        production = subprocess.run(
            [
                sys.executable,
                "-I",
                str(CHECKER),
                str(fixture.ferric),
                str(fixture.index_path),
                str(fixture.fe2o3),
            ],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            env={"PATH": os.environ.get("PATH", "")},
        )
        if production.returncode == 0 or not any(
            marker in production.stdout
            for marker in (
                "trusted M1 validator is absent",
                "trusted M1 validator rejected artifact-identity",
            )
        ):
            raise AssertionError(
                "production CLI accepted synthetic self-reports without trusted validators:\n"
                f"{production.stdout}"
            )
        print("PASS: production CLI rejects synthetic validator self-reports")

        cases: list[tuple[str, str, Mutation, str]] = []

        def add(
            name: str,
            marker: str,
            mutate: Mutation,
            encoding: str = "canonical",
        ) -> None:
            cases.append((name, marker, mutate, encoding))

        def missing_theorem_selector(value: dict[str, Any], fixture: Fixture) -> None:
            binding = foundation_binding(value, "verus-theorem")
            current = Path(artifact(value, binding["artifact_id"])["path"]).stem
            rewrite_foundation_artifact(
                value,
                fixture,
                binding,
                current,
                b"FORMAT=FERRIC-M1-POSITIVE-RESULT-V1\n",
            )

        add(
            "missing theorem foundation selector",
            "foundation selector is missing or has the wrong kind",
            missing_theorem_selector,
        )

        def malformed_mutation_selector(
            value: dict[str, Any], fixture: Fixture
        ) -> None:
            binding = foundation_binding(value, "negative-mutation")
            current = Path(artifact(value, binding["artifact_id"])["path"]).stem
            rewrite_foundation_artifact(
                value,
                fixture,
                binding,
                current,
                f"MUTATION={current}\nnot-a-record\n".encode("ascii"),
            )

        add(
            "malformed mutation foundation selector",
            "foundation selector artifact is malformed",
            malformed_mutation_selector,
        )

        def fake_theorem_selector(value: dict[str, Any], fixture: Fixture) -> None:
            binding = foundation_binding(value, "verus-theorem")
            selector = "fake-unregistered-theorem"
            rewrite_foundation_artifact(
                value,
                fixture,
                binding,
                selector,
                f"THEOREM={selector}\n".encode("ascii"),
            )

        add(
            "fake theorem foundation selector",
            "foundation selector is not in the checked registry",
            fake_theorem_selector,
        )

        def cross_property_theorem_selector(
            value: dict[str, Any], fixture: Fixture
        ) -> None:
            binding = foundation_binding(
                value, "verus-theorem", "model_bundle_well_formed"
            )
            selector = next(
                row[0]
                for row in fixture.foundation_rows["verus-theorem"]
                if row[2] != binding["obligation_id"]
            )
            rewrite_foundation_artifact(
                value,
                fixture,
                binding,
                selector,
                f"THEOREM={selector}\n".encode("ascii"),
            )

        add(
            "cross-property theorem selector",
            "foundation selector substituted a different property",
            cross_property_theorem_selector,
        )

        def cross_path_theorem_selector(
            value: dict[str, Any], fixture: Fixture
        ) -> None:
            binding = foundation_binding(value, "verus-theorem", "scheduler_refined")
            selector = next(
                row[0]
                for row in fixture.foundation_rows["verus-theorem"]
                if row[2] == binding["obligation_id"] and row[3] != binding["path_id"]
            )
            rewrite_foundation_artifact(
                value,
                fixture,
                binding,
                selector,
                f"THEOREM={selector}\n".encode("ascii"),
            )

        add(
            "cross-path theorem selector",
            "foundation selector substituted a different path",
            cross_path_theorem_selector,
        )

        def fe2o3_foundation_substitution(value: dict[str, Any], _: Fixture) -> None:
            binding = foundation_binding(value, "verus-theorem", "lifetime_safe")
            property_record = next(
                record
                for record in fixture.requirements["assurance_properties"]
                if record["name"] == "lifetime_safe"
            )
            fe2o3_path = next(
                path_id
                for path_id in property_record["path_obligations"]
                if next(
                    record
                    for record in value["path_resolutions"]
                    if record["id"] == path_id
                )["source_identity_id"]
                == "source.fe2o3"
            )
            binding["path_id"] = fe2o3_path
            binding["source_identity_id"] = "source.fe2o3"
            recompute_binding(binding)

        add(
            "fe2o3 theorem source substitution",
            "foundation selector substituted a non-Ferric source",
            fe2o3_foundation_substitution,
        )

        add(
            "roadmap omission",
            "exactly 50 obligation records",
            lambda value, _: value["obligations"].pop(0),
        )
        add(
            "assurance omission",
            "exactly 50 obligation records",
            lambda value, _: value["obligations"].pop(),
        )
        add(
            "duplicate obligation",
            "duplicate M1 closure obligation",
            lambda value, _: value["obligations"].__setitem__(
                1, copy.deepcopy(value["obligations"][0])
            ),
        )
        add(
            "status weakening",
            "closure status weakened or promoted",
            lambda value, _: obligation_with_status(value, "Proved").__setitem__(
                "closure_status", "Checked"
            ),
        )
        add(
            "status promotion",
            "closure status weakened or promoted",
            lambda value, _: obligation_with_status(value, "Unsupported").__setitem__(
                "closure_status", "Proved"
            ),
        )
        add(
            "wrong Ferric commit",
            "source commit or tree mismatch",
            lambda value, _: value["sources"][1].__setitem__(
                "commit", "0123456789abcdef0123456789abcdef01234567"
            ),
        )
        add(
            "wrong fe2o3 tree",
            "source commit or tree mismatch",
            lambda value, _: value["sources"][0].__setitem__(
                "tree", "89abcdef0123456789abcdef0123456789abcdef"
            ),
        )
        add(
            "wrong resolved path",
            "path resolution drifted",
            lambda value, _: value["path_resolutions"][0].__setitem__(
                "path", "wrong/path.rs"
            ),
        )
        add(
            "path omission",
            "path resolution roster must contain exactly",
            lambda value, _: value["path_resolutions"].pop(),
        )
        add(
            "source omission",
            "source roster must contain exactly",
            lambda value, _: value["sources"].pop(),
        )
        add(
            "profile omission",
            "profile-kind closure is incomplete",
            lambda value, _: value["evidence_bindings"].pop(0),
        )
        add(
            "wrong profile",
            "wrong profile or kind",
            lambda value, _: value["evidence_bindings"][0].__setitem__(
                "profile_id", "nonclaim"
            ),
        )
        add(
            "wrong evidence kind",
            "wrong profile or kind",
            lambda value, _: value["evidence_bindings"][0].__setitem__(
                "evidence_kind", "hardware-test"
            ),
        )

        def add_inapplicable_roadmap_binding(
            value: dict[str, Any], kind: str, roadmap_id: str, profile: str
        ) -> None:
            source = (
                value["evidence_bindings"][0]
                if kind == "tcb-report"
                else binding_of_kind(value, kind)
            )
            roadmap = next(
                record
                for record in fixture.requirements["roadmap_requirements"]
                if record["id"] == roadmap_id
            )
            path_id = roadmap["path_obligations"][0]
            path = next(
                record
                for record in value["path_resolutions"]
                if record["id"] == path_id
            )
            record = copy.deepcopy(source)
            record.update(
                {
                    "id": f"binding.zz-{kind}",
                    "artifact_id": (
                        value["tcb"][0]["artifact_id"]
                        if kind == "tcb-report"
                        else source["artifact_id"]
                    ),
                    "evidence_kind": kind,
                    "obligation_class": "Roadmap",
                    "obligation_id": roadmap_id,
                    "path_id": path_id,
                    "profile_id": profile,
                    "source_identity_id": path["source_identity_id"],
                    "statement_sha256": sha256(roadmap["title"].encode("utf-8")),
                }
            )
            recompute_binding(record)
            value["evidence_bindings"].append(record)

        for kind, roadmap_id, profile in (
            ("negative-mutation", "m1.r01", "admission"),
            ("unsupported-rationale", "m1.r24", "nonclaim"),
            ("verus-theorem", "m1.r01", "admission"),
            ("tcb-report", "m1.r01", "admission"),
        ):
            add(
                f"Roadmap {kind} binding",
                "evidence kind does not support the obligation class",
                lambda value, _fixture, kind=kind, roadmap_id=roadmap_id, profile=profile: (
                    add_inapplicable_roadmap_binding(value, kind, roadmap_id, profile)
                ),
            )
        add(
            "duplicate binding",
            "duplicate M1 evidence binding id",
            lambda value, _: value["evidence_bindings"].__setitem__(
                -1, copy.deepcopy(value["evidence_bindings"][0])
            ),
        )

        def duplicate_profile_kind_path(value: dict[str, Any], _: Fixture) -> None:
            record = copy.deepcopy(value["evidence_bindings"][0])
            record["id"] = "binding.zz-duplicate-triplet"
            recompute_binding(record)
            value["evidence_bindings"].append(record)

        add(
            "duplicate profile-kind-path binding",
            "duplicate M1 profile-kind-path evidence binding",
            duplicate_profile_kind_path,
        )

        def omit_graph_path(value: dict[str, Any], _: Fixture) -> None:
            graph = next(
                record
                for record in fixture.requirements["assurance_properties"]
                if record["name"] == "graph_refined"
            )
            path_id = graph["path_obligations"][-1]
            value["evidence_bindings"][:] = [
                record
                for record in value["evidence_bindings"]
                if not (
                    record["obligation_class"] == "Assurance"
                    and record["obligation_id"] == "graph_refined"
                    and record["path_id"] == path_id
                )
            ]

        add(
            "required binding path omission",
            "M1 evidence path coverage is incomplete: Assurance:graph_refined",
            omit_graph_path,
        )
        add(
            "wrong binding source",
            "wrong source identity",
            lambda value, _: value["evidence_bindings"][0].__setitem__(
                "source_identity_id",
                (
                    "source.fe2o3"
                    if value["evidence_bindings"][0]["source_identity_id"]
                    == "source.ferric"
                    else "source.ferric"
                ),
            ),
        )
        add(
            "binding reused by obligation",
            "evidence binding roster drifted",
            lambda value, _: value["obligations"][1].__setitem__(
                "evidence_binding_ids", value["obligations"][0]["evidence_binding_ids"]
            ),
        )

        def reuse_artifact(value: dict[str, Any], _: Fixture) -> None:
            first = value["evidence_bindings"][0]
            second = next(
                record
                for record in value["evidence_bindings"][1:]
                if record["evidence_kind"] == first["evidence_kind"]
            )
            second["artifact_id"] = first["artifact_id"]
            recompute_binding(second)

        add(
            "artifact reused by bindings",
            "artifact is reused across incompatible bindings",
            reuse_artifact,
        )

        def reuse_artifact_across_repeated_pair(
            value: dict[str, Any], _: Fixture
        ) -> None:
            graph_bindings = [
                record
                for record in value["evidence_bindings"]
                if record["obligation_class"] == "Assurance"
                and record["obligation_id"] == "graph_refined"
            ]
            first = next(
                record
                for record in graph_bindings
                if any(
                    other["profile_id"] == record["profile_id"]
                    and other["evidence_kind"] == record["evidence_kind"]
                    and other["path_id"] != record["path_id"]
                    for other in graph_bindings
                )
            )
            second = next(
                record
                for record in graph_bindings
                if record is not first
                and record["profile_id"] == first["profile_id"]
                and record["evidence_kind"] == first["evidence_kind"]
                and record["path_id"] != first["path_id"]
            )
            second["artifact_id"] = first["artifact_id"]
            recompute_binding(second)

        add(
            "artifact reused across repeated pair",
            "artifact is reused across incompatible bindings",
            reuse_artifact_across_repeated_pair,
        )

        def mistype_receipt(value: dict[str, Any], _: Fixture) -> None:
            receipt = value["obligations"][0]["receipt_artifact_id"]
            artifact(value, receipt)["kind"] = "CheckerTranscript"

        add("fake receipt kind", "receipt is unavailable or fake", mistype_receipt)

        def tamper_receipt(value: dict[str, Any], fixture: Fixture) -> None:
            receipt = value["obligations"][0]["receipt_artifact_id"]
            record = artifact(value, receipt)
            (fixture.evidence / record["path"]).write_text(
                "tampered\n", encoding="utf-8"
            )

        add("fake receipt bytes", "artifact identity mismatch", tamper_receipt)

        def tamper_closure(value: dict[str, Any], fixture: Fixture) -> None:
            closure = value["sources"][0]["source_closure_artifact_id"]
            record = artifact(value, closure)
            (fixture.evidence / record["path"]).write_text(
                "tampered\n", encoding="utf-8"
            )

        add("source closure tamper", "artifact identity mismatch", tamper_closure)
        add(
            "TCB omission",
            "TCB roster must contain exactly",
            lambda value, _: value["tcb"].pop(),
        )
        add(
            "TCB kind drift",
            "TCB kind or identity drifted",
            lambda value, _: value["tcb"][0].__setitem__("kind", "Runtime"),
        )

        def substitute_global_tcb(value: dict[str, Any], _: Fixture) -> None:
            value["tcb"][0]["artifact_id"] = value["tcb"][1]["artifact_id"]
            value["tcb"][0]["identity_sha256"] = value["tcb"][1]["identity_sha256"]

        add(
            "global TCB report substitution",
            "TCB artifact or identity is reused",
            substitute_global_tcb,
        )

        def substitute_proof(value: dict[str, Any], _: Fixture) -> None:
            record = obligation_with_status(value, "Proved")
            record["proof_artifact_ids"] = record["mutation_artifact_ids"]

        add(
            "proof self-report substitution",
            "no exact theorem artifacts",
            substitute_proof,
        )

        def substitute_mutation(value: dict[str, Any], _: Fixture) -> None:
            record = obligation_with_status(value, "Proved")
            record["mutation_artifact_ids"] = record["proof_artifact_ids"]

        add(
            "mutation self-report substitution",
            "no exact mutation artifacts",
            substitute_mutation,
        )

        def omit_validator_field(value: dict[str, Any], _: Fixture) -> None:
            obligation_with_status(value, "Validated").pop("validator_artifact_ids")

        add(
            "validator evidence omission",
            "has unexpected keys",
            omit_validator_field,
        )

        def remove_kind(value: dict[str, Any], _: Fixture, kind: str) -> None:
            target = binding_of_kind(value, kind)
            value["evidence_bindings"].remove(target)

        def remove_applicable_pair(
            value: dict[str, Any], _: Fixture, kind: str
        ) -> None:
            target = binding_of_kind(value, kind)
            value["evidence_bindings"][:] = [
                record
                for record in value["evidence_bindings"]
                if not (
                    record["obligation_class"] == target["obligation_class"]
                    and record["obligation_id"] == target["obligation_id"]
                    and record["profile_id"] == target["profile_id"]
                    and record["evidence_kind"] == target["evidence_kind"]
                )
            ]

        add(
            "hardware evidence omission",
            "profile-kind closure is incomplete",
            lambda value, fixture: remove_kind(value, fixture, "hardware-test"),
        )
        add(
            "performance evidence omission",
            "profile-kind closure is incomplete",
            lambda value, fixture: remove_kind(value, fixture, "performance-gate"),
        )
        for kind in (
            "negative-mutation",
            "unsupported-rationale",
            "verus-theorem",
        ):
            add(
                f"Assurance {kind} omission",
                "profile-kind closure is incomplete",
                lambda value, fixture, kind=kind: remove_applicable_pair(
                    value, fixture, kind
                ),
            )
        add(
            "unsupported Roadmap dependency drift",
            "roadmap assurance dependencies drifted",
            lambda value, _: obligation(value, "Roadmap", "m1.r24")[
                "assurance_dependencies"
            ].remove("distribution_preserved"),
        )
        add(
            "unsupported rationale drift",
            "Unsupported closure rationale drifted",
            lambda value, _: obligation_with_status(value, "Unsupported").__setitem__(
                "rationale", "a stronger claim"
            ),
        )
        add(
            "unsupported rationale substitution",
            "no exact rationale artifact",
            lambda value, _: obligation_with_status(value, "Unsupported").__setitem__(
                "rationale_artifact_ids", [value["tcb"][0]["artifact_id"]]
            ),
        )
        add(
            "unsupported rationale artifact omission",
            "no exact rationale artifact",
            lambda value, _: obligation_with_status(value, "Unsupported")[
                "rationale_artifact_ids"
            ].pop(),
        )
        add(
            "unsupported rationale artifact addition",
            "no exact rationale artifact",
            lambda value, _: obligation_with_status(value, "Unsupported")[
                "rationale_artifact_ids"
            ].append(value["tcb"][0]["artifact_id"]),
        )
        add(
            "unsupported rationale artifact duplication",
            "contains a duplicate reference",
            lambda value, _: obligation_with_status(value, "Unsupported")[
                "rationale_artifact_ids"
            ].append(
                obligation_with_status(value, "Unsupported")["rationale_artifact_ids"][
                    0
                ]
            ),
        )

        def drift_rationale_binding_statement(
            value: dict[str, Any], _: Fixture
        ) -> None:
            record = binding_of_kind(value, "unsupported-rationale")
            record["statement_sha256"] = "34" * 32
            recompute_binding(record)

        add(
            "unsupported rationale binding statement drift",
            "wrong statement identity",
            drift_rationale_binding_statement,
        )

        def unsupported_as_proof(value: dict[str, Any], _: Fixture) -> None:
            unsupported = obligation_with_status(value, "Unsupported")
            proved = obligation_with_status(value, "Proved")
            proved["proof_artifact_ids"] = unsupported["rationale_artifact_ids"]

        add(
            "unsupported cannot discharge Proved",
            "no exact theorem artifacts",
            unsupported_as_proof,
        )
        add(
            "binding digest tamper",
            "binding identity mismatch",
            lambda value, _: value["evidence_bindings"][0].__setitem__(
                "binding_sha256", "12" * 32
            ),
        )
        add(
            "statement identity tamper",
            "wrong statement identity",
            lambda value, _: value["evidence_bindings"][0].__setitem__(
                "statement_sha256", "12" * 32
            ),
        )
        add(
            "noncanonical JSON",
            "not canonical JSON",
            lambda _value, _fixture: None,
            "minified",
        )
        add(
            "duplicate JSON key",
            "duplicate JSON key",
            lambda _value, _fixture: None,
            "duplicate-key",
        )
        add(
            "requirements identity drift",
            "does not bind the exact requirements manifest",
            lambda value, _: value.__setitem__("requirements_sha256", "12" * 32),
        )
        add(
            "index-selected validator path",
            "M1 evidence index has unexpected keys",
            lambda value, _: value.__setitem__(
                "validator_path", "artifacts/self-reported-validator.py"
            ),
        )

        def add_unreferenced(value: dict[str, Any], fixture: Fixture) -> None:
            content = b"synthetic unreferenced self-report\n"
            relative = "artifacts/unreferenced.txt"
            (fixture.evidence / relative).write_bytes(content)
            value["artifacts"].append(
                {
                    "id": "artifact.zzzzz",
                    "kind": "TheoremTranscript",
                    "path": relative,
                    "sha256": sha256(content),
                    "size_bytes": len(content),
                }
            )

        add(
            "unreferenced self-report",
            "contains unreferenced artifacts",
            add_unreferenced,
        )

        def mistype_bound_artifact(value: dict[str, Any], _: Fixture) -> None:
            binding = binding_of_kind(value, "hardware-test")
            artifact(value, binding["artifact_id"])["kind"] = "TcbReport"

        add(
            "hardware self-report substitution",
            "cannot satisfy its kind",
            mistype_bound_artifact,
        )

        def mistype_kind(value: dict[str, Any], _: Fixture, kind: str) -> None:
            binding = binding_of_kind(value, kind)
            artifact(value, binding["artifact_id"])["kind"] = "CheckerTranscript"

        for evidence_kind in (
            "verus-theorem",
            "negative-mutation",
            "independent-validator",
            "performance-gate",
        ):
            add(
                f"{evidence_kind} self-report substitution",
                "cannot satisfy its kind",
                lambda value, fixture, kind=evidence_kind: mistype_kind(
                    value, fixture, kind
                ),
            )

        for name, marker, mutate, encoding in cases:
            run_hostile_case(
                checker,
                fixture,
                name,
                marker,
                mutate,
                encoding=encoding,
            )

        fixture.reset_evidence()
        missing = fixture.evidence / "missing.json"
        passed, output = invoke(checker, fixture, missing)
        if passed or "M1 evidence index is unavailable" not in output:
            raise AssertionError(f"missing evidence did not fail closed:\n{output}")
        print("PASS: hostile missing evidence file")

        pinned = [
            (kind, record)
            for kind, record in checker.TRUSTED_VALIDATORS.items()
            if record[2] is not None
        ]
        if not pinned:
            raise AssertionError("hostile policy requires one source-pinned validator")
        evidence_kind, (relative, _protocol, _source_pin) = pinned[0]
        validator = fixture.ferric / relative
        validator.write_text("raise SystemExit(0)\n", encoding="utf-8")
        output_buffer = io.StringIO()
        try:
            with contextlib.redirect_stderr(output_buffer):
                checker.invoke_trusted_validator(
                    fixture.ferric,
                    {relative},
                    evidence_kind,
                    {},
                    None,
                )
        except SystemExit:
            pass
        else:
            raise AssertionError("production accepted a substituted validator source")
        if "source identity mismatch" not in output_buffer.getvalue():
            raise AssertionError(
                "substituted validator did not fail at its source identity boundary:\n"
                f"{output_buffer.getvalue()}"
            )
        print("PASS: hostile substituted trusted-validator source")
        print(f"PASS: {len(cases) + 2} hostile M1 evidence-index fixtures")


if __name__ == "__main__":
    main()
