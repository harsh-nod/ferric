#!/usr/bin/env python3
"""Exercise all five planner-bound M1 unsupported-rationale producers."""

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
from types import ModuleType
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.unsupported-rationale.v1"
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
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def read_json(path: Path) -> dict[str, Any]:
    raw = path.read_bytes()
    value = json.loads(raw)
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"fixture is not canonical JSON: {path}")
    return value


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_bytes(canonical_bytes(value))


def command(arguments: list[str], description: str, *, cwd: Path | None = None) -> str:
    result = subprocess.run(
        arguments,
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=240,
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
            "user.name=M1 Unsupported Rationale Policy",
            "-c",
            "user.email=m1-unsupported-rationale@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "add M1 unsupported-rationale producer",
        ],
        "commit fixture",
    )


def invoke(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    binding_id: str,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "-I",
            str(producer),
            str(ferric),
            str(fe2o3),
            str(plan),
            binding_id,
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=240,
        env={"PATH": os.environ.get("PATH", "")},
    )


def expect_failure(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    binding_id: str,
    expected: str,
) -> None:
    result = invoke(producer, ferric, fe2o3, plan, binding_id)
    if result.returncode == 0 or expected not in result.stdout:
        fail(
            f"rationale producer accepted hostile input; expected {expected!r}:\n"
            f"{result.stdout}"
        )


def run_planner(planner: Path, ferric: Path, fe2o3: Path, output: Path) -> None:
    result = subprocess.run(
        [sys.executable, "-I", str(planner), str(ferric), str(fe2o3), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=240,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"planner rejected rationale producer fixture:\n{result.stdout}")


def materialize_tcb(producer: Path, ferric: Path, fe2o3: Path, plan: Path) -> None:
    for subject, _ in TCB:
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
            timeout=240,
            env={"PATH": os.environ.get("PATH", "")},
        )
        if result.returncode != 0:
            fail(f"TCB prerequisite failed for {subject}:\n{result.stdout}")


def rationale_slots(plan: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        slot
        for slot in plan["binding_slots"]
        if slot["binding"]["evidence_kind"] == "unsupported-rationale"
    ]


def outer_tcb(plan_root: Path) -> list[dict[str, str]]:
    result = []
    for subject, kind in TCB:
        artifact_id = f"artifact.{subject}"
        raw = (plan_root / "artifacts" / f"{artifact_id}.tcb-report.json").read_bytes()
        result.append(
            {
                "artifact_id": artifact_id,
                "id": subject,
                "identity_sha256": digest_bytes(raw),
                "kind": kind,
            }
        )
    return result


def validate_all_reports(
    validator: Path,
    plan_root: Path,
    plan: dict[str, Any],
    ferric: Path,
) -> None:
    resolutions = {row["id"]: row for row in plan["path_resolutions"]}
    tcb = outer_tcb(plan_root)
    observed = []
    for slot in rationale_slots(plan):
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
        len(observed) != 5
        or digest_bytes(("\n".join(observed) + "\n").encode("ascii"))
        != "234623d24473bb78252a0541395d68f09b591d7e947c8e55e286a2e8b57a6b81"
    ):
        fail("validated unsupported-rationale roster drifted")


def copy_plan(source: Path, destination: Path) -> Path:
    shutil.copytree(source, destination)
    return destination


def mutate_plan(plan_root: Path, edit: Callable[[dict[str, Any]], None]) -> None:
    plan_path = plan_root / "plan.json"
    queue_path = plan_root / "missing-work.json"
    plan = read_json(plan_path)
    edit(plan)
    write_json(plan_path, plan)
    queue = read_json(queue_path)
    queue["plan_sha256"] = digest_bytes(plan_path.read_bytes())
    write_json(queue_path, queue)


def mutate_queue(plan_root: Path, edit: Callable[[dict[str, Any]], None]) -> None:
    queue_path = plan_root / "missing-work.json"
    queue = read_json(queue_path)
    edit(queue)
    write_json(queue_path, queue)


