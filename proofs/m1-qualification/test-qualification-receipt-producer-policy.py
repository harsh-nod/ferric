#!/usr/bin/env python3
"""Exercise hostile qualification-run intake before receipt publication."""

from __future__ import annotations

import contextlib
import copy
from datetime import datetime, timedelta, timezone
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


Mutation = Callable[[dict[str, Any], Path], None]


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def load_producer(repo: Path) -> Any:
    path = repo / "proofs/m1-qualification/produce-qualification-receipt.py"
    spec = importlib.util.spec_from_file_location("qualification_finalizer", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    sys.dont_write_bytecode = True
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_receipt_policy(repo: Path) -> Any:
    path = repo / "proofs/m1/evidence/test-qualification-receipt-policy.py"
    spec = importlib.util.spec_from_file_location("qualification_receipt_policy", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    sys.dont_write_bytecode = True
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def gate_fixture(
    producer: Any,
    ferric: Path,
    _fe2o3: Path,
    _candidate_path: Path,
    _artifact_root: Path,
    candidate: dict[str, Any],
    tools: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], datetime, datetime]:
    checker_sha256 = producer.digest_bytes(
        (ferric / "proofs/check-m1-evidence-index.py").read_bytes()
    )
    python_sha256 = next(
        record["identity_sha256"]
        for record in tools
        if record["id"] == "runtime.python"
    )
    candidate_sha256 = producer.digest_bytes(producer.canonical_bytes(candidate))
    rosters = producer.gate_rosters(candidate)
    run_start = datetime.now(timezone.utc).replace(microsecond=0)
    gates = []
    for ordinal, gate_id in enumerate(producer.GATE_IDS):
        started = run_start + timedelta(seconds=ordinal * 2)
        finished = started + timedelta(seconds=1)
        artifacts, bindings = rosters[gate_id]
        gates.append(
            {
                "artifact_ids": artifacts,
                "binding_ids": bindings,
                "command_sha256": producer.canonical_digest(
                    {
                        "candidate_sha256": candidate_sha256,
                        "checker_sha256": checker_sha256,
                        "gate_id": gate_id,
                        "protocol": producer.PRE_RECEIPT_PROTOCOL,
                        "runtime_python_sha256": python_sha256,
                    }
                ),
                "finished_at_utc": producer.utc_timestamp(finished),
                "id": gate_id,
                "output_sha256": producer.digest_bytes(
                    (
                        f"PASS: {producer.PRE_RECEIPT_PROTOCOL} gate={gate_id} "
                        f"candidate_sha256={candidate_sha256}\n"
                    ).encode("ascii")
                ),
                "result": "pass",
                "started_at_utc": producer.utc_timestamp(started),
            }
        )
    return gates, run_start, run_start + timedelta(seconds=13)


def finalizer_integration(repo: Path, producer: Any, temporary: Path) -> None:
    temporary.mkdir(mode=0o700)
    policy = load_receipt_policy(repo)
    fixture = policy.Fixture(temporary / "receipt-fixture")
    plan_root = fixture.evidence
    plan_root.chmod(0o700)
    candidate = fixture._index_projection(fixture.base_index)
    plan, queue = fixture._plan_queue(fixture.base_index)
    (plan_root / "plan.json").write_bytes(canonical_bytes(plan))
    (plan_root / "missing-work.json").write_bytes(canonical_bytes(queue))
    for path in (
        plan_root / fixture.completion_relative,
        plan_root / fixture.report_relative,
        plan_root / "evidence-index.json",
    ):
        if path.exists() or path.is_symlink():
            path.unlink()
    transcript_directory = plan_root / "qualification-transcripts"
    if transcript_directory.exists():
        shutil.rmtree(transcript_directory)
    requirements = json.loads(
        (fixture.ferric / "proofs/M1_REQUIREMENTS.json").read_text(encoding="ascii")
    )
    requirements_raw = canonical_bytes(requirements)
    queue_raw = canonical_bytes(queue)
    originals = (
        producer.validate_plan,
        producer.derive_candidate_index,
        producer.validator_roster,
        producer.execute_pre_receipt_gates,
    )

    def fixed_plan(
        _ferric: Path, _fe2o3: Path, _plan: Path, *, replay: bool = True
    ) -> tuple[dict[str, Any], dict[str, Any], bytes, dict[str, Any], bytes]:
        del replay
        return plan, queue, queue_raw, requirements, requirements_raw

    producer.validate_plan = fixed_plan
    producer.derive_candidate_index = lambda *_args: (candidate, candidate["tcb"])
    producer.validator_roster = lambda *_args: fixture._validators()
    producer.execute_pre_receipt_gates = lambda *args: gate_fixture(producer, *args)
    run_parent = temporary / "qualification-runs"
    run_parent.mkdir(mode=0o700)
    run_root = run_parent / "run"
    try:
        producer.prepare_candidate(
            str(fixture.ferric),
            str(fixture.fe2o3),
            str(plan_root),
            str(run_root),
        )
        checker = fixture.ferric / "proofs/check-m1-evidence-index.py"
        precheck = subprocess.run(
            [
                sys.executable,
                "-I",
                str(checker),
                producer.PRE_RECEIPT_PROTOCOL,
                "evidence-index",
                str(fixture.ferric),
                str(run_root / "candidate-index.json"),
                str(plan_root),
                str(fixture.fe2o3),
            ],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=120,
            cwd=fixture.ferric,
            env={"PATH": os.environ.get("PATH", "")},
        )
        if precheck.returncode != 0 or "gate=evidence-index" not in precheck.stdout:
            raise AssertionError(
                "prepared candidate failed the real pre-receipt checker:\n"
                f"{precheck.stdout}"
            )
        tools_root = temporary / "integration-tools"
        tools_root.mkdir(mode=0o700)
        tool_records = []
        for identifier, output in (
            ("compiler.cargo", "cargo 1.97.1 (integration)"),
            ("compiler.rustc", "rustc 1.97.1 (integration)"),
        ):
            path = tools_root / identifier
            path.write_text(f"#!/bin/sh\nprintf '%s\\n' '{output}'\n", encoding="ascii")
            path.chmod(0o700)
            tool_records.append({"binary_absolute_path": str(path), "id": identifier})
        intake = {
            "candidate_index_relative_path": "candidate-index.json",
            "environment": fixture._environment(),
            "format": producer.INTAKE_FORMAT,
            "run_id": "123e4567-e89b-42d3-a456-426614174111",
            "tools": tool_records,
        }
        intake_path = run_root / "intake.json"
        intake_path.write_bytes(canonical_bytes(intake))
        intake_path.chmod(0o600)
        producer.produce(
            str(fixture.ferric),
            str(fixture.fe2o3),
            str(plan_root),
            str(run_root),
        )
    finally:
        (
            producer.validate_plan,
            producer.derive_candidate_index,
            producer.validator_roster,
            producer.execute_pre_receipt_gates,
        ) = originals

    final_index = json.loads(
        (plan_root / "evidence-index.json").read_text(encoding="ascii")
    )
    receipt = next(
        record
        for record in final_index["artifacts"]
        if record["id"] == "artifact.qualification.m1"
    )
    context = {
        "artifact": receipt,
        "artifact_absolute_path": str(plan_root / receipt["path"]),
        "format": producer.INDEX_FORMAT,
        "index": final_index,
        "repository_absolute_paths": {
            "fe2o3": str(fixture.fe2o3),
            "ferric": str(fixture.ferric),
        },
        "requirements_sha256": final_index["requirements_sha256"],
        "sources": final_index["sources"],
        "subject": "qualification:M1",
        "tcb": final_index["tcb"],
    }
    payload = (
        json.dumps(context, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    )
    validator = fixture.ferric / "proofs/m1/evidence/validate-qualification-receipt.py"
    validated = subprocess.run(
        [sys.executable, "-I", str(validator), producer.VALIDATOR_PROTOCOL],
        check=False,
        input=payload,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=120,
        cwd=fixture.ferric,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if validated.returncode != 0 or "PASS:" not in validated.stdout:
        raise AssertionError(
            f"published receipt failed the authoritative validator:\n{validated.stdout}"
        )
    print("PASS: prepare-candidate -> precheck -> produce -> receipt validator")


class Fixture:
    def __init__(self, producer: Any, temporary: Path) -> None:
        self.producer = producer
        self.temporary = temporary
        self.root = temporary / "qualification-run"
        self.root.mkdir(mode=0o700)
        self.tools_root = temporary / "tools"
        self.tools_root.mkdir(mode=0o700)
        self.candidate = {
            "artifacts": [],
            "evidence_bindings": [],
            "format": producer.INDEX_FORMAT,
            "obligations": [],
            "path_resolutions": [],
            "requirements_sha256": producer.digest_bytes(b"requirements"),
            "sources": [],
            "tcb": [],
        }
        self.base = self._intake()
        self.materialize(self.base)

    def _tool(self, identifier: str, version_output: str) -> dict[str, str]:
        path = self.tools_root / identifier
        path.write_text(
            f"#!/bin/sh\nprintf '%s\\n' '{version_output}'\n",
            encoding="ascii",
        )
        path.chmod(0o700)
        return {
            "binary_absolute_path": str(path),
            "id": identifier,
        }

    def _environment(self) -> dict[str, Any]:
        digest = self.producer.digest_bytes
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
                "module_sha256": digest(b"module"),
                "name": "amdgpu",
                "version": "6.14.14-test",
            },
            "firmware": {
                "bundle_sha256": digest(b"firmware"),
                "package_version": "20260825.1",
            },
            "host": {
                "kernel_sha256": digest(b"kernel"),
                "machine": "x86_64",
                "os_release_sha256": digest(b"os-release"),
            },
            "rocm": {
                "installation_sha256": digest(b"rocm"),
                "version": "7.1.0",
            },
        }

    def _intake(self) -> dict[str, Any]:
        return {
            "candidate_index_relative_path": "candidate-index.json",
            "environment": self._environment(),
            "format": self.producer.INTAKE_FORMAT,
            "run_id": "123e4567-e89b-42d3-a456-426614174001",
            "tools": [
                self._tool("compiler.cargo", "cargo 1.97.1 (test)"),
                self._tool("compiler.rustc", "rustc 1.97.1 (test)"),
            ],
        }

    def clear_run_files(self) -> None:
        for path in self.root.iterdir():
            if path.is_symlink() or path.is_file():
                path.unlink()
            elif path.is_dir():
                shutil.rmtree(path)

    def materialize(
        self, value: dict[str, Any], *, noncanonical_intake: bool = False
    ) -> None:
        self.clear_run_files()
        candidate = self.root / value["candidate_index_relative_path"]
        candidate.write_bytes(canonical_bytes(self.candidate))
        candidate.chmod(0o600)
        intake = self.root / "intake.json"
        if noncanonical_intake:
            intake.write_text(json.dumps(value), encoding="ascii")
        else:
            intake.write_bytes(canonical_bytes(value))
        intake.chmod(0o600)

    def invoke(self) -> tuple[bool, str]:
        output = io.StringIO()
        try:
            with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                self.producer.load_intake(self.root, self.candidate)
        except SystemExit:
            return False, output.getvalue()
        return True, output.getvalue()


def run_case(
    fixture: Fixture,
    name: str,
    marker: str,
    mutate: Mutation,
    *,
    materialize: bool = True,
    noncanonical_intake: bool = False,
) -> None:
    value = copy.deepcopy(fixture.base)
    mutate(value, fixture.root)
    if materialize:
        fixture.materialize(value, noncanonical_intake=noncanonical_intake)
    passed, output = fixture.invoke()
    if passed or marker not in output:
        raise AssertionError(
            f"hostile qualification intake {name!r} did not fail with {marker!r}:\n"
            f"{output}"
        )
    print(f"PASS: hostile {name}")


def main() -> None:
    if len(sys.argv) not in {1, 3}:
        raise SystemExit(f"usage: {sys.argv[0]} [FERRIC_REPO FE2O3_OBJECT_REPO]")
    repo = (
        Path(sys.argv[1]).resolve() if len(sys.argv) == 3 else Path(__file__).parents[2]
    )
    producer = load_producer(repo)
    source = (
        repo / "proofs/m1-qualification/produce-qualification-receipt.py"
    ).read_text(encoding="ascii")
    for forbidden in (
        "import validate-qualification-receipt",
        "import check-m1-evidence-index",
    ):
        if forbidden in source:
            raise AssertionError(
                f"qualification finalizer imports or invokes trust: {forbidden}"
            )
    with tempfile.TemporaryDirectory(prefix="ferric-m1-finalizer-policy-") as raw:
        fixture = Fixture(producer, Path(raw))
        passed, output = fixture.invoke()
        if not passed:
            raise AssertionError(
                f"canonical qualification intake was rejected:\n{output}"
            )
        print("PASS: canonical qualification-run intake")

        run_case(
            fixture,
            "invalid environment",
            "target device or host identity drifted",
            lambda value, _: value["environment"]["device"].__setitem__(
                "processor", "gfx950"
            ),
        )
        run_case(
            fixture,
            "noncanonical intake",
            "not a canonical JSON object",
            lambda _value, _root: None,
            noncanonical_intake=True,
        )

        value = copy.deepcopy(fixture.base)
        fixture.materialize(value)
        (fixture.root / "intake.json").chmod(0o644)
        passed, output = fixture.invoke()
        if passed or "owner-private 0600" not in output:
            raise AssertionError(f"hostile public intake was accepted:\n{output}")
        print("PASS: hostile public intake")

        value = copy.deepcopy(fixture.base)
        fixture.materialize(value)
        candidate = fixture.root / "candidate-index.json"
        candidate.write_bytes(canonical_bytes({**fixture.candidate, "artifacts": [{}]}))
        passed, output = fixture.invoke()
        if passed or "differs from the plan-derived closure" not in output:
            raise AssertionError(
                f"hostile candidate substitution was accepted:\n{output}"
            )
        print("PASS: hostile candidate substitution")

        value = copy.deepcopy(fixture.base)
        bad_tool = fixture.tools_root / "compiler.cargo"
        bad_tool.write_text(
            "#!/bin/sh\nprintf '%s\\n' 'cargo 1.96.0 (hostile)'\n",
            encoding="ascii",
        )
        fixture.materialize(value)
        passed, output = fixture.invoke()
        if passed or "tool version drifted" not in output:
            raise AssertionError(f"hostile tool version was accepted:\n{output}")
        print("PASS: hostile measured tool version")

        publication_root = Path(raw) / "publication"
        publication_root.mkdir(mode=0o700)
        publication_fd, _ = producer.open_held_directory(
            publication_root, "test publication root"
        )
        output_path = publication_root / "output.json"
        original_fsync = producer.os.fsync

        def replace_then_fail(descriptor: int) -> None:
            metadata = os.fstat(descriptor)
            if not producer.stat.S_ISREG(metadata.st_mode):
                original_fsync(descriptor)
                return
            os.unlink(output_path.name, dir_fd=publication_fd)
            replacement = os.open(
                output_path.name,
                os.O_WRONLY | os.O_CREAT | os.O_EXCL,
                0o600,
                dir_fd=publication_fd,
            )
            os.write(replacement, b"replacement\n")
            os.close(replacement)
            raise OSError("injected publication failure")

        producer.os.fsync = replace_then_fail
        try:
            injected_output = io.StringIO()
            with (
                contextlib.redirect_stdout(injected_output),
                contextlib.redirect_stderr(injected_output),
            ):
                try:
                    producer.publish_new_at(
                        publication_fd, output_path.name, output_path, b"original\n"
                    )
                except SystemExit:
                    pass
                else:
                    raise AssertionError("injected publication failure was accepted")
        finally:
            producer.os.fsync = original_fsync
        if output_path.read_bytes() != b"replacement\n":
            raise AssertionError("publication cleanup deleted a replacement")
        os.close(publication_fd)
        print("PASS: publication cleanup preserves a concurrent replacement")

        held_root = Path(raw) / "held-publication"
        held_root.mkdir(mode=0o700)
        outside = Path(raw) / "outside"
        outside.mkdir(mode=0o700)
        held_fd, _ = producer.open_held_directory(held_root, "held publication root")
        renamed = Path(raw) / "held-publication-renamed"
        held_root.rename(renamed)
        held_root.symlink_to(outside, target_is_directory=True)
        produced_path = held_root / "held.json"
        published = producer.publish_new_at(
            held_fd, produced_path.name, produced_path, b"held\n"
        )
        if (outside / produced_path.name).exists() or not (
            renamed / produced_path.name
        ).exists():
            raise AssertionError("held directory publication followed a swapped parent")
        producer.rollback([published])
        os.close(held_fd)
        print("PASS: held directory publication rejects parent redirection")

        boundary_candidate = {
            "artifacts": [{"size_bytes": producer.MAX_TOTAL_ARTIFACT_BYTES - 10}]
        }
        producer.validate_final_artifact_size(boundary_candidate, 10)
        output = io.StringIO()
        try:
            with contextlib.redirect_stderr(output):
                producer.validate_final_artifact_size(boundary_candidate, 11)
        except SystemExit:
            pass
        else:
            raise AssertionError("receipt bytes were omitted from final size admission")
        if "final M1 artifact closure exceeds" not in output.getvalue():
            raise AssertionError(
                f"receipt-inclusive size failure was unclear:\n{output.getvalue()}"
            )
        print("PASS: final size admission includes qualification receipt bytes")

        gate_candidate = fixture.candidate
        gate_candidate_path = fixture.root / "candidate-index.json"
        checker_bytes = (repo / "proofs/check-m1-evidence-index.py").read_bytes()
        runtime_identity = producer.digest_bytes(Path(sys.executable).read_bytes())
        gate_tools = [
            {
                "id": "runtime.python",
                "identity_sha256": runtime_identity,
            }
        ]
        original_run = producer.subprocess.run
        calls = 0

        def inspect_pinned_run(arguments: list[str], **options: Any) -> Any:
            nonlocal calls
            calls += 1
            if arguments[1:3] != ["-I", "-c"] or "pass_fds" not in options:
                raise AssertionError("pre-receipt checker was not descriptor-pinned")
            descriptor = options["pass_fds"][0]
            offset = os.lseek(descriptor, 0, os.SEEK_CUR)
            os.lseek(descriptor, 0, os.SEEK_SET)
            held = os.read(descriptor, len(checker_bytes) + 1)
            os.lseek(descriptor, offset, os.SEEK_SET)
            if held != checker_bytes:
                raise AssertionError("executed checker bytes differ from held source")
            gate_id = arguments[-5]
            candidate_sha256 = producer.digest_bytes(
                producer.canonical_bytes(gate_candidate)
            )
            stdout = (
                f"PASS: {producer.PRE_RECEIPT_PROTOCOL} gate={gate_id} "
                f"candidate_sha256={candidate_sha256}\n"
            )
            return subprocess.CompletedProcess(arguments, 0, stdout, "")

        producer.subprocess.run = inspect_pinned_run
        try:
            gates, _, _ = producer.execute_pre_receipt_gates(
                repo,
                repo,
                gate_candidate_path,
                fixture.root,
                gate_candidate,
                gate_tools,
            )
        finally:
            producer.subprocess.run = original_run
        if calls != len(producer.GATE_IDS) or [
            record["id"] for record in gates
        ] != list(producer.GATE_IDS):
            raise AssertionError("descriptor-pinned gate roster is incomplete")
        print("PASS: finalizer executes held source-pinned checker bytes")

        finalizer_integration(repo, producer, Path(raw) / "integration")

    print("PASS: qualification-receipt producer hostile policy")


if __name__ == "__main__":
    main()
