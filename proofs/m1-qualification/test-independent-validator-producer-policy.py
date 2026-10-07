#!/usr/bin/env python3
"""Exercise Ferric independent-review export and external-response intake."""

from __future__ import annotations

import contextlib
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
from typing import Any, NoReturn


VALIDATOR_PROTOCOL = "ferric.m1-validator.independent-validator.v1"
EXTERNAL_PROTOCOL = "ferric.external-independent-validation.v1"
RESPONSE_FORMAT = "FERRIC-M1-INDEPENDENT-VALIDATION-RESPONSE-V1"
INDEPENDENCE_ATTESTATION = (
    "The named checker organization, repository, source closure, and executable "
    "are independent of the Ferric and fe2o3 subject source closures."
)
CASE_MATRIX = (
    ("canonical-subject", "PASS"),
    ("boundary-conforming-subject", "PASS"),
    ("obligation-substitution", "EXPECTED_FAIL"),
    ("property-substitution", "EXPECTED_FAIL"),
    ("path-substitution", "EXPECTED_FAIL"),
    ("profile-substitution", "EXPECTED_FAIL"),
    ("source-closure-substitution", "EXPECTED_FAIL"),
    ("target-substitution", "EXPECTED_FAIL"),
    ("tcb-substitution", "EXPECTED_FAIL"),
    ("malformed-status", "EXPECTED_FAIL"),
)
TCB = (
    ("tcb.compiler", "Compiler"),
    ("tcb.hardware", "Hardware"),
    ("tcb.runtime", "Runtime"),
)


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def compact_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    ).encode("ascii")


def read_json(path: Path) -> dict[str, Any]:
    raw = path.read_bytes()
    value = json.loads(raw)
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"fixture is not canonical JSON: {path}")
    return value


def write_json(path: Path, value: dict[str, Any], *, mode: int = 0o600) -> None:
    path.write_bytes(canonical_bytes(value))
    path.chmod(mode)


def private_directory(path: Path) -> None:
    path.mkdir(mode=0o700)
    path.chmod(0o700)


def command(
    arguments: list[str],
    description: str,
    *,
    cwd: Path | None = None,
    timeout: int = 300,
) -> str:
    result = subprocess.run(
        arguments,
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"{description} failed (status {result.returncode}):\n{result.stdout}")
    return result.stdout.strip()


def clone_at(source: Path, destination: Path, revision: str = "HEAD") -> None:
    command(
        ["git", "clone", "--quiet", "--no-hardlinks", str(source), str(destination)],
        f"clone {source}",
    )
    command(
        ["git", "-C", str(destination), "checkout", "--quiet", "--detach", revision],
        f"check out {revision}",
    )


def commit_fixture(repository: Path) -> None:
    command(["git", "-C", str(repository), "add", "-A"], "stage fixture")
    command(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=M1 Independent Review Policy",
            "-c",
            "user.email=m1-independent-review@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "add independent-review intake test fixture",
        ],
        "commit fixture",
    )


def run_planner(planner: Path, ferric: Path, fe2o3: Path, output: Path) -> None:
    command(
        [sys.executable, "-I", str(planner), str(ferric), str(fe2o3), str(output)],
        "run M1 planner",
        cwd=ferric,
    )


def materialize_tcb(producer: Path, ferric: Path, fe2o3: Path, plan: Path) -> None:
    for identifier, _ in TCB:
        command(
            [
                sys.executable,
                "-I",
                str(producer),
                str(ferric),
                str(fe2o3),
                str(plan),
                identifier,
            ],
            f"materialize {identifier}",
            cwd=ferric,
        )


def invoke_export(
    producer: Path, ferric: Path, fe2o3: Path, plan: Path, handoff: Path
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "-I",
            str(producer),
            "export-all",
            str(ferric),
            str(fe2o3),
            str(plan),
            str(handoff),
        ],
        cwd=ferric,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=300,
        env={"PATH": os.environ.get("PATH", "")},
    )


def invoke_intake(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    response: Path,
    binding_id: str,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "-I",
            str(producer),
            "intake",
            str(ferric),
            str(fe2o3),
            str(plan),
            str(response),
            binding_id,
        ],
        cwd=ferric,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=300,
        env={"PATH": os.environ.get("PATH", "")},
    )


def expect_intake_failure(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    response: Path,
    binding_id: str,
    expected: str,
) -> None:
    result = invoke_intake(producer, ferric, fe2o3, plan, response, binding_id)
    if result.returncode == 0 or expected not in result.stdout:
        fail(
            "independent-review intake accepted a synthetic hostile fixture; "
            f"expected {expected!r}:\n{result.stdout}"
        )


