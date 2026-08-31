#!/usr/bin/env python3
"""Exercise canonical and hostile M1 qualification receipts."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[3]
VALIDATOR_RELATIVE = Path("proofs/m1/evidence/validate-qualification-receipt.py")
INDEX_POLICY_RELATIVE = Path("proofs/m1-evidence-index/test-policy.py")
CHECKER_RELATIVE = Path("proofs/check-m1-evidence-index.py")
PROTOCOL = "ferric.m1-validator.qualification-receipt.v1"
Mutation = Callable[["Fixture"], None]


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any, *, compact: bool = False) -> bytes:
    if compact:
        source = json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        )
    else:
        source = json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True)
    return (source + "\n").encode("ascii")


def canonical_digest(value: Any) -> str:
    return digest_bytes(
        json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
    )


def load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    sys.dont_write_bytecode = True
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def replace_unpinned_validator(checker: Path, evidence_kind: str, source: Path) -> None:
    text = checker.read_text(encoding="ascii")
    marker = (
        f'    "{evidence_kind}": (\n'
        f'        "{source.relative_to(checker.parents[1]).as_posix()}",'
    )
    start = text.find(marker)
    if start < 0:
        raise AssertionError(f"missing checker registry row: {evidence_kind}")
    end = text.find("    ),", start)
    block = text[start : end + len("    ),")]
    source_sha256 = digest_bytes(source.read_bytes())
    if "        None," not in block:
        if f'        "{source_sha256}",' not in block:
            raise AssertionError(
                f"checker registry source pin drifted: {evidence_kind}"
            )
        return
    replacement = block.replace("        None,", f'        "{source_sha256}",')
    checker.write_text(
        text[:start] + replacement + text[end + len("    ),") :], encoding="ascii"
    )


def prepare_template(repo: Path, destination: Path) -> None:
    shutil.copytree(
        repo,
        destination,
        ignore=shutil.ignore_patterns(
            ".git", ".ruff_cache", "target", "__pycache__", "*.pyc", "*.receipt"
        ),
    )
    checker = destination / CHECKER_RELATIVE
    for evidence_kind, name in (
        ("independent-validator", "validate-independent-validator.py"),
        ("performance-gate", "validate-performance-report.py"),
    ):
        source = destination / "proofs/m1/evidence" / name
        if not source.exists():
            source.write_text(
                "#!/usr/bin/env python3\n"
                f'"""Synthetic test-only {evidence_kind} validator."""\n'
                "raise SystemExit(1)\n",
                encoding="ascii",
            )
        replace_unpinned_validator(checker, evidence_kind, source)


class Fixture:
    def __init__(self, temporary: Path) -> None:
        self.temporary = temporary
        self.template = temporary / "template"
        prepare_template(ROOT, self.template)
        index_policy = load_module(
            self.template / INDEX_POLICY_RELATIVE, "qualification_index_policy"
        )
        index_policy.ROOT = self.template
        index_policy.REQUIREMENTS = self.template / "proofs/M1_REQUIREMENTS.json"
        self.foundation = index_policy.Fixture(temporary / "foundation")
        self.ferric = self.foundation.ferric
        self.fe2o3 = self.foundation.fe2o3
        self.evidence = self.foundation.evidence
        self.validator_path = self.ferric / VALIDATOR_RELATIVE
        self.validator = load_module(self.validator_path, "qualification_validator")
        original_receipt_id = self.foundation.index["obligations"][0][
            "receipt_artifact_id"
        ]
        if {
            record["receipt_artifact_id"]
            for record in self.foundation.index["obligations"]
            if record["obligation_class"] == "Roadmap"
        } != {original_receipt_id}:
            raise AssertionError("synthetic index did not use one canonical receipt")
        self.receipt_id = "artifact.qualification.m1"
        self._artifact(self.foundation.index, original_receipt_id)["id"] = (
            self.receipt_id
        )
        for obligation in self.foundation.index["obligations"]:
            if obligation["obligation_class"] == "Roadmap":
                obligation["receipt_artifact_id"] = self.receipt_id
        self.report_relative = f"artifacts/{self.receipt_id}.qualification-receipt.json"
        self.transcript_relative = f"qualification-transcripts/{self.receipt_id}.json"
        self.completion_relative = "completed-work.json"
        self._install_receipt_paths()
        self.base_index = copy.deepcopy(self.foundation.index)
        self.base_transcript = self._make_transcript(self.base_index)
        self.base_completion = self._make_completion(
            self.base_index, self.base_transcript["gates"]
        )
        self._bind_completion(self.base_transcript, self.base_completion)
        self.base_transcript["qualification_id_sha256"] = (
            self.validator.qualification_identity(self.base_transcript)
        )
        self.base_report = self._make_report(self.base_index, self.base_transcript)
        self.index: dict[str, Any] = {}
        self.transcript: dict[str, Any] = {}
        self.report: dict[str, Any] = {}
        self.completion: dict[str, Any] = {}
        self.context: dict[str, Any] = {}
        self.plan_mutation: Callable[[dict[str, Any]], None] | None = None
        self.reset()

    def _artifact(self, index: dict[str, Any], identifier: str) -> dict[str, Any]:
        return next(
            record for record in index["artifacts"] if record["id"] == identifier
        )

    def _install_receipt_paths(self) -> None:
        record = self._artifact(self.foundation.index, self.receipt_id)
        old_relative = record["path"]
        self.foundation.payloads.pop(old_relative)
        record["path"] = self.report_relative
        record["sha256"] = digest_bytes(b"temporary qualification receipt\n")
        record["size_bytes"] = len(b"temporary qualification receipt\n")

    def _source_closure_roster(self, index: dict[str, Any]) -> list[dict[str, Any]]:
        result = []
        for source in index["sources"]:
            closure = self._artifact(index, source["source_closure_artifact_id"])
            raw = self.foundation.payloads[closure["path"]]
            result.append(
                {
                    "artifact_id": closure["id"],
                    "commit": source["commit"],
                    "file_count": len(raw.splitlines()),
                    "id": source["id"],
                    "sha256": source["source_closure_sha256"],
                    "tree": source["tree"],
                }
            )
        return result

    def _validators(self) -> list[dict[str, Any]]:
        return self.validator.checker_registry(self.ferric)

    def _tools(self, validators: list[dict[str, Any]]) -> list[dict[str, Any]]:
        by_id = {record["evidence_kind"]: record for record in validators}
        return [
            {
                "authority": "qualification-measured-binary",
                "id": "compiler.cargo",
                "identity_sha256": digest_bytes(b"synthetic cargo binary identity"),
                "version": "1.97.1",
            },
            {
                "authority": "qualification-measured-binary",
                "id": "compiler.rustc",
                "identity_sha256": digest_bytes(b"synthetic rustc binary identity"),
                "version": "1.97.1",
            },
            {
                "authority": "pinned-proof-tool-closure",
                "id": "compiler.verus",
                "identity_sha256": digest_bytes(
                    (self.ferric / "proofs/verus/VERUS_CLOSURE_MANIFEST").read_bytes()
                ),
                "version": (self.ferric / "proofs/verus/VERUS_VERSION")
                .read_text(encoding="ascii")
                .strip(),
            },
            {
                "authority": "qualification-measured-binary",
                "id": "runtime.python",
                "identity_sha256": digest_bytes(
                    Path(sys.executable).resolve(strict=True).read_bytes()
                ),
                "version": (
                    f"{sys.version_info.major}.{sys.version_info.minor}."
                    f"{sys.version_info.micro}"
                ),
            },
            {
                "authority": "checker-owned-source",
                "id": "validator.evidence-index",
                "identity_sha256": digest_bytes(
                    (self.ferric / CHECKER_RELATIVE).read_bytes()
                ),
                "version": self.validator.INDEX_FORMAT,
            },
            {
                "authority": "checker-owned-source",
                "id": "validator.qualification-receipt",
                "identity_sha256": by_id["qualification-receipt"]["source_sha256"],
                "version": PROTOCOL,
            },
        ]

    def _environment(self) -> dict[str, Any]:
        return {
            "device": {
                "device_count": 1,
                "device_uuid": "123e4567-e89b-42d3-a456-426614174000",
                "marketing_name": "AMD Instinct MI300X",
                "pci_bdf": "0000:41:00.0",
                "processor": "gfx942",
                "vendor_id": "1002",
                "xnack": "disabled",
            },
            "driver": {
                "module_sha256": digest_bytes(b"synthetic amdgpu module"),
                "name": "amdgpu",
                "version": "6.14.14-test",
            },
            "firmware": {
                "bundle_sha256": digest_bytes(b"synthetic firmware bundle"),
                "package_version": "20260821.1",
            },
            "host": {
                "kernel_sha256": digest_bytes(b"synthetic host kernel"),
                "machine": "x86_64",
                "os_release_sha256": digest_bytes(b"synthetic os release"),
            },
            "rocm": {
                "installation_sha256": digest_bytes(b"synthetic ROCm installation"),
                "version": "7.1.0",
            },
        }

    def _gates(self, index: dict[str, Any]) -> list[dict[str, Any]]:
        rosters = self.validator.expected_gate_rosters(index, self.receipt_id)
        candidate = self._index_projection(index)
        candidate_sha256 = digest_bytes(canonical_bytes(candidate))
        checker_sha256 = digest_bytes((self.ferric / CHECKER_RELATIVE).read_bytes())
        tools = self._tools(self._validators())
        python_sha256 = next(
            record["identity_sha256"]
            for record in tools
            if record["id"] == "runtime.python"
        )
        result = []
        for number, identifier in enumerate(self.validator.GATE_IDS, start=1):
            artifacts, bindings = rosters[identifier]
            result.append(
                {
                    "artifact_ids": artifacts,
                    "binding_ids": bindings,
                    "command_sha256": canonical_digest(
                        {
                            "candidate_sha256": candidate_sha256,
                            "checker_sha256": checker_sha256,
                            "gate_id": identifier,
                            "protocol": self.validator.PRE_RECEIPT_PROTOCOL,
                            "runtime_python_sha256": python_sha256,
                        }
                    ),
                    "finished_at_utc": f"2026-08-21T00:{number:02d}:30Z",
                    "id": identifier,
                    "output_sha256": digest_bytes(
                        (
                            f"PASS: {self.validator.PRE_RECEIPT_PROTOCOL} "
                            f"gate={identifier} candidate_sha256={candidate_sha256}\n"
                        ).encode("ascii")
                    ),
                    "result": "pass",
                    "started_at_utc": f"2026-08-21T00:{number:02d}:00Z",
                }
            )
        return result

    def _index_projection(self, index: dict[str, Any]) -> dict[str, Any]:
        return {
            **index,
            "artifacts": [
                record
                for record in index["artifacts"]
                if record["id"] != self.receipt_id
            ],
        }

    def _plan_queue(
        self, index: dict[str, Any]
    ) -> tuple[dict[str, Any], dict[str, Any]]:
        plan_index = getattr(self, "base_index", index)
        candidate = self._index_projection(plan_index)
        artifact_by_id = {record["id"]: record for record in candidate["artifacts"]}
        binding_slots = []
        for binding in candidate["evidence_bindings"]:
            artifact = artifact_by_id[binding["artifact_id"]]
            binding_slots.append(
                {
                    "binding": copy.deepcopy(binding),
                    "expected_artifact": {
                        key: artifact[key] for key in ("id", "kind", "path")
                    },
                    "foundation_selectors": [],
                    "producer": {
                        "availability": "available",
                        "command": ["synthetic-source-bound-producer", binding["id"]],
                        "role": "synthetic-policy-producer",
                    },
                    "state": "missing",
                }
            )
        closure_by_id = {
            record["artifact_id"]: record
            for record in self._source_closure_roster(candidate)
        }
        measure_source = self.ferric / "proofs/m1/evidence/measure-source-closure.py"
        source_closures = []
        for source in candidate["sources"]:
            artifact = artifact_by_id[source["source_closure_artifact_id"]]
            repository = source["repository"]
            source_closures.append(
                {
                    "artifact": copy.deepcopy(artifact),
                    "file_count": closure_by_id[artifact["id"]]["file_count"],
                    "producer": {
                        "command": [
                            "python3",
                            "-I",
                            "proofs/m1/evidence/measure-source-closure.py",
                            repository.upper() + "_REPO",
                            artifact["path"],
                        ],
                        "source_sha256": digest_bytes(measure_source.read_bytes()),
                    },
                }
            )
        validators = [
            {key: value for key, value in record.items() if key != "availability"}
            for record in self._validators()
        ]
        roadmap_bindings = sum(
            record["obligation_class"] == "Roadmap"
            for record in candidate["evidence_bindings"]
        )
        requirements = json.loads(
            (self.ferric / "proofs/M1_REQUIREMENTS.json").read_text(encoding="ascii")
        )
        plan = {
            "allocation_sha256": self.validator.ALLOCATION_SHA256,
            "authority": self.validator.PLAN_AUTHORITY,
            "binding_slots": binding_slots,
            "counts": {
                "assurance_binding_slots": len(candidate["evidence_bindings"])
                - roadmap_bindings,
                "binding_slots": len(candidate["evidence_bindings"]),
                "obligation_slots": len(candidate["obligations"]),
                "path_resolutions": len(candidate["path_resolutions"]),
                "roadmap_binding_slots": roadmap_bindings,
                "source_closures": len(source_closures),
                "trusted_validators": len(validators),
            },
            "fe2o3_pins": {
                "direct": [],
                "direct_count": 0,
                "resolved": [],
                "resolved_count": 0,
                "revision": next(
                    record["commit"]
                    for record in candidate["sources"]
                    if record["repository"] == "fe2o3"
                ),
                "root_dependencies": [],
                "root_dependency_count": 0,
            },
            "finalization": {
                "completion_transition_output": self.completion_relative,
                "evidence_index_output": "requires-authenticated-work-queue-completion-v1",
                "qualification_receipt_output": "requires-authenticated-work-queue-completion-v1",
                "required_validator": "proofs/check-m1-evidence-index.py",
            },
            "format": self.validator.PLAN_FORMAT,
            "nonclaim": self.validator.PLAN_NONCLAIM,
            "obligation_slots": self.validator.expected_obligation_slots(candidate),
            "path_resolutions": copy.deepcopy(candidate["path_resolutions"]),
            "planner_sha256": digest_bytes(
                (self.ferric / "proofs/m1-qualification/planner.py").read_bytes()
            ),
            "requirements": {
                "format": requirements["format"],
                "path": "proofs/M1_REQUIREMENTS.json",
                "sha256": digest_bytes(
                    (self.ferric / "proofs/M1_REQUIREMENTS.json").read_bytes()
                ),
            },
            "source_closures": source_closures,
            "sources": copy.deepcopy(candidate["sources"]),
            "target": self.validator.TARGET,
            "trusted_validators": validators,
        }
        binding_work = [
            {
                "expected_artifact": slot["expected_artifact"],
                "id": slot["binding"]["id"].replace("binding.", "work.", 1),
                "producer": slot["producer"],
                "state": "missing",
                "subject": f"binding:{slot['binding']['id']}",
            }
            for slot in binding_slots
        ]
        tcb_work = []
        for record in candidate["tcb"]:
            artifact = artifact_by_id[record["artifact_id"]]
            tcb_work.append(
                {
                    "expected_artifact": {
                        key: artifact[key] for key in ("id", "kind", "path")
                    },
                    "id": f"work.{record['id']}",
                    "producer": {
                        "availability": "available",
                        "command": [
                            "python3",
                            "-I",
                            "proofs/m1-qualification/produce-tcb-report.py",
                            "FERRIC_REPO",
                            "FE2O3_REPO",
                            "PLAN_DIR",
                            record["id"],
                        ],
                        "role": f"ferric-{record['kind'].lower()}-tcb-reporter",
                    },
                    "state": "missing",
                    "subject": f"tcb:{record['id']}",
                }
            )
        receipt_work = {
            "expected_artifact": {
                "id": self.receipt_id,
                "kind": "QualificationReceipt",
                "path": self.report_relative,
            },
            "id": "work.qualification.m1",
            "producer": {
                "availability": "available",
                "command": [
                    "python3",
                    "-I",
                    "proofs/m1-qualification/produce-qualification-receipt.py",
                    "FERRIC_REPO",
                    "FE2O3_REPO",
                    "PLAN_DIR",
                    "QUALIFICATION_RUN_ROOT",
                ],
                "role": "ferric-m1-qualification-intake-finalizer",
            },
            "state": "blocked-on-all-validated-evidence",
            "subject": "qualification:M1",
        }
        items = sorted(
            [*binding_work, *tcb_work, receipt_work], key=lambda record: record["id"]
        )
        item_ids = [record["id"] for record in items]
        queue = {
            "authority": self.validator.PLAN_AUTHORITY,
            "counts": {
                "available_producer_items": len(item_ids),
                "missing_items": len(item_ids),
                "missing_producer_items": 0,
            },
            "format": self.validator.WORK_FORMAT,
            "items": items,
            "nonclaim": self.validator.PLAN_NONCLAIM,
            "plan_path": "plan.json",
            "plan_sha256": digest_bytes(canonical_bytes(plan)),
            "status": "INCOMPLETE",
        }
        return plan, queue

    def _make_completion(
        self, index: dict[str, Any], gates: list[dict[str, Any]]
    ) -> dict[str, Any]:
        plan, queue = self._plan_queue(index)
        item_ids = [record["id"] for record in queue["items"]]
        return {
            "authority": self.validator.COMPLETION_AUTHORITY,
            "candidate_index_sha256": digest_bytes(
                canonical_bytes(self._index_projection(index))
            ),
            "completed_item_ids": item_ids,
            "counts": {"completed_items": len(item_ids), "missing_items": 0},
            "format": self.validator.COMPLETION_FORMAT,
            "gate_roster_sha256": canonical_digest(gates),
            "plan_path": "plan.json",
            "plan_sha256": digest_bytes(canonical_bytes(plan)),
            "queue_path": "missing-work.json",
            "queue_sha256": digest_bytes(canonical_bytes(queue)),
            "status": "COMPLETE",
        }

    def _bind_completion(
        self, transcript: dict[str, Any], completion: dict[str, Any]
    ) -> None:
        raw = canonical_bytes(completion)
        transcript["work_queue_completion_relative_path"] = self.completion_relative
        transcript["work_queue_completion_sha256"] = digest_bytes(raw)
        transcript["work_queue_completion_size_bytes"] = len(raw)

    def _make_transcript(self, index: dict[str, Any]) -> dict[str, Any]:
        validators = self._validators()
        environment = self._environment()
        tools = self._tools(validators)
        gates = self._gates(index)
        transcript = {
            "all_required_gates_passed": True,
            "environment": environment,
            "environment_identity_sha256": canonical_digest(environment),
            "finished_at_utc": "2026-08-21T00:10:00Z",
            "format": self.validator.TRANSCRIPT_FORMAT,
            "gate_roster_sha256": canonical_digest(gates),
            "gates": gates,
            "index_roster_sha256": canonical_digest(self._index_projection(index)),
            "milestone": "M1",
            "no_failed_gates": True,
            "no_skipped_gates": True,
            "protocol": self.validator.QUALIFICATION_PROTOCOL,
            "qualification_id_sha256": "",
            "requirements_sha256": index["requirements_sha256"],
            "result": "pass",
            "run_id": "123e4567-e89b-42d3-a456-426614174001",
            "source_closure_sha256s": {
                record["id"]: record["source_closure_sha256"]
                for record in index["sources"]
            },
            "source_roster_sha256": canonical_digest(index["sources"]),
            "started_at_utc": "2026-08-21T00:00:00Z",
            "target": copy.deepcopy(self.validator.TARGET_VALUE),
            "target_identity_sha256": canonical_digest(self.validator.TARGET_VALUE),
            "tcb_identity_sha256s": {
                record["id"]: record["identity_sha256"] for record in index["tcb"]
            },
            "tcb_roster_sha256": canonical_digest(index["tcb"]),
            "tool_roster_sha256": canonical_digest(tools),
            "tools": tools,
            "validator_roster_sha256": canonical_digest(validators),
            "work_queue_completion_relative_path": self.completion_relative,
            "work_queue_completion_sha256": digest_bytes(b"pending completion"),
            "work_queue_completion_size_bytes": 1,
        }
        transcript["qualification_id_sha256"] = self.validator.qualification_identity(
            transcript
        )
        return transcript

    def _make_report(
        self, index: dict[str, Any], transcript: dict[str, Any]
    ) -> dict[str, Any]:
        validators = self._validators()
        nonself = [
            record for record in index["artifacts"] if record["id"] != self.receipt_id
        ]
        transcript_raw = canonical_bytes(transcript)
        requirements = json.loads(
            (self.ferric / "proofs/M1_REQUIREMENTS.json").read_text(encoding="ascii")
        )
        return {
            "artifact_count": len(index["artifacts"]),
            "artifact_roster_sha256": canonical_digest(nonself),
            "assurance_count": 17,
            "authority": self.validator.AUTHORITY,
            "binding_count": len(index["evidence_bindings"]),
            "binding_roster_sha256": canonical_digest(index["evidence_bindings"]),
            "format": self.validator.REPORT_FORMAT,
            "gate_ids": list(self.validator.GATE_IDS),
            "index_roster_sha256": canonical_digest(self._index_projection(index)),
            "milestone": "M1",
            "nonclaim": self.validator.NONCLAIM,
            "obligation_roster_sha256": canonical_digest(index["obligations"]),
            "path_count": 39,
            "path_roster_sha256": canonical_digest(index["path_resolutions"]),
            "protocol": PROTOCOL,
            "qualification_id_sha256": transcript["qualification_id_sha256"],
            "receipt_artifact": {
                "id": self.receipt_id,
                "kind": "QualificationReceipt",
                "path": self.report_relative,
            },
            "requirements_roster_sha256": canonical_digest(requirements),
            "requirements_sha256": index["requirements_sha256"],
            "result": "pass",
            "roadmap_count": 33,
            "source_closure_roster": self._source_closure_roster(index),
            "source_roster": copy.deepcopy(index["sources"]),
            "source_roster_sha256": canonical_digest(index["sources"]),
            "target": self.validator.TARGET,
            "tcb_roster": copy.deepcopy(index["tcb"]),
            "tcb_roster_sha256": canonical_digest(index["tcb"]),
            "transcript_relative_path": self.transcript_relative,
            "transcript_sha256": digest_bytes(transcript_raw),
            "transcript_size_bytes": len(transcript_raw),
            "validator_count": len(validators),
            "validator_roster": validators,
            "validator_roster_sha256": canonical_digest(validators),
            "work_queue_completion_relative_path": transcript[
                "work_queue_completion_relative_path"
            ],
            "work_queue_completion_sha256": transcript["work_queue_completion_sha256"],
            "work_queue_completion_size_bytes": transcript[
                "work_queue_completion_size_bytes"
            ],
        }

    def reset(self) -> None:
        self.index = copy.deepcopy(self.base_index)
        self.transcript = copy.deepcopy(self.base_transcript)
        self.report = copy.deepcopy(self.base_report)
        self.completion = copy.deepcopy(self.base_completion)
        self.plan_mutation = None
        self.materialize()

    def seal_transcript(self) -> None:
        self.transcript["environment_identity_sha256"] = canonical_digest(
            self.transcript["environment"]
        )
        self.transcript["target_identity_sha256"] = canonical_digest(
            self.transcript["target"]
        )
        self.transcript["tool_roster_sha256"] = canonical_digest(
            self.transcript["tools"]
        )
        self.transcript["gate_roster_sha256"] = canonical_digest(
            self.transcript["gates"]
        )
        self.completion = self._make_completion(self.index, self.transcript["gates"])
        self.seal_completion()

    def seal_completion(self) -> None:
        self._bind_completion(self.transcript, self.completion)
        self.transcript["qualification_id_sha256"] = (
            self.validator.qualification_identity(self.transcript)
        )
        self.report["qualification_id_sha256"] = self.transcript[
            "qualification_id_sha256"
        ]
        self.report["work_queue_completion_relative_path"] = self.transcript[
            "work_queue_completion_relative_path"
        ]
        self.report["work_queue_completion_sha256"] = self.transcript[
            "work_queue_completion_sha256"
        ]
        self.report["work_queue_completion_size_bytes"] = self.transcript[
            "work_queue_completion_size_bytes"
        ]

    def materialize(
        self, *, canonical_report: bool = True, canonical_transcript: bool = True
    ) -> None:
        self.foundation.reset_evidence()
        plan, queue = self._plan_queue(self.index)
        if self.plan_mutation is not None:
            self.plan_mutation(plan)
            queue["plan_sha256"] = digest_bytes(canonical_bytes(plan))
            self.completion["plan_sha256"] = queue["plan_sha256"]
            self.completion["queue_sha256"] = digest_bytes(canonical_bytes(queue))
            self.seal_completion()
        (self.evidence / "plan.json").write_bytes(canonical_bytes(plan))
        (self.evidence / "missing-work.json").write_bytes(canonical_bytes(queue))
        (self.evidence / self.completion_relative).write_bytes(
            canonical_bytes(self.completion)
        )
        transcript_raw = canonical_bytes(self.transcript)
        if not canonical_transcript:
            transcript_raw = json.dumps(self.transcript).encode("ascii")
        transcript_path = self.evidence / self.transcript_relative
        transcript_path.parent.mkdir(parents=True, exist_ok=True)
        transcript_path.write_bytes(transcript_raw)
        self.report["transcript_sha256"] = digest_bytes(transcript_raw)
        self.report["transcript_size_bytes"] = len(transcript_raw)
        report_raw = canonical_bytes(self.report)
        if not canonical_report:
            report_raw = json.dumps(self.report).encode("ascii")
        report_path = self.evidence / self.report_relative
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_bytes(report_raw)
        artifact = self._artifact(self.index, self.receipt_id)
        artifact["path"] = self.report_relative
        artifact["sha256"] = digest_bytes(report_raw)
        artifact["size_bytes"] = len(report_raw)
        self.context = {
            "artifact": copy.deepcopy(artifact),
            "artifact_absolute_path": str(report_path),
            "format": self.validator.INDEX_FORMAT,
            "index": self.index,
            "repository_absolute_paths": {
                "fe2o3": str(self.fe2o3),
                "ferric": str(self.ferric),
            },
            "requirements_sha256": self.index["requirements_sha256"],
            "sources": copy.deepcopy(self.index["sources"]),
            "subject": "qualification:M1",
            "tcb": copy.deepcopy(self.index["tcb"]),
        }

    def invoke(
        self, *, protocol: str = PROTOCOL, payload: bytes | None = None
    ) -> tuple[bool, str]:
        if payload is None:
            payload = canonical_bytes(self.context, compact=True)
        result = subprocess.run(
            [sys.executable, "-I", str(self.validator_path), protocol],
            check=False,
            input=payload,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=120,
            cwd=self.ferric,
            env={"PATH": os.environ.get("PATH", "")},
        )
        return result.returncode == 0, result.stdout.decode("utf-8", errors="replace")


def run_case(
    fixture: Fixture,
    name: str,
    marker: str,
    mutate: Mutation,
    *,
    materialize: bool = True,
) -> None:
    fixture.reset()
    mutate(fixture)
    if materialize:
        fixture.materialize()
    passed, output = fixture.invoke()
    if passed or marker not in output:
        raise AssertionError(
            f"hostile qualification case {name!r} did not fail with {marker!r}:\n{output}"
        )
    print(f"PASS: hostile {name}")


def artifact(index: dict[str, Any], identifier: str) -> dict[str, Any]:
    return next(record for record in index["artifacts"] if record["id"] == identifier)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="ferric-m1-receipt-policy-") as raw:
        fixture = Fixture(Path(raw))
        passed, output = fixture.invoke()
        if not passed or f"PASS: {PROTOCOL}" not in output:
            raise AssertionError(
                f"canonical qualification receipt was rejected:\n{output}"
            )
        print("PASS: canonical complete M1 qualification receipt")

        cases: list[tuple[str, str, Mutation]] = []

        def add(name: str, marker: str, mutate: Mutation) -> None:
            cases.append((name, marker, mutate))

        add(
            "partial roadmap roster",
            "33 roadmap and 17 assurance",
            lambda item: item.index["obligations"].pop(0),
        )
        add(
            "partial assurance roster",
            "33 roadmap and 17 assurance",
            lambda item: item.index["obligations"].pop(),
        )
        add(
            "duplicate closure row",
            "missing, duplicated, or reordered",
            lambda item: item.index["obligations"].__setitem__(
                1, copy.deepcopy(item.index["obligations"][0])
            ),
        )
        add(
            "status weakening",
            "status was weakened or promoted",
            lambda item: item.index["obligations"][33].__setitem__(
                "closure_status", "Validated"
            ),
        )
        add(
            "unsupported promotion",
            "status was weakened or promoted",
            lambda item: next(
                record
                for record in item.index["obligations"]
                if record["closure_status"] == "Unsupported"
            ).__setitem__("closure_status", "Proved"),
        )
        add(
            "split roadmap receipts",
            "qualification receipt is unavailable",
            lambda item: item.index["obligations"][1].__setitem__(
                "receipt_artifact_id", item.index["tcb"][0]["artifact_id"]
            ),
        )
        add(
            "binding omission",
            "evidence closure is incomplete",
            lambda item: item.index["evidence_bindings"].pop(0),
        )
        add(
            "duplicate binding",
            "duplicate or unknown qualification binding",
            lambda item: item.index["evidence_bindings"].__setitem__(
                1, copy.deepcopy(item.index["evidence_bindings"][0])
            ),
        )
        add(
            "unknown binding field",
            "fields drifted",
            lambda item: item.index["evidence_bindings"][0].__setitem__(
                "claimed_pass", True
            ),
        )
        add(
            "artifact omission",
            "unavailable",
            lambda item: item.index["artifacts"].pop(0),
        )
        add(
            "duplicate artifact",
            "duplicate qualification artifact",
            lambda item: item.index["artifacts"].__setitem__(
                1, copy.deepcopy(item.index["artifacts"][0])
            ),
        )
        add(
            "unknown artifact kind",
            "unknown qualification artifact kind",
            lambda item: item.index["artifacts"][0].__setitem__(
                "kind", "SelfReportedPass"
            ),
        )
        add(
            "source replay",
            "source replay or identity drifted",
            lambda item: item.index["sources"][1].__setitem__(
                "commit", digest_bytes(b"replayed Ferric commit")[:40]
            ),
        )
        add(
            "source closure substitution",
            "source closure drifted",
            lambda item: item.index["sources"][0].__setitem__(
                "source_closure_sha256", digest_bytes(b"substituted closure")
            ),
        )
        add(
            "TCB omission",
            "TCB roster is incomplete",
            lambda item: item.index["tcb"].pop(),
        )
        add(
            "TCB identity substitution",
            "TCB identity, order, or kind drifted",
            lambda item: item.index["tcb"][0].__setitem__(
                "identity_sha256", digest_bytes(b"substituted TCB")
            ),
        )
        add(
            "target substitution",
            "target identity drifted",
            lambda item: (
                item.transcript["target"].__setitem__("architecture", "gfx950"),
                item.seal_transcript(),
            ),
        )
        add(
            "device substitution",
            "target device or host identity drifted",
            lambda item: (
                item.transcript["environment"]["device"].__setitem__(
                    "marketing_name", "Synthetic GPU"
                ),
                item.seal_transcript(),
            ),
        )
        add(
            "tool omission",
            "tool roster is incomplete",
            lambda item: (
                item.transcript["tools"].pop(),
                item.seal_transcript(),
            ),
        )
        add(
            "tool identity substitution",
            "tool identity drifted",
            lambda item: (
                item.transcript["tools"][-1].__setitem__(
                    "identity_sha256", digest_bytes(b"substituted validator")
                ),
                item.seal_transcript(),
            ),
        )
        add(
            "gate omission",
            "gate roster is incomplete",
            lambda item: (
                item.transcript["gates"].pop(),
                item.seal_transcript(),
            ),
        )
        add(
            "failed gate",
            "failed, skipped, replayed, or incomplete",
            lambda item: (
                item.transcript["gates"][1].__setitem__("result", "fail"),
                item.seal_transcript(),
            ),
        )
        add(
            "proof gate partial",
            "failed, skipped, replayed, or incomplete",
            lambda item: (
                item.transcript["gates"][3]["artifact_ids"].pop(),
                item.seal_transcript(),
            ),
        )
        add(
            "hardware gate partial",
            "failed, skipped, replayed, or incomplete",
            lambda item: (
                item.transcript["gates"][1]["binding_ids"].pop(),
                item.seal_transcript(),
            ),
        )
        add(
            "performance gate partial",
            "failed, skipped, replayed, or incomplete",
            lambda item: (
                item.transcript["gates"][2]["artifact_ids"].pop(),
                item.seal_transcript(),
            ),
        )
        add(
            "quality gate skipped",
            "failed, skipped, replayed, or incomplete",
            lambda item: (
                item.transcript["gates"][4].__setitem__("result", "skipped"),
                item.seal_transcript(),
            ),
        )
        add(
            "duplicate gate output",
            "failed, skipped, replayed, or incomplete",
            lambda item: (
                item.transcript["gates"][1].__setitem__(
                    "output_sha256", item.transcript["gates"][0]["output_sha256"]
                ),
                item.seal_transcript(),
            ),
        )
        add(
            "self-reported result only",
            "partial, replayed, self-reported, or inconsistent",
            lambda item: (
                item.transcript.__setitem__("no_skipped_gates", False),
                item.seal_transcript(),
            ),
        )
        add(
            "qualification id replay",
            "partial, replayed, self-reported, or inconsistent",
            lambda item: item.transcript.__setitem__(
                "qualification_id_sha256", digest_bytes(b"old qualification run")
            ),
        )
        add(
            "unknown transcript field",
            "fields drifted",
            lambda item: item.transcript.__setitem__("self_attested", True),
        )
        add(
            "receipt roster substitution",
            "content, roster, status, or identity drifted",
            lambda item: item.report.__setitem__("roadmap_count", 32),
        )
        add(
            "unknown receipt field",
            "fields drifted",
            lambda item: item.report.__setitem__("promoted_authority", True),
        )
        add(
            "partial work completion",
            "completed-work transition is partial, replayed, or inconsistent",
            lambda item: (
                item.completion["completed_item_ids"].pop(),
                item.seal_completion(),
            ),
        )
        add(
            "weakened work completion status",
            "completed-work transition is partial, replayed, or inconsistent",
            lambda item: (
                item.completion.__setitem__("status", "INCOMPLETE"),
                item.seal_completion(),
            ),
        )
        add(
            "replayed work completion candidate",
            "completed-work transition is partial, replayed, or inconsistent",
            lambda item: (
                item.completion.__setitem__(
                    "candidate_index_sha256",
                    digest_bytes(b"replayed candidate index"),
                ),
                item.seal_completion(),
            ),
        )
        add(
            "nested plan source replay",
            "plan nested identity differs from the qualified closure",
            lambda item: setattr(
                item,
                "plan_mutation",
                lambda plan: plan["sources"][0].__setitem__(
                    "commit", digest_bytes(b"replayed plan source")[:40]
                ),
            ),
        )

        for name, marker, mutate in cases:
            run_case(fixture, name, marker, mutate)

        fixture.reset()
        passed, output = fixture.invoke(protocol="ferric.m1-validator.fake.v1")
        if passed or "protocol mismatch" not in output:
            raise AssertionError(
                f"hostile protocol substitution was accepted:\n{output}"
            )
        print("PASS: hostile protocol substitution")

        fixture.reset()
        duplicate_context = canonical_bytes(fixture.context, compact=True).decode(
            "ascii"
        )
        needle = f'"format":"{fixture.validator.INDEX_FORMAT}",'
        duplicate_context = duplicate_context.replace(needle, needle + needle, 1)
        passed, output = fixture.invoke(payload=duplicate_context.encode("ascii"))
        if passed or "duplicate JSON key" not in output:
            raise AssertionError(
                f"hostile duplicate context key was accepted:\n{output}"
            )
        print("PASS: hostile duplicate context key")

        fixture.reset()
        passed, output = fixture.invoke(
            payload=json.dumps(fixture.context).encode("ascii")
        )
        if passed or "not canonical JSON" not in output:
            raise AssertionError(
                f"hostile noncanonical context was accepted:\n{output}"
            )
        print("PASS: hostile noncanonical context")

        fixture.reset()
        fixture.materialize(canonical_transcript=False)
        passed, output = fixture.invoke()
        if passed or "not canonical JSON" not in output:
            raise AssertionError(
                f"hostile noncanonical transcript was accepted:\n{output}"
            )
        print("PASS: hostile noncanonical transcript")

        fixture.reset()
        transcript = fixture.evidence / fixture.transcript_relative
        transcript.unlink()
        transcript.symlink_to(fixture.evidence / fixture.report_relative)
        passed, output = fixture.invoke()
        if passed or "path contains a symlink" not in output:
            raise AssertionError(f"hostile transcript symlink was accepted:\n{output}")
        print("PASS: hostile transcript symlink")

        fixture.reset()
        source_artifact = artifact(
            fixture.index, fixture.index["sources"][0]["source_closure_artifact_id"]
        )
        source_path = fixture.evidence / source_artifact["path"]
        source_path.unlink()
        source_path.symlink_to(fixture.evidence / fixture.report_relative)
        passed, output = fixture.invoke()
        if passed or "path contains a symlink" not in output:
            raise AssertionError(f"hostile artifact symlink was accepted:\n{output}")
        print("PASS: hostile artifact symlink")

        fixture.reset()
        first = fixture.evidence / fixture.index["artifacts"][0]["path"]
        second_record = fixture.index["artifacts"][1]
        second = fixture.evidence / second_record["path"]
        second.unlink()
        os.link(first, second)
        second_record["sha256"] = digest_bytes(first.read_bytes())
        second_record["size_bytes"] = first.stat().st_size
        fixture.context["index"] = fixture.index
        passed, output = fixture.invoke()
        if passed or not any(
            marker in output
            for marker in ("single-link regular file", "must not be hard-linked")
        ):
            raise AssertionError(f"hostile artifact hard link was accepted:\n{output}")
        print("PASS: hostile artifact hard link")

        fixture.reset()
        (fixture.ferric / "untracked-qualification-input").write_text(
            "not committed\n", encoding="ascii"
        )
        passed, output = fixture.invoke()
        if passed or "not the exact clean Git tree" not in output:
            raise AssertionError(f"hostile dirty source was accepted:\n{output}")
        print("PASS: hostile dirty source tree")
        (fixture.ferric / "untracked-qualification-input").unlink()

        fixture.reset()
        original_fstat = fixture.validator.os.fstat
        calls = 0

        def unstable_fstat(descriptor: int) -> os.stat_result:
            nonlocal calls
            calls += 1
            result = original_fstat(descriptor)
            if calls == 2:
                values = list(result)
                values[8] = result.st_mtime + 1
                return os.stat_result(values)
            return result

        fixture.validator.os.fstat = unstable_fstat
        output_buffer = io.StringIO()
        try:
            with contextlib.redirect_stderr(output_buffer):
                fixture.validator.validate(fixture.context)
        except SystemExit:
            pass
        else:
            raise AssertionError("hostile simulated TOCTOU input was accepted")
        finally:
            fixture.validator.os.fstat = original_fstat
        if "changed while it was read" not in output_buffer.getvalue():
            raise AssertionError(
                "simulated TOCTOU did not fail at the stable-read boundary:\n"
                f"{output_buffer.getvalue()}"
            )
        print("PASS: hostile simulated in-read TOCTOU")

        print(f"PASS: {len(cases) + 10} hostile qualification-receipt fixtures")


if __name__ == "__main__":
    main()