def set_binding_field(plan: dict[str, Any], ordinal: int, key: str, value: str) -> None:
    binding = plan["binding_slots"][ordinal]["binding"]
    binding[key] = value
    payload = {name: item for name, item in binding.items() if name != "binding_sha256"}
    binding["binding_sha256"] = digest_bytes(compact_bytes(payload))


def load_producer(path: Path) -> ModuleType:
    sys.dont_write_bytecode = True
    specification = importlib.util.spec_from_file_location("rationale_producer", path)
    if specification is None or specification.loader is None:
        fail("cannot load rationale producer race policy")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def expect_direct_failure(action: Callable[[], None], expected: str) -> None:
    errors = io.StringIO()
    try:
        with contextlib.redirect_stderr(errors):
            action()
    except SystemExit:
        pass
    else:
        fail("descriptor-race policy accepted a replacement")
    if expected not in errors.getvalue():
        fail(
            f"descriptor-race failure mismatch, expected {expected!r}: {errors.getvalue()}"
        )


def publication_races(root: Path, baseline: Path, producer: Path) -> int:
    module = load_producer(producer)
    cases = 0
    for race in ("plan", "artifact"):
        plan = copy_plan(baseline, root / f"race-{race}")
        plan_fd = module.open_private_directory(plan, "race plan")
        artifact_fd = module.open_private_directory_at(
            plan_fd, "artifacts", "race artifact directory"
        )
        original = module.create_new_file_at
        replaced = False

        def intercept(
            directory_fd: int, name: str, value: bytes, description: str
        ) -> int:
            nonlocal replaced
            if not replaced:
                replaced = True
                if race == "plan":
                    held = plan.with_name(f"{plan.name}-held")
                    plan.rename(held)
                    plan.mkdir(mode=0o700)
                else:
                    current = plan / "artifacts"
                    current.rename(plan / "artifacts-held")
                    current.mkdir(mode=0o700)
            return original(directory_fd, name, value, description)

        module.create_new_file_at = intercept
        expected = {
            "plan": "plan directory was replaced",
            "artifact": "artifact directory was replaced",
        }[race]
        artifact_id = "artifact.binding.00259"
        report_name = f"{artifact_id}.unsupported-rationale.json"
        try:
            expect_direct_failure(
                lambda: module.publish_rationale(
                    plan,
                    plan_fd,
                    artifact_fd,
                    artifact_id,
                    b"{}\n",
                    lambda: None,
                ),
                expected,
            )
        finally:
            module.create_new_file_at = original
            os.close(artifact_fd)
            os.close(plan_fd)
        candidates = [plan / "artifacts" / report_name]
        if race == "plan":
            candidates.append(
                plan.with_name(f"{plan.name}-held") / "artifacts" / report_name
            )
        else:
            candidates.append(plan / "artifacts-held" / report_name)
        if any(path.exists() for path in candidates):
            fail(f"{race} replacement left a false rationale completion marker")
        cases += 1
    return cases


def completion_marker_failures(root: Path, baseline: Path, producer: Path) -> int:
    module = load_producer(producer)
    cases = 0
    for phase, failure_call in (("pre-report", 1), ("post-report", 2)):
        plan = copy_plan(baseline, root / f"failure-{phase}")
        plan_fd = module.open_private_directory(plan, f"{phase} plan")
        artifact_fd = module.open_private_directory_at(
            plan_fd, "artifacts", f"{phase} artifact directory"
        )
        calls = 0

        def reject_completion() -> None:
            nonlocal calls
            calls += 1
            if calls == failure_call:
                module.fail(f"injected {phase} input drift")

        artifact_id = "artifact.binding.00259"
        report = plan / "artifacts" / f"{artifact_id}.unsupported-rationale.json"
        try:
            expect_direct_failure(
                lambda: module.publish_rationale(
                    plan,
                    plan_fd,
                    artifact_fd,
                    artifact_id,
                    b"{}\n",
                    reject_completion,
                ),
                f"injected {phase} input drift",
            )
            if report.exists():
                fail(f"{phase} failure left a false rationale completion marker")
            module.publish_rationale(
                plan,
                plan_fd,
                artifact_fd,
                artifact_id,
                b"{}\n",
                lambda: None,
            )
            if report.read_bytes() != b"{}\n":
                fail(f"{phase} retry did not publish the exact rationale report")
        finally:
            os.close(artifact_fd)
            os.close(plan_fd)
        cases += 2
    return cases


