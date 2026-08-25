#!/usr/bin/env python3
"""Exercise the Ferric M1 performance intake producer without claiming evidence."""

from __future__ import annotations

import contextlib
from concurrent.futures import ThreadPoolExecutor, as_completed
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.performance-report.v1"


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        fail(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def compact_bytes(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def command(arguments: list[str], description: str, *, cwd: Path | None = None) -> str:
    result = subprocess.run(
        arguments,
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=300,
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
    command(["git", "-C", str(repository), "add", "-A"], "stage performance fixture")
    command(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=M1 Performance Producer Policy",
            "-c",
            "user.email=m1-performance-producer@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "add M1 performance intake producer",
        ],
        "commit performance fixture",
    )


def expect_rejected(operation: Callable[[], None], description: str) -> None:
    output = io.StringIO()
    try:
        with contextlib.redirect_stderr(output):
            operation()
    except SystemExit as error:
        if error.code in (None, 0):
            fail(f"hostile case exited successfully: {description}")
        return
    fail(f"hostile performance intake was accepted: {description}")


def make_intake(
    repo: Path, producer: Any, validator_policy: Any, validator: Any, root: Path
) -> tuple[
    dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any], bytes, bytes
]:
    report_path, measurement_path, context, report, measurements = (
        validator_policy.make_fixture(repo, validator, root)
    )
    intake = {
        "environment": copy.deepcopy(report["environment"]),
        "format": producer.INTAKE_FORMAT,
        "measurements": copy.deepcopy(measurements),
    }
    slot = {"binding": copy.deepcopy(context["binding"])}
    requirements = {"sha256": context["requirements_sha256"]}
    generated_measurements, generated_report = producer.performance_documents(
        requirements,
        copy.deepcopy(context["sources"]),
        copy.deepcopy(context["tcb"]),
        slot,
        copy.deepcopy(context["path_resolution"]),
        intake,
        (repo / producer.PERFORMANCE_POLICY_PATH).read_bytes(),
    )
    if generated_measurements != measurement_path.read_bytes():
        fail("producer changed the external measurement roster")
    if generated_report != report_path.read_bytes():
        fail("producer report differs from the unchanged trusted-validator fixture")
    return (
        intake,
        context,
        slot,
        context["path_resolution"],
        generated_measurements,
        generated_report,
    )


def validate_intake_hostiles(producer: Any, canonical: dict[str, Any]) -> int:
    mutations: list[tuple[str, Callable[[dict[str, Any]], None]]] = [
        ("wrong-format", lambda value: value.__setitem__("format", "wrong")),
        ("extra-field", lambda value: value.__setitem__("extra", True)),
        ("missing-environment", lambda value: value.pop("environment")),
        (
            "environment-substitution",
            lambda value: value["environment"].__setitem__(
                "environment_sha256", digest_bytes(b"substitute")
            ),
        ),
        (
            "measurement-authority",
            lambda value: value["measurements"].__setitem__("authority", "unchecked"),
        ),
        (
            "measurement-target",
            lambda value: value["measurements"].__setitem__("target", "gfx942:xnack+"),
        ),
        (
            "missing-gate-class",
            lambda value: value["measurements"]["cells"].pop(),
        ),
        (
            "faulted-sample",
            lambda value: value["measurements"]["cells"][0]["rows"][10].__setitem__(
                "faults", ["synthetic-fault"]
            ),
        ),
        (
            "reordered-sample",
            lambda value: value["measurements"]["cells"][0]["rows"].reverse(),
        ),
        (
            "invented-zero",
            lambda value: value["measurements"]["cells"][0]["rows"][10]["values"][
                "ferric"
            ].__setitem__("primary", 0),
        ),
        (
            "workload-roster-substitution",
            lambda value: value["measurements"]["qualification_identities"].__setitem__(
                "workload_roster_sha256", digest_bytes(b"different roster")
            ),
        ),
    ]
    count = 0
    for name, mutate in mutations:
        value = copy.deepcopy(canonical)
        mutate(value)
        expect_rejected(lambda value=value: producer.validate_intake(value), name)
        count += 1
    return count


def custody_policy(producer: Any, intake: dict[str, Any], root: Path) -> int:
    custody_root = root / "custody"
    custody_root.mkdir(mode=0o700)
    intake_path = custody_root / "intake.json"
    intake_path.write_bytes(canonical_bytes(intake))
    intake_path.chmod(0o600)
    custody = producer.authenticate_absolute_directory(
        custody_root, "test intake directory", private=True
    )
    directory_fd = producer.directory_custody_fd(custody)
    held = None
    try:
        value, held = producer.read_held_file_at(
            directory_fd,
            intake_path.name,
            producer.MAX_INTAKE_BYTES,
            "test performance intake",
            single_link=True,
        )
        producer.validate_intake(value)
        producer.revalidate_held_file(directory_fd, held)
    finally:
        if held is not None:
            held[1].close()
        producer.close_absolute_directory(custody)

    symlink = custody_root / "symlink.json"
    symlink.symlink_to(intake_path.name)
    custody = producer.authenticate_absolute_directory(
        custody_root, "test intake directory", private=True
    )
    try:
        expect_rejected(
            lambda: producer.read_held_file_at(
                producer.directory_custody_fd(custody),
                symlink.name,
                producer.MAX_INTAKE_BYTES,
                "symlink intake",
                single_link=True,
            ),
            "symlink-intake",
        )
    finally:
        producer.close_absolute_directory(custody)

    hardlink = custody_root / "hardlink.json"
    os.link(intake_path, hardlink)
    custody = producer.authenticate_absolute_directory(
        custody_root, "test intake directory", private=True
    )
    try:
        expect_rejected(
            lambda: producer.read_held_file_at(
                producer.directory_custody_fd(custody),
                hardlink.name,
                producer.MAX_INTAKE_BYTES,
                "hardlink intake",
                single_link=True,
            ),
            "hardlink-intake",
        )
    finally:
        producer.close_absolute_directory(custody)
    hardlink.unlink()

    intake_path.chmod(0o644)
    custody = producer.authenticate_absolute_directory(
        custody_root, "test intake directory", private=True
    )
    try:
        expect_rejected(
            lambda: producer.read_held_file_at(
                producer.directory_custody_fd(custody),
                intake_path.name,
                producer.MAX_INTAKE_BYTES,
                "public intake",
                single_link=True,
            ),
            "public-intake",
        )
    finally:
        producer.close_absolute_directory(custody)
    return 3


def policy_source_custody_policy(producer: Any, root: Path) -> int:
    policy_root = root / "policy-source-custody"
    policy_root.mkdir(mode=0o700)
    policy_path = policy_root / "PERFORMANCE.md"
    policy_path.write_bytes(b"held performance policy\n")
    policy_path.chmod(0o644)
    custody = producer.authenticate_absolute_directory(
        policy_root, "policy source directory", private=True
    )
    directory_fd = producer.directory_custody_fd(custody)
    held = producer.read_held_bytes_at(
        directory_fd,
        policy_path.name,
        producer.MAX_REPORT_BYTES,
        "policy source file",
        owner_private=False,
    )
    displaced = policy_root / "PERFORMANCE.held"
    try:
        policy_path.rename(displaced)
        policy_path.write_bytes(b"substituted performance policy\n")
        policy_path.chmod(0o644)
        expect_rejected(
            lambda: producer.revalidate_held_file(directory_fd, held),
            "performance-policy-source-replacement",
        )
        if not policy_path.exists() or not displaced.exists():
            fail("policy source custody test did not preserve both file identities")
    finally:
        held[1].close()
        producer.close_absolute_directory(custody)
    return 1


def rebound_cleanup_policy(producer: Any, root: Path) -> int:
    cleanup_root = root / "rebound-cleanup"
    cleanup_root.mkdir(mode=0o700)
    custody = producer.authenticate_absolute_directory(
        cleanup_root, "rebound cleanup directory", private=True
    )
    directory_fd = producer.directory_custody_fd(custody)
    measurement_fd, created = producer.ensure_private_child_directory(
        directory_fd, "measurements", "rebound measurement directory"
    )
    if not created:
        fail("rebound cleanup policy did not create its measurement directory")
    displaced = cleanup_root / "measurements.held"
    try:
        (cleanup_root / "measurements").rename(displaced)
        (cleanup_root / "measurements").mkdir(mode=0o700)
        failure = producer.rollback_exact_directory(
            directory_fd,
            "measurements",
            measurement_fd,
            "M1 measurement directory",
        )
        if failure != "cannot remove replaced failed M1 measurement directory":
            fail(f"rebound cleanup returned an unexpected result: {failure!r}")
        if not (cleanup_root / "measurements").is_dir() or not displaced.is_dir():
            fail("rebound cleanup removed a directory identity it did not create")
    finally:
        os.close(measurement_fd)
        producer.close_absolute_directory(custody)

    sync_root = root / "post-create-sync-failure"
    sync_root.mkdir(mode=0o700)
    custody = producer.authenticate_absolute_directory(
        sync_root, "post-create sync directory", private=True
    )
    directory_fd = producer.directory_custody_fd(custody)
    original_fsync = producer.os.fsync
    sync_failed = False

    def fail_first_parent_sync(descriptor: int) -> None:
        nonlocal sync_failed
        if descriptor == directory_fd and not sync_failed:
            sync_failed = True
            raise OSError("injected parent-directory sync failure")
        original_fsync(descriptor)

    producer.os.fsync = fail_first_parent_sync
    try:
        expect_rejected(
            lambda: producer.ensure_private_child_directory(
                directory_fd,
                "measurements",
                "post-create sync measurement directory",
            ),
            "post-create-directory-sync-failure",
        )
    finally:
        producer.os.fsync = original_fsync
        producer.close_absolute_directory(custody)
    if (sync_root / "measurements").exists():
        fail("post-create sync failure retained the exact created directory")

    rebound_root = root / "post-create-rebound"
    rebound_root.mkdir(mode=0o700)
    custody = producer.authenticate_absolute_directory(
        rebound_root, "post-create rebound root", private=True
    )
    directory_fd = producer.directory_custody_fd(custody)
    original_verify = producer.verify_private_directory
    rebound_injected = False

    def inject_rebound(metadata: os.stat_result, description: str) -> None:
        nonlocal rebound_injected
        if description == "post-create rebound measurement directory":
            rebound_injected = True
            (rebound_root / "measurements").rename(rebound_root / "measurements.held")
            (rebound_root / "measurements").mkdir(mode=0o700)
            producer.fail("injected post-create directory rebound")
        original_verify(metadata, description)

    producer.verify_private_directory = inject_rebound
    try:
        expect_rejected(
            lambda: producer.ensure_private_child_directory(
                directory_fd,
                "measurements",
                "post-create rebound measurement directory",
            ),
            "post-create-directory-rebound",
        )
    finally:
        producer.verify_private_directory = original_verify
        producer.close_absolute_directory(custody)
    if (
        not rebound_injected
        or not (rebound_root / "measurements").is_dir()
        or not (rebound_root / "measurements.held").is_dir()
    ):
        fail("post-create rebound cleanup removed an unauthenticated directory")

    open_root = root / "post-create-open-failure"
    open_root.mkdir(mode=0o700)
    custody = producer.authenticate_absolute_directory(
        open_root, "post-create open-failure root", private=True
    )
    directory_fd = producer.directory_custody_fd(custody)
    original_open = producer.os.open

    def fail_created_directory_open(
        path: Any, flags: int, *args: Any, **kwargs: Any
    ) -> int:
        if path == "measurements" and kwargs.get("dir_fd") == directory_fd:
            raise OSError("injected created-directory open failure")
        return original_open(path, flags, *args, **kwargs)

    producer.os.open = fail_created_directory_open
    try:
        expect_rejected(
            lambda: producer.ensure_private_child_directory(
                directory_fd,
                "measurements",
                "post-create open-failure measurement directory",
            ),
            "post-create-directory-open-failure",
        )
    finally:
        producer.os.open = original_open
        producer.close_absolute_directory(custody)
    if (open_root / "measurements").exists():
        fail("post-create open failure retained the exact created directory")

    stat_root = root / "post-create-stat-failure"
    stat_root.mkdir(mode=0o700)
    custody = producer.authenticate_absolute_directory(
        stat_root, "post-create stat-failure root", private=True
    )
    directory_fd = producer.directory_custody_fd(custody)
    original_stat = producer.os.stat

    def fail_created_directory_stat(path: Any, *args: Any, **kwargs: Any) -> Any:
        if path == "measurements" and kwargs.get("dir_fd") == directory_fd:
            raise OSError("injected created-directory identity failure")
        return original_stat(path, *args, **kwargs)

    producer.os.stat = fail_created_directory_stat
    try:
        expect_rejected(
            lambda: producer.ensure_private_child_directory(
                directory_fd,
                "measurements",
                "post-create stat-failure measurement directory",
            ),
            "cannot identify failed post-create stat-failure measurement directory",
        )
    finally:
        producer.os.stat = original_stat
        producer.close_absolute_directory(custody)
    if not (stat_root / "measurements").is_dir():
        fail("unidentified post-create directory was removed or not reported")
    (stat_root / "measurements").rmdir()
    return 5


def tcb_authentication_policy(
    repo: Path,
    producer: Any,
    tcb_producer: Any,
    sources: list[dict[str, Any]],
    root: Path,
) -> int:
    requirements = json.loads(
        (repo / "proofs/M1_REQUIREMENTS.json").read_text(encoding="ascii")
    )
    validators = tcb_producer.trusted_validators(repo)[1]
    artifacts = root / "tcb-artifacts"
    artifacts.mkdir(mode=0o700)
    for identifier in producer.TCB_IDS:
        local = producer.tcb_report_for(
            repo, requirements, sources, validators, identifier
        )
        reviewed = tcb_producer.report_for(
            repo, requirements, sources, validators, identifier
        )
        if producer.canonical_bytes(local) != tcb_producer.canonical_bytes(reviewed):
            fail(f"performance producer TCB projection drifted: {identifier}")
        path = artifacts / f"artifact.{identifier}.tcb-report.json"
        path.write_bytes(canonical_bytes(local))
        path.chmod(0o600)
    custody = producer.authenticate_absolute_directory(
        artifacts, "policy TCB directory", private=True
    )
    artifact_fd = producer.directory_custody_fd(custody)
    held: list[Any] = []
    try:
        roster, held = producer.authenticate_tcb_reports(
            artifact_fd, repo, requirements, sources, validators
        )
        if [row["id"] for row in roster] != list(producer.TCB_IDS):
            fail("performance producer TCB roster order drifted")
    finally:
        for item in held:
            item[1].close()
        producer.close_absolute_directory(custody)

    subject = producer.TCB_IDS[0]
    path = artifacts / f"artifact.{subject}.tcb-report.json"
    tampered = json.loads(path.read_text(encoding="ascii"))
    tampered["component_roster"][0]["authority"] = "attacker-authored"
    path.write_bytes(canonical_bytes(tampered))
    path.chmod(0o600)
    custody = producer.authenticate_absolute_directory(
        artifacts, "policy TCB directory", private=True
    )
    try:
        expect_rejected(
            lambda: producer.authenticate_tcb_reports(
                producer.directory_custody_fd(custody),
                repo,
                requirements,
                sources,
                validators,
            ),
            "semantic-tcb-tampering",
        )
    finally:
        producer.close_absolute_directory(custody)
    return 1


def disjointness_policy(producer: Any, root: Path) -> int:
    repository = root / "repository"
    nested = repository / "nested"
    nested.mkdir(parents=True, mode=0o700)
    repository.chmod(0o700)
    plan = root / "plan"
    plan_nested = plan / "inputs"
    plan_nested.mkdir(parents=True, mode=0o700)
    plan.chmod(0o700)
    lexical = root / "outside" / ".." / "repository" / "nested" / "intake.json"
    if not producer.within(
        producer.lexical_absolute(lexical), producer.lexical_absolute(repository)
    ):
        fail("lexical '..' bypassed normalized performance-intake disjointness")

    repository_custody = producer.authenticate_absolute_directory(
        repository, "policy repository", private=True
    )
    nested_custody = producer.authenticate_absolute_directory(
        nested, "policy nested directory", private=True
    )
    try:
        if not producer.directory_descends_from(
            producer.directory_custody_fd(nested_custody),
            producer.directory_custody_fd(repository_custody),
        ):
            fail("directory identity alias bypassed performance-intake disjointness")
    finally:
        producer.close_absolute_directory(nested_custody)
        producer.close_absolute_directory(repository_custody)

    plan_custody = producer.authenticate_absolute_directory(
        plan, "policy plan", private=True
    )
    plan_nested_custody = producer.authenticate_absolute_directory(
        plan_nested, "policy nested plan directory", private=True
    )
    try:
        if not producer.directory_descends_from(
            producer.directory_custody_fd(plan_nested_custody),
            producer.directory_custody_fd(plan_custody),
        ):
            fail("directory identity alias bypassed performance plan disjointness")
    finally:
        producer.close_absolute_directory(plan_nested_custody)
        producer.close_absolute_directory(plan_custody)

    alias = root / "repository-alias"
    alias.symlink_to(repository, target_is_directory=True)
    expect_rejected(
        lambda: producer.authenticate_absolute_directory(
            alias, "symlink-component repository alias", private=True
        ),
        "symlink-component-repository-alias",
    )
    return 4


def invoke_validator(
    validator_path: Path, context: dict[str, Any]
) -> subprocess.CompletedProcess[bytes]:
    payload = (
        json.dumps(context, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    ).encode("ascii")
    return subprocess.run(
        [sys.executable, "-I", str(validator_path), PROTOCOL],
        input=payload,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
        timeout=30,
    )


def publication_policy(
    producer: Any,
    validator_path: Path,
    context: dict[str, Any],
    slot: dict[str, Any],
    measurement_bytes: bytes,
    report_bytes: bytes,
    root: Path,
) -> None:
    evidence = root / "evidence"
    evidence.mkdir(mode=0o700)
    (evidence / "artifacts").mkdir(mode=0o700)
    custody = producer.authenticate_absolute_directory(
        evidence, "policy evidence root", private=True
    )
    root_fd = producer.directory_custody_fd(custody)
    artifact_fd, _ = producer.ensure_private_child_directory(
        root_fd, "artifacts", "policy artifact directory"
    )
    measurement_fd, created = producer.ensure_private_child_directory(
        root_fd, "measurements", "policy measurement directory"
    )
    if not created:
        fail("policy did not create its measurement directory")
    artifact_id = slot["binding"]["artifact_id"]
    measurement_name = f"{artifact_id}.measurements.json"
    report_name = f"{artifact_id}.performance-report.json"
    states: list[tuple[bool, bool]] = []

    def custody_check() -> None:
        states.append(
            (
                (evidence / "measurements" / measurement_name).exists(),
                (evidence / "artifacts" / report_name).exists(),
            )
        )

    try:
        producer.publish_performance(
            custody,
            root_fd,
            artifact_fd,
            measurement_fd,
            artifact_id,
            measurement_bytes,
            report_bytes,
            custody_check,
        )
    finally:
        os.close(measurement_fd)
        os.close(artifact_fd)
        producer.close_absolute_directory(custody)
    if (
        not states
        or states[0] != (False, False)
        or (True, False) not in states
        or states[-1] != (True, True)
    ):
        fail(f"performance report was not published after measurements: {states}")
    for path in (
        evidence / "measurements" / measurement_name,
        evidence / "artifacts" / report_name,
    ):
        if stat.S_IMODE(path.stat().st_mode) != 0o600:
            fail(f"published performance file is not owner-private: {path}")
    report_path = evidence / "artifacts" / report_name
    context = copy.deepcopy(context)
    context["artifact_absolute_path"] = str(report_path)
    context["artifact"]["path"] = f"artifacts/{report_name}"
    context["artifact"]["sha256"] = digest_bytes(report_bytes)
    context["artifact"]["size_bytes"] = len(report_bytes)
    result = invoke_validator(validator_path, context)
    if result.returncode != 0 or not result.stdout.startswith(
        f"PASS: {PROTOCOL} ".encode("ascii")
    ):
        fail(f"unchanged trusted validator rejected producer output: {result.stdout!r}")


def rollback_policy(
    producer: Any,
    artifact_id: str,
    measurement_bytes: bytes,
    report_bytes: bytes,
    root: Path,
) -> None:
    evidence = root / "rollback"
    evidence.mkdir(mode=0o700)
    (evidence / "artifacts").mkdir(mode=0o700)
    custody = producer.authenticate_absolute_directory(
        evidence, "rollback evidence root", private=True
    )
    root_fd = producer.directory_custody_fd(custody)
    artifact_fd, _ = producer.ensure_private_child_directory(
        root_fd, "artifacts", "rollback artifact directory"
    )
    measurement_fd, _ = producer.ensure_private_child_directory(
        root_fd, "measurements", "rollback measurement directory"
    )
    original = producer.create_new_file_at
    calls = 0

    def fail_report(
        directory_fd: int, name: str, value: bytes, description: str
    ) -> int:
        nonlocal calls
        calls += 1
        if calls == 2:
            producer.fail("injected report publication failure")
        return original(directory_fd, name, value, description)

    producer.create_new_file_at = fail_report
    try:
        expect_rejected(
            lambda: producer.publish_performance(
                custody,
                root_fd,
                artifact_fd,
                measurement_fd,
                artifact_id,
                measurement_bytes,
                report_bytes,
                lambda: None,
            ),
            "partial-publication-rollback",
        )
    finally:
        producer.create_new_file_at = original
        os.close(measurement_fd)
        os.close(artifact_fd)
        producer.close_absolute_directory(custody)
    if list((evidence / "measurements").iterdir()) or list(
        (evidence / "artifacts").iterdir()
    ):
        fail("failed performance publication retained a partial output")


def read_json(path: Path) -> dict[str, Any]:
    raw = path.read_bytes()
    value = json.loads(raw)
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"fixture is not canonical JSON: {path}")
    return value


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_bytes(canonical_bytes(value))


def run_planner(planner: Path, ferric: Path, fe2o3: Path, output: Path) -> None:
    result = subprocess.run(
        [sys.executable, "-I", str(planner), str(ferric), str(fe2o3), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=300,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if (
        result.returncode != 0
        or "PASS: prepared external M1 evidence plan" not in result.stdout
    ):
        fail(f"planner rejected performance producer fixture:\n{result.stdout}")


def materialize_tcb(producer: Path, ferric: Path, fe2o3: Path, plan: Path) -> None:
    for subject in ("tcb.compiler", "tcb.hardware", "tcb.runtime"):
        result = subprocess.run(
            [
                sys.executable,
                "-I",
                str(producer),
                str(ferric),
                str(fe2o3),
                str(plan),
                subject,
            ],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=300,
            env={"PATH": os.environ.get("PATH", "")},
        )
        if result.returncode != 0:
            fail(f"TCB prerequisite failed for {subject}:\n{result.stdout}")


def performance_slots(plan: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        slot
        for slot in plan["binding_slots"]
        if slot["binding"]["evidence_kind"] == "performance-gate"
    ]


def invoke_cli(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    intake: Path,
    binding_id: str,
    inherited_environment: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    environment = {"PATH": os.environ.get("PATH", "")}
    if inherited_environment is not None:
        environment.update(inherited_environment)
    return subprocess.run(
        [
            sys.executable,
            "-I",
            str(producer),
            str(ferric),
            str(fe2o3),
            str(plan),
            str(intake),
            binding_id,
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=600,
        env=environment,
    )


def outer_tcb(plan_root: Path) -> list[dict[str, str]]:
    kinds = {
        "tcb.compiler": "Compiler",
        "tcb.hardware": "Hardware",
        "tcb.runtime": "Runtime",
    }
    rows = []
    for subject, kind in kinds.items():
        artifact_id = f"artifact.{subject}"
        raw = (plan_root / "artifacts" / f"{artifact_id}.tcb-report.json").read_bytes()
        rows.append(
            {
                "artifact_id": artifact_id,
                "id": subject,
                "identity_sha256": digest_bytes(raw),
                "kind": kind,
            }
        )
    return rows


def validate_all_cli_reports(
    validator: Path,
    plan_root: Path,
    plan: dict[str, Any],
    ferric: Path,
) -> None:
    resolutions = {row["id"]: row for row in plan["path_resolutions"]}
    tcb = outer_tcb(plan_root)
    observed: list[str] = []
    for slot in performance_slots(plan):
        binding = slot["binding"]
        artifact = slot["expected_artifact"]
        report_path = plan_root / artifact["path"]
        raw = report_path.read_bytes()
        context = {
            "artifact": {
                **artifact,
                "sha256": digest_bytes(raw),
                "size_bytes": len(raw),
            },
            "artifact_absolute_path": str(report_path),
            "binding": binding,
            "format": "ferric.m1-evidence-index.v1",
            "path_resolution": resolutions[binding["path_id"]],
            "requirements_sha256": plan["requirements"]["sha256"],
            "sources": plan["sources"],
            "subject": f"binding:{binding['id']}",
            "tcb": tcb,
        }
        payload = compact_bytes(context)
        result = subprocess.run(
            [sys.executable, "-I", str(validator), PROTOCOL],
            cwd=ferric,
            check=False,
            input=payload + b"\n",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=60,
            env={"PATH": os.environ.get("PATH", "")},
        )
        expected = (
            f"PASS: {PROTOCOL} artifact_sha256={digest_bytes(raw)} "
            f"context_sha256={digest_bytes(payload)}\n"
        ).encode("ascii")
        if result.returncode != 0 or result.stdout != expected:
            fail(
                f"trusted validator rejected produced {binding['id']}: "
                f"exit={result.returncode}, output={result.stdout!r}"
            )
        observed.append(binding["id"])
    if (
        len(observed) != 36
        or digest_bytes(("\n".join(observed) + "\n").encode("ascii"))
        != "534b95746e961c13f470aca4be53fa4d35f54fa5c8efe6a79792a8c28fe7e645"
    ):
        fail("validated performance binding roster drifted")


def copy_plan(source: Path, destination: Path) -> Path:
    shutil.copytree(source, destination)
    return destination


def expect_cli_failure(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    intake: Path,
    binding_id: str,
    expected: str,
) -> None:
    result = invoke_cli(producer, ferric, fe2o3, plan, intake, binding_id)
    if result.returncode == 0 or expected not in result.stdout:
        fail(
            f"performance CLI accepted hostile fixture; expected {expected!r}:\n"
            f"{result.stdout}"
        )


def cli_hostile_inputs(
    root: Path,
    baseline: Path,
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    intake: Path,
    binding_id: str,
) -> int:
    environment_plan = copy_plan(baseline, root / "inherited-git-environment")
    environment_result = invoke_cli(
        producer,
        ferric,
        fe2o3,
        environment_plan,
        intake,
        binding_id,
        {
            "GIT_CONFIG_GLOBAL": str(root / "attacker.gitconfig"),
            "GIT_DIR": str(root / "attacker.git"),
            "GIT_WORK_TREE": str(root / "attacker-work-tree"),
        },
    )
    if environment_result.returncode != 0:
        fail(
            "performance CLI did not neutralize a hostile inherited Git environment:\n"
            f"{environment_result.stdout}"
        )

    public_parent = root / "public-intake-parent"
    public_parent.mkdir(mode=0o755)
    public_parent.chmod(0o755)
    public_intake = public_parent / "intake.json"
    public_intake.write_bytes(intake.read_bytes())
    public_intake.chmod(0o600)
    expect_cli_failure(
        producer,
        ferric,
        fe2o3,
        baseline,
        public_intake,
        binding_id,
        "performance intake parent directory must be an exact owner-private 0700 directory",
    )

    plan_root = copy_plan(baseline, root / "plan-drift")
    plan_path = plan_root / "plan.json"
    queue_path = plan_root / "missing-work.json"
    plan = read_json(plan_path)
    plan["target"] = "gfx942:xnack+"
    write_json(plan_path, plan)
    queue = read_json(queue_path)
    queue["plan_sha256"] = digest_bytes(plan_path.read_bytes())
    write_json(queue_path, queue)
    expect_cli_failure(
        producer,
        ferric,
        fe2o3,
        plan_root,
        intake,
        binding_id,
        "not the exact current planner output",
    )

    plan_root = copy_plan(baseline, root / "queue-drift")
    queue_path = plan_root / "missing-work.json"
    queue = read_json(queue_path)
    queue["counts"]["available_producer_items"] -= 1
    write_json(queue_path, queue)
    expect_cli_failure(
        producer,
        ferric,
        fe2o3,
        plan_root,
        intake,
        binding_id,
        "work-queue counts drifted",
    )

    plan_root = copy_plan(baseline, root / "source-closure-drift")
    closure = plan_root / "source-closures/source.ferric.records"
    closure.write_bytes(closure.read_bytes() + b"attacker|600|1|" + b"a" * 64 + b"\n")
    closure.chmod(0o600)
    expect_cli_failure(
        producer,
        ferric,
        fe2o3,
        plan_root,
        intake,
        binding_id,
        "source-closure bytes or declaration drifted",
    )

    plan_root = copy_plan(baseline, root / "tcb-drift")
    report = plan_root / "artifacts/artifact.tcb.compiler.tcb-report.json"
    value = read_json(report)
    value["component_roster"][0]["authority"] = "attacker-authored"
    write_json(report, value)
    expect_cli_failure(
        producer,
        ferric,
        fe2o3,
        plan_root,
        intake,
        binding_id,
        "not the exact authenticated projection",
    )
    return 6


def full_cli_policy(
    repo: Path,
    fe2o3_source: Path,
    intake_value: dict[str, Any],
    root: Path,
) -> int:
    ferric = root / "ferric"
    clone_at(repo, ferric)
    shutil.copytree(
        repo / "proofs/m1-qualification",
        ferric / "proofs/m1-qualification",
        dirs_exist_ok=True,
        ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
    )
    if command(
        [
            "git",
            "-C",
            str(ferric),
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
        ],
        "inspect performance Ferric fixture",
    ):
        commit_fixture(ferric)
    cargo = (ferric / "Cargo.toml").read_text(encoding="utf-8")
    marker = 'fe2o3-amdhsa-loader = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "'
    if cargo.count(marker) != 1:
        fail("cannot locate exact fe2o3 revision")
    revision = cargo.split(marker, 1)[1].split('"', 1)[0]
    fe2o3 = root / "fe2o3"
    clone_at(fe2o3_source, fe2o3, revision)

    external = root / "external-performance"
    external.mkdir(mode=0o700)
    intake = external / "intake.json"
    intake.write_bytes(canonical_bytes(intake_value))
    intake.chmod(0o600)
    planner = ferric / "proofs/m1-qualification/planner.py"
    tcb_producer = ferric / "proofs/m1-qualification/produce-tcb-report.py"
    producer = ferric / "proofs/m1-qualification/produce-performance-report.py"
    validator = ferric / "proofs/m1/evidence/validate-performance-report.py"
    baseline = root / "baseline"
    run_planner(planner, ferric, fe2o3, baseline)
    materialize_tcb(tcb_producer, ferric, fe2o3, baseline)
    plan = read_json(baseline / "plan.json")
    queue = read_json(baseline / "missing-work.json")
    if queue["counts"] != {
        "available_producer_items": 357,
        "missing_items": 358,
        "missing_producer_items": 1,
    }:
        fail(f"performance CLI fixture queue-count drifted: {queue['counts']}")
    slots = performance_slots(plan)
    if len(slots) != 36:
        fail("planner did not expose exactly 36 performance bindings")
    canonical = copy_plan(baseline, root / "canonical")
    futures = {}
    with ThreadPoolExecutor(max_workers=4) as pool:
        for ordinal, slot in enumerate(slots, 1):
            binding_id = slot["binding"]["id"]
            future = pool.submit(
                invoke_cli,
                producer,
                ferric,
                fe2o3,
                canonical,
                intake,
                binding_id,
            )
            futures[future] = (ordinal, binding_id)
        for future in as_completed(futures):
            ordinal, binding_id = futures[future]
            try:
                result = future.result()
            except BaseException as error:
                fail(
                    f"performance CLI crashed for canonical binding "
                    f"{ordinal}/{len(slots)} {binding_id}: {error}"
                )
            if result.returncode != 0:
                fail(
                    f"performance CLI rejected canonical binding "
                    f"{ordinal}/{len(slots)} {binding_id}:\n{result.stdout}"
                )
    validate_all_cli_reports(validator, canonical, plan, ferric)
    outputs = [
        *canonical.joinpath("artifacts").glob("*performance-report.json"),
        *canonical.joinpath("measurements").glob("*.measurements.json"),
    ]
    if len(outputs) != 72:
        fail(f"performance producer output count drifted: {len(outputs)}")
    if any(
        (canonical / name).exists() for name in ("evidence-index.json", "receipt.json")
    ):
        fail("performance producer emitted a forbidden closure output")

    hostile_root = root / "cli-hostile"
    hostile_root.mkdir(mode=0o700)
    hostile_count = cli_hostile_inputs(
        hostile_root,
        baseline,
        producer,
        ferric,
        fe2o3,
        intake,
        slots[0]["binding"]["id"],
    )
    if command(
        ["git", "-C", str(ferric), "status", "--porcelain=v1", "--untracked-files=all"],
        "recheck performance Ferric fixture",
    ) or command(
        ["git", "-C", str(fe2o3), "status", "--porcelain=v1", "--untracked-files=all"],
        "recheck performance fe2o3 fixture",
    ):
        fail("performance production dirtied an exact source repository")
    return hostile_count


def audit_separation(repo: Path, producer_path: Path) -> None:
    raw = producer_path.read_text(encoding="ascii")
    forbidden = (
        "validate-performance-report.py",
        "ferric.m1-validator.performance-report.v1",
    )
    if any(item in raw for item in forbidden):
        fail("performance producer invokes or names the trusted validator")
    checker = load_module(
        repo / "proofs/check-m1-evidence-index.py", "performance_checker_pin"
    )
    validator = repo / "proofs/m1/evidence/validate-performance-report.py"
    expected = (
        "proofs/m1/evidence/validate-performance-report.py",
        PROTOCOL,
        hashlib.sha256(validator.read_bytes()).hexdigest(),
    )
    if checker.TRUSTED_VALIDATORS.get("performance-gate") != expected:
        fail("checker-owned performance validator pin drifted")


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_OBJECT_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    fe2o3_source = Path(sys.argv[2]).resolve(strict=True)
    producer_path = repo / "proofs/m1-qualification/produce-performance-report.py"
    validator_path = repo / "proofs/m1/evidence/validate-performance-report.py"
    validator_policy_path = (
        repo / "proofs/m1/evidence/test-performance-report-policy.py"
    )
    tcb_producer_path = repo / "proofs/m1-qualification/produce-tcb-report.py"
    producer = load_module(producer_path, "ferric_performance_producer")
    validator = load_module(validator_path, "ferric_performance_validator_for_fixture")
    validator_policy = load_module(
        validator_policy_path, "ferric_performance_validator_policy_fixture"
    )
    tcb_producer = load_module(tcb_producer_path, "ferric_reviewed_tcb_producer")
    audit_separation(repo, producer_path)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-performance-producer.") as raw:
        root = Path(raw)
        intake, context, slot, _, measurement_bytes, report_bytes = make_intake(
            repo, producer, validator_policy, validator, root / "fixture"
        )
        hostile_count = validate_intake_hostiles(producer, intake)
        hostile_count += custody_policy(producer, intake, root)
        hostile_count += policy_source_custody_policy(producer, root)
        hostile_count += rebound_cleanup_policy(producer, root)
        hostile_count += tcb_authentication_policy(
            repo, producer, tcb_producer, context["sources"], root
        )
        hostile_count += disjointness_policy(producer, root)
        publication_policy(
            producer,
            validator_path,
            context,
            slot,
            measurement_bytes,
            report_bytes,
            root,
        )
        rollback_policy(
            producer,
            slot["binding"]["artifact_id"],
            measurement_bytes,
            report_bytes,
            root,
        )
        hostile_count += full_cli_policy(repo, fe2o3_source, intake, root)
    print(
        "PASS: M1 performance producer emitted and separately validated all 36 "
        "bindings from one synthetic mechanics-only intake, preserved measurements-first "
        f"publication, handled {hostile_count} hostile cases, and rolled back a "
        "partial publication; this is not performance evidence"
    )


if __name__ == "__main__":
    main()
