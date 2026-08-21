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

    def add_artifact(self, kind: str, description: str, content: bytes) -> str:
        identifier = f"artifact.{self.next_artifact:05d}"
        self.next_artifact += 1
        relative = f"artifacts/{identifier}.txt"
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
            ]
            for pair_index, (profile, kind) in enumerate(pairs):
                path_id = spec["paths"][pair_index % len(spec["paths"])]
                binding_id = f"binding.{next_binding:05d}"
                next_binding += 1
                artifact_id = self.add_artifact(
                    ARTIFACT_KINDS[kind],
                    f"{kind} for {spec['class']} {spec['id']}",
                    (
                        f"synthetic test-only {kind} artifact for "
                        f"{spec['class']} {spec['id']} {profile}\n"
                    ).encode("utf-8"),
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
                record["receipt_artifact_id"] = self.add_artifact(
                    "QualificationReceipt",
                    f"receipt for {spec['id']}",
                    f"synthetic test-only receipt {spec['id']}\n".encode("ascii"),
                )
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
                record["rationale_artifact_id"] = rationale_ids[0]
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


def obligation_with_status(index: dict[str, Any], status: str) -> dict[str, Any]:
    return next(
        record for record in index["obligations"] if record["closure_status"] == status
    )


def recompute_binding(record: dict[str, Any]) -> None:
    payload = {key: value for key, value in record.items() if key != "binding_sha256"}
    record["binding_sha256"] = canonical_digest(payload)


def fixture_validator(kind: str, context: dict[str, Any]) -> None:
    if kind not in {
        *ARTIFACT_KINDS,
        "qualification-receipt",
    }:
        raise AssertionError(f"unexpected synthetic validator kind: {kind}")
    path = Path(context["artifact_absolute_path"])
    if sha256(path.read_bytes()) != context["artifact"]["sha256"]:
        raise AssertionError("synthetic validator received the wrong artifact")


def invoke(checker: Any, fixture: Fixture, index_path: Path) -> tuple[bool, str]:
    output = io.StringIO()
    try:
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            checker.validate_evidence_index(
                fixture.ferric,
                index_path,
                fixture.fe2o3,
                _test_only_validator=fixture_validator,
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
        passed, output = invoke(checker, fixture, fixture.index_path)
        if (
            not passed
            or "PASS: structurally complete synthetic M1 evidence index" not in output
        ):
            raise AssertionError(f"synthetic complete index was rejected:\n{output}")
        print("PASS: complete synthetic index through test-only validator harness")

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
        add(
            "duplicate binding",
            "duplicate M1 evidence binding id",
            lambda value, _: value["evidence_bindings"].__setitem__(
                -1, copy.deepcopy(value["evidence_bindings"][0])
            ),
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
                "rationale_artifact_id", value["tcb"][0]["artifact_id"]
            ),
        )

        def unsupported_as_proof(value: dict[str, Any], _: Fixture) -> None:
            unsupported = obligation_with_status(value, "Unsupported")
            proved = obligation_with_status(value, "Proved")
            proved["proof_artifact_ids"] = [unsupported["rationale_artifact_id"]]

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

        unpinned = [
            (kind, record)
            for kind, record in checker.TRUSTED_VALIDATORS.items()
            if record[2] is None
        ]
        if not unpinned:
            raise AssertionError("hostile policy requires one RequiredFuture validator")
        evidence_kind, (relative, _protocol, _source_pin) = unpinned[0]
        validator = fixture.ferric / relative
        validator.parent.mkdir(parents=True, exist_ok=True)
        validator.write_text("raise SystemExit(0)\n", encoding="utf-8")
        git(fixture.ferric, "add", relative)
        git(fixture.ferric, "commit", "-q", "-m", "synthetic unpinned validator")
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
            raise AssertionError("production accepted an unpinned validator source")
        if "has no pinned source identity" not in output_buffer.getvalue():
            raise AssertionError(
                "unpinned validator did not fail at its source identity boundary:\n"
                f"{output_buffer.getvalue()}"
            )
        print("PASS: hostile unpinned trusted-validator source")
        print(f"PASS: {len(cases) + 2} hostile M1 evidence-index fixtures")


if __name__ == "__main__":
    main()