def plan_file_races(root: Path, baseline: Path, producer: Path) -> int:
    module = load_producer(producer)
    cases = 0
    for race, name, artifact_id in (
        ("replacement", "plan.json", "artifact.binding.00259"),
        ("in-place", "missing-work.json", "artifact.binding.00260"),
    ):
        plan = copy_plan(baseline, root / f"race-plan-file-{race}")
        plan_fd = module.open_private_directory(plan, f"{race} plan")
        artifact_fd = module.open_private_directory_at(
            plan_fd, "artifacts", f"{race} artifact directory"
        )
        raw = (plan / name).read_bytes()
        held = module.authenticate_held_file_at(plan_fd, name, raw, f"race {name}")
        calls = 0

        def race_plan_file() -> None:
            nonlocal calls
            calls += 1
            if calls == 2:
                path = plan / name
                if race == "replacement":
                    path.rename(plan / f"{name}.held")
                    path.write_bytes(raw)
                    path.chmod(0o600)
                else:
                    with path.open("r+b") as output:
                        output.write(b"X")
                        output.flush()
                        os.fsync(output.fileno())
            module.revalidate_held_file(plan_fd, held)

        report = plan / "artifacts" / f"{artifact_id}.unsupported-rationale.json"
        try:
            expect_direct_failure(
                lambda: module.publish_rationale(
                    plan,
                    plan_fd,
                    artifact_fd,
                    artifact_id,
                    b"{}\n",
                    race_plan_file,
                ),
                f"race {name} changed after authentication",
            )
            if report.exists():
                fail(f"{race} {name} race left a false completion marker")
        finally:
            held[1].close()
            os.close(artifact_fd)
            os.close(plan_fd)
        cases += 1
    return cases


def published_byte_race(root: Path, baseline: Path, producer: Path) -> int:
    module = load_producer(producer)
    plan = copy_plan(baseline, root / "race-published-report")
    plan_fd = module.open_private_directory(plan, "report byte-race plan")
    artifact_fd = module.open_private_directory_at(
        plan_fd, "artifacts", "report byte-race artifact directory"
    )
    artifact_id = "artifact.binding.00259"
    report = plan / "artifacts" / f"{artifact_id}.unsupported-rationale.json"
    calls = 0

    def mutate_after_readback() -> None:
        nonlocal calls
        calls += 1
        if calls != 2:
            return
        raw = report.read_bytes()
        with report.open("r+b") as output:
            output.write(bytes([raw[0] ^ 1]) + raw[1:])
            output.flush()
            os.fsync(output.fileno())

    try:
        expect_direct_failure(
            lambda: module.publish_rationale(
                plan,
                plan_fd,
                artifact_fd,
                artifact_id,
                b"{}\n",
                mutate_after_readback,
            ),
            "published M1 unsupported-rationale report bytes or binding changed",
        )
        if report.exists():
            fail("same-size report overwrite left a false completion marker")
    finally:
        os.close(artifact_fd)
        os.close(plan_fd)
    return 1


def repository_replacement_race(root: Path, producer: Path) -> int:
    module = load_producer(producer)
    repository = root / "race-repository"
    repository.mkdir()
    repository_fd = module.open_directory(repository, "race repository")
    repository.rename(root / "race-repository-held")
    repository.mkdir()
    try:
        expect_direct_failure(
            lambda: module.revalidate_directory(
                repository, repository_fd, "race repository"
            ),
            "race repository was replaced",
        )
    finally:
        os.close(repository_fd)
    return 1