def copy_tree(source: Path, destination: Path) -> Path:
    shutil.copytree(source, destination)
    return destination


def load_module(path: Path) -> Any:
    spec = importlib.util.spec_from_file_location("independent_review_producer", path)
    if spec is None or spec.loader is None:
        fail(f"cannot load producer module: {path}")
    sys.dont_write_bytecode = True
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def expect_module_failure(action: Any, expected: str) -> str:
    output = io.StringIO()
    try:
        with contextlib.redirect_stderr(output), contextlib.redirect_stdout(output):
            action()
    except SystemExit as error:
        if error.code == 0:
            fail("in-process hostile intake exited successfully")
    else:
        fail("in-process hostile intake unexpectedly succeeded")
    observed = output.getvalue()
    if expected not in observed:
        fail(f"in-process hostile intake missed {expected!r}:\n{observed}")
    return observed


def tree_identity(root: Path) -> list[tuple[str, str, int]]:
    result = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        if path.is_dir():
            result.append((relative + "/", "", path.stat().st_mode & 0o777))
        else:
            result.append(
                (relative, digest_bytes(path.read_bytes()), path.stat().st_mode & 0o777)
            )
    return result


def make_synthetic_test_response(
    response_root: Path,
    request_path: Path,
    sentinel: Path,
) -> dict[str, Any]:
    """Create a temp-only mechanics fixture; it is not independent evidence."""
    request = read_json(request_path)
    binding = request["binding"]
    responses = response_root / "responses"
    if not response_root.exists():
        private_directory(response_root)
        private_directory(responses)
    elif not responses.is_dir():
        fail("synthetic response root lost its response directory")
    binding_root = responses / binding["id"]
    private_directory(binding_root)
    checker_root = binding_root / "checker"
    outputs_root = binding_root / "outputs"
    private_directory(checker_root)
    private_directory(outputs_root)

    source_closure = b"SYNTHETIC TEST ONLY: outside checker source closure\n"
    input_schema = b'{"synthetic_test_only":true,"type":"input-schema"}\n'
    output_schema = b'{"synthetic_test_only":true,"type":"output-schema"}\n'
    executable = (
        "#!/bin/sh\n"
        "# SYNTHETIC TEST ONLY. Intake must never run this file.\n"
        f"touch '{sentinel}'\n"
        "exit 99\n"
    ).encode("ascii")
    material = {
        "source-closure.records": (source_closure, 0o600),
        "executable.bin": (executable, 0o700),
        "input-schema.json": (input_schema, 0o600),
        "output-schema.json": (output_schema, 0o600),
    }
    for name, (raw, mode) in material.items():
        path = checker_root / name
        path.write_bytes(raw)
        path.chmod(mode)
    checker = {
        "commit": digest_bytes(b"synthetic outside checker commit")[:40],
        "executable_path": "bin/outside-m1-checker",
        "executable_sha256": digest_bytes(executable),
        "id": "outside-lab.synthetic-m1-checker",
        "input_schema_sha256": digest_bytes(input_schema),
        "organization": "outside-lab",
        "output_schema_sha256": digest_bytes(output_schema),
        "protocol": EXTERNAL_PROTOCOL,
        "repository": "outside-m1-checker",
        "source_closure_sha256": digest_bytes(source_closure),
        "tree": digest_bytes(b"synthetic outside checker tree")[:40],
        "version": "1.0.0",
    }
    results = []
    cases = {row["id"]: row for row in request["cases"]}
    for identifier, expected in CASE_MATRIX:
        output = canonical_bytes(
            {
                "case_id": identifier,
                "notice": "SYNTHETIC TEST ONLY - NOT INDEPENDENT EVIDENCE",
                "observed_status": expected,
                "request_sha256": digest_bytes(request_path.read_bytes()),
            }
        )
        output_path = outputs_root / f"{identifier}.output"
        output_path.write_bytes(output)
        output_path.chmod(0o600)
        results.append(
            {
                "exit_code": 0 if expected == "PASS" else 1,
                "expected_status": expected,
                "id": identifier,
                "input_sha256": cases[identifier]["input_sha256"],
                "observed_status": expected,
                "output_path": f"outputs/{identifier}.output",
                "output_sha256": digest_bytes(output),
                "output_size_bytes": len(output),
            }
        )
    response = {
        "binding_sha256": binding["binding_sha256"],
        "checker": checker,
        "completed_at_utc": "2026-08-24T12:01:00Z",
        "format": RESPONSE_FORMAT,
        "independence_attestation": INDEPENDENCE_ATTESTATION,
        "request_sha256": digest_bytes(request_path.read_bytes()),
        "results": results,
        "started_at_utc": "2026-08-24T12:00:00Z",
    }
    write_json(binding_root / "response.json", response)
    return response