def repository_commit_transition_race(
    root: Path, baseline: Path, producer: Path
) -> int:
    module = load_producer(producer)
    repository = root / "clean-transition-repository"
    repository.mkdir()
    command(["git", "-C", str(repository), "init", "--quiet"], "init race repo")
    source = repository / "source.txt"
    source.write_text("first\n", encoding="ascii")
    command(["git", "-C", str(repository), "add", "source.txt"], "stage first")
    command(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=M1 Rationale Race",
            "-c",
            "user.email=m1-rationale-race@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "first",
        ],
        "commit first",
    )
    first_commit = command(
        ["git", "-C", str(repository), "rev-parse", "HEAD"], "identify first commit"
    )
    first_tree = command(
        ["git", "-C", str(repository), "rev-parse", "HEAD^{tree}"],
        "identify first tree",
    )
    source.write_text("second\n", encoding="ascii")
    command(["git", "-C", str(repository), "add", "source.txt"], "stage second")
    command(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=M1 Rationale Race",
            "-c",
            "user.email=m1-rationale-race@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "second",
        ],
        "commit second",
    )
    second_commit = command(
        ["git", "-C", str(repository), "rev-parse", "HEAD"], "identify second commit"
    )
    command(
        ["git", "-C", str(repository), "checkout", "--quiet", first_commit],
        "restore authenticated commit",
    )
    repository_fd = module.open_directory(repository, "transition repository")
    plan = copy_plan(baseline, root / "race-clean-commit-transition")
    plan_fd = module.open_private_directory(plan, "transition plan")
    artifact_fd = module.open_private_directory_at(
        plan_fd, "artifacts", "transition artifact directory"
    )
    artifact_id = "artifact.binding.00287"
    report = plan / "artifacts" / f"{artifact_id}.unsupported-rationale.json"
    calls = 0

    def transition_checkout() -> None:
        nonlocal calls
        calls += 1
        if calls == 2:
            command(
                ["git", "-C", str(repository), "checkout", "--quiet", second_commit],
                "inject clean commit transition",
            )
        module.revalidate_repository_identities(
            {"ferric": (repository, repository_fd)},
            {"ferric": (first_commit, first_tree)},
        )

    try:
        expect_direct_failure(
            lambda: module.publish_rationale(
                plan,
                plan_fd,
                artifact_fd,
                artifact_id,
                b"{}\n",
                transition_checkout,
            ),
            "authenticated source commit or tree changed",
        )
        if report.exists():
            fail("clean commit transition left a false rationale completion marker")
    finally:
        os.close(artifact_fd)
        os.close(plan_fd)
        os.close(repository_fd)
    return 1


def source_closure_races(root: Path, baseline: Path, producer: Path) -> int:
    module = load_producer(producer)
    cases = 0
    for race, artifact_id in (
        ("replacement", "artifact.binding.00285"),
        ("in-place", "artifact.binding.00286"),
    ):
        plan = copy_plan(baseline, root / f"race-source-closure-{race}")
        plan_value = read_json(plan / "plan.json")
        plan_fd = module.open_private_directory(plan, f"{race} closure plan")
        artifact_fd = module.open_private_directory_at(
            plan_fd, "artifacts", f"{race} closure artifact directory"
        )
        custody = module.authenticate_source_closures(plan_fd, plan_value)
        report = plan / "artifacts" / f"{artifact_id}.unsupported-rationale.json"
        closure = plan / "source-closures/source.fe2o3.records"
        calls = 0

        def mutate_closure() -> None:
            nonlocal calls
            calls += 1
            if calls == 2:
                raw = closure.read_bytes()
                if race == "replacement":
                    closure.rename(closure.with_suffix(".held"))
                    closure.write_bytes(raw)
                    closure.chmod(0o600)
                else:
                    with closure.open("r+b") as output:
                        output.write(bytes([raw[0] ^ 1]) + raw[1:])
                        output.flush()
                        os.fsync(output.fileno())
            module.revalidate_source_closures(plan_fd, custody)

        try:
            expect_direct_failure(
                lambda: module.publish_rationale(
                    plan,
                    plan_fd,
                    artifact_fd,
                    artifact_id,
                    b"{}\n",
                    mutate_closure,
                ),
                "source closure source.fe2o3 changed after authentication",
            )
            if report.exists():
                fail(f"{race} source closure race left a false completion marker")
        finally:
            module.close_source_closures(custody)
            os.close(artifact_fd)
            os.close(plan_fd)
        cases += 1
    return cases


def tcb_replacement_race(
    root: Path, baseline: Path, producer: Path, ferric: Path
) -> int:
    module = load_producer(producer)
    requirements = read_json(ferric / "proofs/M1_REQUIREMENTS.json")
    validators = module.trusted_validators(ferric)[1]
    cases = 0
    for race in ("replacement", "in-place"):
        plan_root = copy_plan(baseline, root / f"race-tcb-report-{race}")
        plan = read_json(plan_root / "plan.json")
        plan_fd = module.open_private_directory(plan_root, f"{race} TCB plan")
        artifact_fd = module.open_private_directory_at(
            plan_fd, "artifacts", f"{race} TCB artifact directory"
        )
        held: list[tuple[str, str, Any, os.stat_result, bytes]] = []
        try:
            _, held = module.authenticate_tcb_reports(
                artifact_fd, ferric, requirements, plan["sources"], validators
            )
            report = plan_root / "artifacts/artifact.tcb.compiler.tcb-report.json"
            raw = report.read_bytes()
            if race == "replacement":
                report.rename(report.with_suffix(".held"))
                report.write_bytes(raw)
                report.chmod(0o600)
            else:
                with report.open("r+b") as output:
                    output.write(bytes([raw[0] ^ 1]) + raw[1:])
                    output.flush()
                    os.fsync(output.fileno())
            expect_direct_failure(
                lambda: module.revalidate_tcb_reports(
                    artifact_fd,
                    held,
                    ferric,
                    requirements,
                    plan["sources"],
                    validators,
                ),
                "TCB report changed after authentication",
            )
        finally:
            for _, _, source, _, _ in held:
                source.close()
            os.close(artifact_fd)
            os.close(plan_fd)
        cases += 1
    return cases