def outer_tcb(plan: Path) -> list[dict[str, Any]]:
    result = []
    for identifier, kind in TCB:
        artifact_id = f"artifact.{identifier}"
        raw = (plan / "artifacts" / f"{artifact_id}.tcb-report.json").read_bytes()
        result.append(
            {
                "artifact_id": artifact_id,
                "id": identifier,
                "identity_sha256": digest_bytes(raw),
                "kind": kind,
            }
        )
    return result


def validate_report(
    validator: Path,
    ferric: Path,
    plan_root: Path,
    plan: dict[str, Any],
    slot: dict[str, Any],
) -> None:
    binding = slot["binding"]
    report_path = plan_root / slot["expected_artifact"]["path"]
    report_raw = report_path.read_bytes()
    resolution = next(
        row for row in plan["path_resolutions"] if row["id"] == binding["path_id"]
    )
    context = {
        "artifact": {
            "id": binding["artifact_id"],
            "kind": "ValidatorTranscript",
            "path": slot["expected_artifact"]["path"],
            "sha256": digest_bytes(report_raw),
            "size_bytes": len(report_raw),
        },
        "artifact_absolute_path": str(report_path),
        "binding": binding,
        "format": "ferric.m1-evidence-index.v1",
        "path_resolution": resolution,
        "requirements_sha256": plan["requirements"]["sha256"],
        "sources": plan["sources"],
        "subject": f"binding:{binding['id']}",
        "tcb": outer_tcb(plan_root),
    }
    result = subprocess.run(
        [sys.executable, "-I", str(validator), VALIDATOR_PROTOCOL],
        cwd=ferric,
        check=False,
        input=compact_bytes(context),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(
            f"trusted validator rejected ingested synthetic mechanics fixture:\n{result.stdout.decode()}"
        )


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_OBJECT_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    fe2o3_source = Path(sys.argv[2]).resolve(strict=True)
    producer_source = repo / "proofs/m1-qualification/produce-independent-validator.py"
    source_text = producer_source.read_text(encoding="ascii")
    forbidden = (
        "subprocess.run([checker",
        "subprocess.run(response",
        "import_module(response",
        "exec(response",
        'validate-independent-validator.py",',
    )
    if any(token in source_text for token in forbidden):
        fail("independent-review producer contains a forbidden checker execution path")

    with tempfile.TemporaryDirectory(
        prefix="ferric-m1-independent-review-policy-"
    ) as raw:
        root = Path(raw)
        ferric = root / "ferric"
        clone_at(repo, ferric)
        qualification = ferric / "proofs/m1-qualification"
        current_sources = (
            "planner.py",
            "produce-tcb-report.py",
            "produce-independent-validator.py",
        )
        for name in current_sources:
            shutil.copy2(repo / "proofs/m1-qualification" / name, qualification / name)
        destination = qualification / "produce-independent-validator.py"
        destination.chmod(0o755)
        commit_fixture(ferric)

        cargo = (ferric / "Cargo.toml").read_text(encoding="utf-8")
        marker = 'fe2o3-amdhsa-loader = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "'
        if cargo.count(marker) != 1:
            fail("cannot locate exact fe2o3 revision")
        revision = cargo.split(marker, 1)[1].split('"', 1)[0]
        fe2o3 = root / "fe2o3"
        clone_at(fe2o3_source, fe2o3, revision)

        planner = ferric / "proofs/m1-qualification/planner.py"
        tcb_producer = ferric / "proofs/m1-qualification/produce-tcb-report.py"
        producer = destination
        module = load_module(producer)
        validator = ferric / "proofs/m1/evidence/validate-independent-validator.py"
        baseline = root / "baseline"
        run_planner(planner, ferric, fe2o3, baseline)
        materialize_tcb(tcb_producer, ferric, fe2o3, baseline)
        plan = read_json(baseline / "plan.json")
        slots = [
            slot
            for slot in plan["binding_slots"]
            if slot["binding"]["evidence_kind"] == "independent-validator"
        ]
        if len(slots) != 44:
            fail("planner did not allocate exactly 44 independent-validator bindings")
        hostile_count = 0

        initialization_root = root / "validator-run-initialization"
        initialization_root.mkdir(mode=0o700)
        initialization_fd = os.open(
            initialization_root, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
        )
        original_open_directory_at = module.open_directory_at

        def fail_validator_run_open(parent_fd: int, name: str, description: str) -> Any:
            if description == "policy validator-run directory":
                module.fail("synthetic validator-run open failure")
            return original_open_directory_at(parent_fd, name, description)

        module.open_directory_at = fail_validator_run_open
        try:
            expect_module_failure(
                lambda: module.ensure_child_directory(
                    initialization_fd,
                    "validator-runs",
                    "policy validator-run directory",
                ),
                "validator-run open failure",
            )
        finally:
            module.open_directory_at = original_open_directory_at
        if (initialization_root / "validator-runs").exists():
            fail("validator-run open failure retained the exact created directory")
        hostile_count += 1

        original_os_stat = module.os.stat
        stat_failure_triggered = False

        def fail_validator_run_stat(path: Any, *args: Any, **kwargs: Any) -> Any:
            nonlocal stat_failure_triggered
            if (
                path == "validator-runs"
                and kwargs.get("dir_fd") == initialization_fd
                and kwargs.get("follow_symlinks") is False
            ):
                stat_failure_triggered = True
                raise OSError("synthetic validator-run identity failure")
            return original_os_stat(path, *args, **kwargs)

        module.os.stat = fail_validator_run_stat
        try:
            expect_module_failure(
                lambda: module.ensure_child_directory(
                    initialization_fd,
                    "validator-runs",
                    "policy validator-run directory",
                ),
                "cannot identify failed policy validator-run directory",
            )
        finally:
            module.os.stat = original_os_stat
            os.close(initialization_fd)
        if (
            not stat_failure_triggered
            or not (initialization_root / "validator-runs").is_dir()
        ):
            fail("unidentified validator-run directory was removed or not reported")
        (initialization_root / "validator-runs").rmdir()
        hostile_count += 1

        public_handoff_parent = root / "hostile-public-handoff-parent"
        public_handoff_parent.mkdir(mode=0o755)
        public_handoff_parent.chmod(0o755)
        public_handoff = public_handoff_parent / "handoff"
        result = invoke_export(producer, ferric, fe2o3, baseline, public_handoff)
        expected = "handoff parent must be an owner-private 0700 directory"
        if result.returncode == 0 or expected not in result.stdout:
            fail(
                "independent-review export accepted a public handoff parent; "
                f"expected {expected!r}:\n{result.stdout}"
            )
        if public_handoff.exists():
            fail("rejected public handoff parent retained an output root")
        hostile_count += 1

        handoff_a = root / "handoff-a"
        handoff_b = root / "handoff-b"
        for handoff in (handoff_a, handoff_b):
            result = invoke_export(producer, ferric, fe2o3, baseline, handoff)
            if result.returncode != 0:
                fail(f"independent-review export failed:\n{result.stdout}")
        if tree_identity(handoff_a) != tree_identity(handoff_b):
            fail("independent-review export is not byte deterministic")
        manifest = read_json(handoff_a / "handoff.json")
        if len(manifest["requests"]) != 44:
            fail("independent-review handoff request count drifted")
        inputs = list(handoff_a.glob("requests/*/cases/*.input.json"))
        if len(inputs) != 440:
            fail(f"independent-review case input count drifted: {len(inputs)}")
        if (baseline / "validator-runs").exists() or list(
            (baseline / "artifacts").glob("*.independent-validator.json")
        ):
            fail("request export published forbidden independent evidence")

        first = slots[0]
        binding_id = first["binding"]["id"]
        sentinel = root / "RETURNED-CHECKER-WAS-EXECUTED"
        response = root / "synthetic-test-only-response"
        for slot in slots:
            request_path = (
                handoff_a / "requests" / slot["binding"]["id"] / "request.json"
            )
            make_synthetic_test_response(response, request_path, sentinel)
        if len(list((response / "responses").glob("binding.*"))) != 44:
            fail("synthetic response fixture count drifted")
        public_response = copy_tree(response, root / "hostile-public-response")
        public_response.chmod(0o755)
        expect_intake_failure(
            producer,
            ferric,
            fe2o3,
            baseline,
            public_response,
            binding_id,
            "external independent-review response must be an owner-private 0700 directory",
        )
        hostile_count += 1
        canonical_plan = copy_tree(baseline, root / "canonical-plan")
        for ordinal, slot in enumerate(slots, 1):
            selected = slot["binding"]["id"]
            result = invoke_intake(
                producer, ferric, fe2o3, canonical_plan, response, selected
            )
            if result.returncode != 0:
                fail(
                    "independent-review intake rejected synthetic mechanics fixture "
                    f"{ordinal}/44 {selected}:\n{result.stdout}"
                )
            if sentinel.exists():
                fail(
                    "independent-review intake executed a returned checker while "
                    f"processing {selected}"
                )
        for slot in slots:
            validate_report(validator, ferric, canonical_plan, plan, slot)
        artifact_id = first["binding"]["artifact_id"]
        outputs = list(
            (canonical_plan / "validator-runs").glob("*.independent-validator.*.json")
        ) + list((canonical_plan / "artifacts").glob("*.independent-validator.json"))
        if len(outputs) != 132:
            fail(f"independent-review final output count drifted: {len(outputs)}")
        if not all(
            path.is_file() and (path.stat().st_mode & 0o777) == 0o600
            for path in outputs
        ):
            fail("independent-review final publication identity drifted")

        tcb_plan = copy_tree(baseline, root / "hostile-plan-tcb-semantics")
        tcb_path = tcb_plan / "artifacts" / "artifact.tcb.compiler.tcb-report.json"
        tcb_report = read_json(tcb_path)
        tcb_report["component_roster"][0]["authority"] = "synthetic-test-substitution"
        write_json(tcb_path, tcb_report)
        expect_intake_failure(
            producer,
            ferric,
            fe2o3,
            tcb_plan,
            response,
            binding_id,
            "exact authenticated projection",
        )
        hostile_count += 1

        replay_response = copy_tree(response, root / "hostile-binding-replay")
        replay_manifest_path = (
            replay_response / "responses" / binding_id / "response.json"
        )
        replay_manifest = read_json(replay_manifest_path)
        replay_manifest["binding_sha256"] = digest_bytes(b"substituted binding")
        write_json(replay_manifest_path, replay_manifest)
        expect_intake_failure(
            producer,
            ferric,
            fe2o3,
            copy_tree(baseline, root / "hostile-plan-replay"),
            replay_response,
            binding_id,
            "binding or attestation drifted",
        )
        hostile_count += 1

        output_response = copy_tree(response, root / "hostile-output")
        output_path = (
            output_response
            / "responses"
            / binding_id
            / "outputs"
            / "canonical-subject.output"
        )
        output_path.write_bytes(b"SYNTHETIC TEST ONLY: substituted output\n")
        output_path.chmod(0o600)
        expect_intake_failure(
            producer,
            ferric,
            fe2o3,
            copy_tree(baseline, root / "hostile-plan-output"),
            output_response,
            binding_id,
            "output identity drifted",
        )
        hostile_count += 1

        checker_response = copy_tree(response, root / "hostile-self-checker")
        checker_manifest_path = (
            checker_response / "responses" / binding_id / "response.json"
        )
        checker_manifest = read_json(checker_manifest_path)
        checker_manifest["checker"]["organization"] = "ferric"
        write_json(checker_manifest_path, checker_manifest)
        expect_intake_failure(
            producer,
            ferric,
            fe2o3,
            copy_tree(baseline, root / "hostile-plan-self-checker"),
            checker_response,
            binding_id,
            "self-validation",
        )
        hostile_count += 1

        missing_response = copy_tree(response, root / "hostile-missing-case")
        missing_manifest_path = (
            missing_response / "responses" / binding_id / "response.json"
        )
        missing_manifest = read_json(missing_manifest_path)
        missing_manifest["results"].pop()
        write_json(missing_manifest_path, missing_manifest)
        expect_intake_failure(
            producer,
            ferric,
            fe2o3,
            copy_tree(baseline, root / "hostile-plan-missing-case"),
            missing_response,
            binding_id,
            "result roster is incomplete",
        )
        hostile_count += 1

        hardlink_response = copy_tree(response, root / "hostile-hardlink")
        hardlink_outputs = hardlink_response / "responses" / binding_id / "outputs"
        hardlink_target = hardlink_outputs / "boundary-conforming-subject.output"
        hardlink_target.unlink()
        os.link(hardlink_outputs / "canonical-subject.output", hardlink_target)
        expect_intake_failure(
            producer,
            ferric,
            fe2o3,
            copy_tree(baseline, root / "hostile-plan-hardlink"),
            hardlink_response,
            binding_id,
            "stable owner-private file",
        )
        hostile_count += 1

        preexisting_plan = copy_tree(baseline, root / "hostile-plan-preexisting")
        preexisting = (
            preexisting_plan / "artifacts" / f"{artifact_id}.independent-validator.json"
        )
        preexisting.write_bytes(b"SYNTHETIC TEST ONLY: preexisting report\n")
        preexisting.chmod(0o600)
        expect_intake_failure(
            producer,
            ferric,
            fe2o3,
            preexisting_plan,
            response,
            binding_id,
            "without replacement",
        )
        rollback_companions = (
            preexisting_plan
            / "validator-runs"
            / f"{artifact_id}.independent-validator.roster.json",
            preexisting_plan
            / "validator-runs"
            / f"{artifact_id}.independent-validator.transcript.json",
        )
        if any(path.exists() for path in rollback_companions):
            fail("failed report publication did not roll back exact companions")
        if preexisting.read_bytes() != b"SYNTHETIC TEST ONLY: preexisting report\n":
            fail("failed publication replaced the preexisting report")
        hostile_count += 1

        module = load_module(producer)
        original_create = module.create_new_at

        absolute_root = root / "absolute-custody-test"
        private_directory(absolute_root)
        authenticated_parent = absolute_root / "authenticated-parent"
        private_directory(authenticated_parent)
        authenticated_leaf = authenticated_parent / "leaf"
        private_directory(authenticated_leaf)
        absolute_custody = module.open_absolute_directory(
            authenticated_leaf, "synthetic absolute-custody leaf", private=True
        )
        authenticated_parent.rename(absolute_root / "authenticated-parent-held")
        private_directory(authenticated_parent)
        private_directory(authenticated_parent / "leaf")
        expect_module_failure(
            lambda: module.revalidate_absolute_directory(absolute_custody),
            "directory was rebound",
        )
        module.close_absolute_directory(absolute_custody)
        hostile_count += 1

        partial_handoff = root / "hostile-partial-handoff"
        partial_counter = 0

        def fail_partial_handoff(
            directory_fd: int, name: str, raw_bytes: bytes, description: str
        ) -> Any:
            nonlocal partial_counter
            if description.startswith("request case"):
                partial_counter += 1
                if partial_counter == 4:
                    module.fail("synthetic mid-handoff publication failure")
            return original_create(directory_fd, name, raw_bytes, description)

        module.create_new_at = fail_partial_handoff
        expect_module_failure(
            lambda: module.export_all(
                str(ferric), str(fe2o3), str(baseline), str(partial_handoff)
            ),
            "mid-handoff publication failure",
        )
        module.create_new_at = original_create
        if partial_handoff.exists():
            fail("failed handoff transaction left a partial output root")
        module.export_all(str(ferric), str(fe2o3), str(baseline), str(partial_handoff))
        if not (partial_handoff / "handoff.json").is_file():
            fail("clean retry after handoff rollback did not publish its manifest")
        hostile_count += 1

        post_mkdir_handoff = root / "hostile-post-mkdir-handoff"
        original_open_directory_at = module.open_directory_at
        post_mkdir_triggered = False

        def fail_after_handoff_mkdir(
            parent_fd: int, name: str, description: str
        ) -> Any:
            nonlocal post_mkdir_triggered
            if description == "independent-review handoff root":
                post_mkdir_triggered = True
                module.fail("synthetic post-mkdir open failure")
            return original_open_directory_at(parent_fd, name, description)

        module.open_directory_at = fail_after_handoff_mkdir
        try:
            expect_module_failure(
                lambda: module.export_all(
                    str(ferric),
                    str(fe2o3),
                    str(baseline),
                    str(post_mkdir_handoff),
                ),
                "post-mkdir open failure",
            )
        finally:
            module.open_directory_at = original_open_directory_at
        if not post_mkdir_triggered or post_mkdir_handoff.exists():
            fail("post-mkdir failure left an untracked handoff root")
        hostile_count += 1

        stat_failure_handoff = root / "hostile-post-mkdir-stat-handoff"
        original_os_stat = module.os.stat
        stat_failure_triggered = False

        def fail_initial_handoff_stat(path: Any, *args: Any, **kwargs: Any) -> Any:
            nonlocal stat_failure_triggered
            if (
                path == stat_failure_handoff.name
                and kwargs.get("dir_fd") is not None
                and kwargs.get("follow_symlinks") is False
            ):
                stat_failure_triggered = True
                raise OSError("synthetic initial directory identity failure")
            return original_os_stat(path, *args, **kwargs)

        module.os.stat = fail_initial_handoff_stat
        try:
            expect_module_failure(
                lambda: module.export_all(
                    str(ferric),
                    str(fe2o3),
                    str(baseline),
                    str(stat_failure_handoff),
                ),
                "cannot identify failed independent-review handoff root",
            )
        finally:
            module.os.stat = original_os_stat
        if not stat_failure_triggered or not stat_failure_handoff.is_dir():
            fail("unidentified post-mkdir path was removed or not reported")
        stat_failure_handoff.rmdir()
        hostile_count += 1

        short_write_handoff = root / "hostile-short-write-handoff"
        short_write_triggered = False

        def fail_handoff_write(
            directory_fd: int, name: str, raw_bytes: bytes, description: str
        ) -> Any:
            nonlocal short_write_triggered
            original_os_write = module.os.write

            def short_write(descriptor: int, value: Any) -> int:
                nonlocal short_write_triggered
                short_write_triggered = True
                return 0

            module.os.write = short_write
            try:
                return original_create(directory_fd, name, raw_bytes, description)
            finally:
                module.os.write = original_os_write

        module.create_new_at = fail_handoff_write
        try:
            expect_module_failure(
                lambda: module.export_all(
                    str(ferric),
                    str(fe2o3),
                    str(baseline),
                    str(short_write_handoff),
                ),
                "cannot completely write",
            )
        finally:
            module.create_new_at = original_create
        if not short_write_triggered or short_write_handoff.exists():
            fail("short-write failure left an untracked handoff file or directory")
        hostile_count += 1

        closure_plan = copy_tree(baseline, root / "hostile-plan-closure-race")
        closure_response = copy_tree(response, root / "hostile-response-closure-race")
        closure_triggered = False

        def publish_closure_output(
            directory_fd: int, name: str, raw_bytes: bytes, description: str
        ) -> Any:
            nonlocal closure_triggered
            published = original_create(directory_fd, name, raw_bytes, description)
            if description == "validator roster" and not closure_triggered:
                receipt = closure_plan / "receipt.json"
                receipt.write_bytes(b"SYNTHETIC TEST ONLY: raced closure output\n")
                receipt.chmod(0o600)
                closure_triggered = True
            return published

        module.create_new_at = publish_closure_output
        expect_module_failure(
            lambda: module.intake(
                str(ferric),
                str(fe2o3),
                str(closure_plan),
                str(closure_response),
                binding_id,
            ),
            "plan containing a closure output",
        )
        module.create_new_at = original_create
        if not closure_triggered or any(
            (closure_plan / "validator-runs").glob("*.independent-validator.*.json")
        ):
            fail("closure-output race did not reject and roll back publication")
        hostile_count += 1

        rebind_plan = copy_tree(baseline, root / "hostile-plan-response-rebind")
        rebind_response = copy_tree(response, root / "hostile-response-parent-rebind")
        rebind_triggered = False

        def rebind_response_parent(
            directory_fd: int, name: str, raw_bytes: bytes, description: str
        ) -> Any:
            nonlocal rebind_triggered
            published = original_create(directory_fd, name, raw_bytes, description)
            if description == "validator roster" and not rebind_triggered:
                live = rebind_response / "responses"
                live.rename(rebind_response / "responses-authenticated")
                private_directory(live)
                rebind_triggered = True
            return published

        module.create_new_at = rebind_response_parent
        expect_module_failure(
            lambda: module.intake(
                str(ferric),
                str(fe2o3),
                str(rebind_plan),
                str(rebind_response),
                binding_id,
            ),
            "directory was rebound",
        )
        module.create_new_at = original_create
        if not rebind_triggered or any(
            (rebind_plan / "validator-runs").glob("*.independent-validator.*.json")
        ):
            fail("response-parent rebinding did not reject and roll back publication")
        hostile_count += 1

        toctou_plan = copy_tree(baseline, root / "hostile-plan-file-toctou")
        toctou_response = copy_tree(response, root / "hostile-response-file-toctou")
        toctou_triggered = False

        def replace_plan_file(
            directory_fd: int, name: str, raw_bytes: bytes, description: str
        ) -> Any:
            nonlocal toctou_triggered
            published = original_create(directory_fd, name, raw_bytes, description)
            if description == "validator roster" and not toctou_triggered:
                live = toctou_plan / "plan.json"
                original = toctou_plan / "plan.authenticated.json"
                live.rename(original)
                live.write_bytes(original.read_bytes())
                live.chmod(0o600)
                toctou_triggered = True
            return published

        module.create_new_at = replace_plan_file
        expect_module_failure(
            lambda: module.intake(
                str(ferric),
                str(fe2o3),
                str(toctou_plan),
                str(toctou_response),
                binding_id,
            ),
            "changed after authentication",
        )
        module.create_new_at = original_create
        if not toctou_triggered or any(
            (toctou_plan / "validator-runs").glob("*.independent-validator.*.json")
        ):
            fail("plan-file TOCTOU did not reject and roll back publication")
        hostile_count += 1

        tcb_toctou_plan = copy_tree(baseline, root / "hostile-plan-tcb-toctou")
        tcb_toctou_response = copy_tree(response, root / "hostile-response-tcb-toctou")
        tcb_toctou_triggered = False

        def replace_tcb_file(
            directory_fd: int, name: str, raw_bytes: bytes, description: str
        ) -> Any:
            nonlocal tcb_toctou_triggered
            published = original_create(directory_fd, name, raw_bytes, description)
            if description == "validator roster" and not tcb_toctou_triggered:
                live = (
                    tcb_toctou_plan
                    / "artifacts"
                    / "artifact.tcb.runtime.tcb-report.json"
                )
                original = (
                    tcb_toctou_plan
                    / "artifacts"
                    / "artifact.tcb.runtime.authenticated.json"
                )
                live.rename(original)
                live.write_bytes(original.read_bytes())
                live.chmod(0o600)
                tcb_toctou_triggered = True
            return published

        module.create_new_at = replace_tcb_file
        expect_module_failure(
            lambda: module.intake(
                str(ferric),
                str(fe2o3),
                str(tcb_toctou_plan),
                str(tcb_toctou_response),
                binding_id,
            ),
            "changed after authentication",
        )
        module.create_new_at = original_create
        if not tcb_toctou_triggered or any(
            (tcb_toctou_plan / "validator-runs").glob("*.independent-validator.*.json")
        ):
            fail("TCB-file TOCTOU did not reject and roll back publication")
        hostile_count += 1

        rebound_plan = copy_tree(baseline, root / "hostile-plan-rollback-rebound")
        rebound_response = copy_tree(
            response, root / "hostile-response-rollback-rebound"
        )
        rebound_roster = (
            rebound_plan
            / "validator-runs"
            / f"{artifact_id}.independent-validator.roster.json"
        )
        rebound_bytes = b"SYNTHETIC TEST ONLY: rebound roster inode\n"
        rebound_triggered = False

        def rebind_rollback_file(
            directory_fd: int, name: str, raw_bytes: bytes, description: str
        ) -> Any:
            nonlocal rebound_triggered
            if description == "validator transcript" and not rebound_triggered:
                rebound_roster.unlink()
                rebound_roster.write_bytes(rebound_bytes)
                rebound_roster.chmod(0o600)
                rebound_triggered = True
                module.fail("synthetic transcript publication failure")
            return original_create(directory_fd, name, raw_bytes, description)

        module.create_new_at = rebind_rollback_file
        expect_module_failure(
            lambda: module.intake(
                str(ferric),
                str(fe2o3),
                str(rebound_plan),
                str(rebound_response),
                binding_id,
            ),
            "rollback failures",
        )
        module.create_new_at = original_create
        if (
            not rebound_triggered
            or rebound_roster.read_bytes() != rebound_bytes
            or (
                rebound_plan / "artifacts" / f"{artifact_id}.independent-validator.json"
            ).exists()
        ):
            fail("rollback removed a rebound inode or left a false completion marker")
        hostile_count += 1

        if sentinel.exists():
            fail("a hostile intake path executed the returned checker")
        if command(
            [
                "git",
                "-C",
                str(ferric),
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
            ],
            "recheck Ferric fixture",
        ) or command(
            [
                "git",
                "-C",
                str(fe2o3),
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
            ],
            "recheck fe2o3 fixture",
        ):
            fail(
                "independent-review tooling dirtied an authenticated source repository"
            )
    print(
        "PASS: independent-review producer exported 44 synthetic request fixtures, "
        "ingested and trusted-validated all 44 temp-only mechanics responses without "
        "executing their checker, "
        f"and rejected {hostile_count} hostile responses"
    )


if __name__ == "__main__":
    main()