def hostile_inputs(
    root: Path,
    baseline: Path,
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    binding_id: str,
) -> int:
    cases = 0

    wrong = copy_plan(baseline, root / "wrong-binding")
    expect_failure(producer, ferric, fe2o3, wrong, "binding.99999", "unknown M1")
    cases += 1

    semantic = copy_plan(baseline, root / "semantic-plan")
    mutate_plan(semantic, lambda value: value["binding_slots"].pop())
    expect_failure(
        producer, ferric, fe2o3, semantic, binding_id, "differs from exact rederivation"
    )
    cases += 1

    wrong_path = copy_plan(baseline, root / "wrong-path")
    mutate_plan(
        wrong_path,
        lambda value: set_binding_field(value, 259, "path_id", "speculation-proof"),
    )
    expect_failure(
        producer, ferric, fe2o3, wrong_path, binding_id, "exact rederivation"
    )
    cases += 1

    wrong_source = copy_plan(baseline, root / "wrong-source")
    mutate_plan(
        wrong_source,
        lambda value: set_binding_field(
            value, 259, "source_identity_id", "source.fe2o3"
        ),
    )
    expect_failure(
        producer, ferric, fe2o3, wrong_source, binding_id, "exact rederivation"
    )
    cases += 1

    queue_command = copy_plan(baseline, root / "queue-command")
    mutate_queue(
        queue_command,
        lambda value: value["items"][259]["producer"].__setitem__("command", None),
    )
    expect_failure(
        producer, ferric, fe2o3, queue_command, binding_id, "exact rederivation"
    )
    cases += 1

    missing_tcb = copy_plan(baseline, root / "missing-tcb")
    (missing_tcb / "artifacts/artifact.tcb.hardware.tcb-report.json").unlink()
    expect_failure(producer, ferric, fe2o3, missing_tcb, binding_id, "TCB report")
    cases += 1

    tampered_tcb = copy_plan(baseline, root / "tampered-tcb")
    with (tampered_tcb / "artifacts/artifact.tcb.compiler.tcb-report.json").open(
        "ab"
    ) as output:
        output.write(b"hostile\n")
    expect_failure(
        producer,
        ferric,
        fe2o3,
        tampered_tcb,
        binding_id,
        "exact authenticated projection",
    )
    cases += 1

    reordered_tcb = copy_plan(baseline, root / "reordered-tcb")
    compiler = reordered_tcb / "artifacts/artifact.tcb.compiler.tcb-report.json"
    runtime = reordered_tcb / "artifacts/artifact.tcb.runtime.tcb-report.json"
    compiler_raw, runtime_raw = compiler.read_bytes(), runtime.read_bytes()
    compiler.write_bytes(runtime_raw)
    runtime.write_bytes(compiler_raw)
    expect_failure(
        producer,
        ferric,
        fe2o3,
        reordered_tcb,
        binding_id,
        "exact authenticated projection",
    )
    cases += 1

    tcb_symlink = copy_plan(baseline, root / "tcb-symlink")
    report = tcb_symlink / "artifacts/artifact.tcb.compiler.tcb-report.json"
    target = tcb_symlink / "compiler-report"
    report.rename(target)
    report.symlink_to(target)
    expect_failure(producer, ferric, fe2o3, tcb_symlink, binding_id, "TCB report")
    cases += 1

    tcb_permissions = copy_plan(baseline, root / "tcb-permissions")
    (tcb_permissions / "artifacts/artifact.tcb.runtime.tcb-report.json").chmod(0o644)
    expect_failure(
        producer, ferric, fe2o3, tcb_permissions, binding_id, "owner-private 0600"
    )
    cases += 1

    artifact_permissions = copy_plan(baseline, root / "artifact-permissions")
    (artifact_permissions / "artifacts").chmod(0o755)
    expect_failure(
        producer, ferric, fe2o3, artifact_permissions, binding_id, "owner-private 0700"
    )
    cases += 1

    plan_permissions = copy_plan(baseline, root / "plan-permissions")
    plan_permissions.chmod(0o755)
    expect_failure(
        producer, ferric, fe2o3, plan_permissions, binding_id, "owner-private 0700"
    )
    cases += 1

    artifact_symlink = copy_plan(baseline, root / "artifact-symlink")
    artifact_target = root / "artifact-symlink-target"
    shutil.copytree(artifact_symlink / "artifacts", artifact_target)
    shutil.rmtree(artifact_symlink / "artifacts")
    (artifact_symlink / "artifacts").symlink_to(artifact_target)
    expect_failure(
        producer,
        ferric,
        fe2o3,
        artifact_symlink,
        binding_id,
        "artifact directory",
    )
    cases += 1

    plan_link = root / "plan-symlink"
    plan_link.symlink_to(baseline, target_is_directory=True)
    expect_failure(producer, ferric, fe2o3, plan_link, binding_id, "contains a symlink")
    cases += 1

    plan_file_permissions = copy_plan(baseline, root / "plan-file-permissions")
    (plan_file_permissions / "plan.json").chmod(0o644)
    expect_failure(
        producer,
        ferric,
        fe2o3,
        plan_file_permissions,
        binding_id,
        "owner-private expected file",
    )
    cases += 1

    closure = copy_plan(baseline, root / "closure-output")
    (closure / "receipt.json").write_text("hostile\n", encoding="ascii")
    expect_failure(producer, ferric, fe2o3, closure, binding_id, "closure output")
    cases += 1

    selected = next(
        slot
        for slot in rationale_slots(read_json(baseline / "plan.json"))
        if slot["binding"]["id"] == binding_id
    )
    preexisting_report = copy_plan(baseline, root / "preexisting-report")
    (preexisting_report / selected["expected_artifact"]["path"]).write_text(
        "hostile\n", encoding="ascii"
    )
    expect_failure(
        producer, ferric, fe2o3, preexisting_report, binding_id, "preexisting output"
    )
    cases += 1

    preexisting_symlink = copy_plan(baseline, root / "preexisting-symlink")
    target_report = root / "hostile-rationale-report"
    target_report.write_bytes(b"hostile\n")
    (preexisting_symlink / selected["expected_artifact"]["path"]).symlink_to(
        target_report
    )
    expect_failure(
        producer, ferric, fe2o3, preexisting_symlink, binding_id, "preexisting output"
    )
    cases += 1

    dirty = ferric / "unsupported-rationale-hostile-untracked"
    dirty.write_bytes(b"hostile\n")
    try:
        expect_failure(
            producer, ferric, fe2o3, baseline, binding_id, "exact clean worktree"
        )
    finally:
        dirty.unlink()
    cases += 1
    return cases


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_OBJECT_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    fe2o3_source = Path(sys.argv[2]).resolve(strict=True)
    producer_source = (
        repo / "proofs/m1-qualification/produce-unsupported-rationale.py"
    ).read_text(encoding="ascii")
    forbidden_copy_paths = (
        "def create_new_report_at(",
        "def publish_report(",
        "def publish_identity(",
        "def artifact_identity_report(",
        "identified-artifacts",
        "produced M1 TCB report",
        "TCB producer created a forbidden closure output",
        "tcb.compiler|tcb.hardware|tcb.runtime",
    )
    if any(token in producer_source for token in forbidden_copy_paths):
        fail("rationale producer retains a stale copied publication path")
    with tempfile.TemporaryDirectory(prefix="ferric-m1-rationale-producer-") as raw:
        root = Path(raw)
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
            "inspect Ferric fixture",
        ):
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
        producer = ferric / "proofs/m1-qualification/produce-unsupported-rationale.py"
        validator = ferric / "proofs/m1/evidence/validate-unsupported-rationale.py"
        baseline = root / "baseline"
        run_planner(planner, ferric, fe2o3, baseline)
        materialize_tcb(tcb_producer, ferric, fe2o3, baseline)
        plan = read_json(baseline / "plan.json")
        slots = rationale_slots(plan)
        if len(slots) != 5:
            fail("planner did not expose exactly five unsupported-rationale bindings")

        canonical = copy_plan(baseline, root / "canonical")
        for ordinal, slot in enumerate(slots, 1):
            binding_id = slot["binding"]["id"]
            result = invoke(producer, ferric, fe2o3, canonical, binding_id)
            if result.returncode != 0:
                fail(
                    f"producer rejected canonical binding {ordinal}/{len(slots)} "
                    f"{binding_id}:\n"
                    f"{result.stdout}"
                )
        validate_all_reports(validator, canonical, plan, ferric)
        if any(
            (canonical / name).exists()
            for name in ("evidence-index.json", "receipt.json")
        ):
            fail("rationale producer emitted a forbidden closure output")

        hostile_root = root / "hostile"
        hostile_root.mkdir()
        hostile_count = hostile_inputs(
            hostile_root,
            baseline,
            producer,
            ferric,
            fe2o3,
            slots[0]["binding"]["id"],
        )
        hostile_count += publication_races(hostile_root, baseline, producer)
        hostile_count += completion_marker_failures(hostile_root, baseline, producer)
        hostile_count += plan_file_races(hostile_root, baseline, producer)
        hostile_count += published_byte_race(hostile_root, baseline, producer)
        hostile_count += repository_replacement_race(hostile_root, producer)
        hostile_count += repository_commit_transition_race(
            hostile_root, baseline, producer
        )
        hostile_count += source_closure_races(hostile_root, baseline, producer)
        hostile_count += tcb_replacement_race(hostile_root, baseline, producer, ferric)
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
            fail("rationale production dirtied an exact source repository")
    print(
        "PASS: M1 unsupported-rationale producer emitted and validated all 5 bindings "
        f"and rejected {hostile_count} hostile inputs"
    )


if __name__ == "__main__":
    main()
