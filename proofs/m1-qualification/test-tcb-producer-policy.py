#!/usr/bin/env python3
"""Exercise the planner-bound M1 TCB-report producer."""

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


SUBJECTS = (
    ("tcb.compiler", "Compiler"),
    ("tcb.hardware", "Hardware"),
    ("tcb.runtime", "Runtime"),
)
PROTOCOL = "ferric.m1-validator.tcb-report.v1"
Mutation = Callable[[Path], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="ascii"))
    if not isinstance(value, dict) or path.read_bytes() != canonical_bytes(value):
        fail(f"test fixture is not canonical JSON: {path}")
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
        timeout=180,
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


def commit_fixture(repository: Path, message: str) -> None:
    command(["git", "-C", str(repository), "add", "-A"], "stage fixture")
    command(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=M1 TCB Producer Policy",
            "-c",
            "user.email=m1-tcb-producer@example.invalid",
            "commit",
            "--quiet",
            "-m",
            message,
        ],
        "commit fixture",
    )


def run_planner(planner: Path, ferric: Path, fe2o3: Path, output: Path) -> None:
    result = subprocess.run(
        [sys.executable, "-I", str(planner), str(ferric), str(fe2o3), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=180,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if (
        result.returncode != 0
        or "PASS: prepared external M1 evidence plan" not in result.stdout
    ):
        fail(f"planner rejected TCB producer fixture:\n{result.stdout}")


def invoke_producer(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    subject: str,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
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
        timeout=180,
        env={"PATH": os.environ.get("PATH", "")},
    )


def expect_producer_pass(
    producer: Path, ferric: Path, fe2o3: Path, plan: Path, subject: str
) -> None:
    result = invoke_producer(producer, ferric, fe2o3, plan, subject)
    if (
        result.returncode != 0
        or f"PASS: produced M1 TCB report subject={subject}" not in result.stdout
    ):
        fail(f"TCB producer rejected canonical {subject} input:\n{result.stdout}")


def expect_producer_failure(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    subject: str,
    expected: str,
) -> None:
    result = invoke_producer(producer, ferric, fe2o3, plan, subject)
    if result.returncode == 0 or expected not in result.stdout:
        fail(
            f"TCB producer accepted hostile fixture; expected {expected!r}:\n"
            f"{result.stdout}"
        )


def report_path(plan: Path, subject: str) -> Path:
    artifact_id = f"artifact.{subject}"
    return plan / "artifacts" / f"{artifact_id}.tcb-report.json"


def validate_reports(ferric: Path, plan_root: Path) -> None:
    plan = read_json(plan_root / "plan.json")
    reports = [
        (subject, kind, report_path(plan_root, subject)) for subject, kind in SUBJECTS
    ]
    tcb = [
        {
            "artifact_id": f"artifact.{subject}",
            "id": subject,
            "identity_sha256": digest_bytes(path.read_bytes()),
            "kind": kind,
        }
        for subject, kind, path in reports
    ]
    if len({record["identity_sha256"] for record in tcb}) != 3:
        fail("the three TCB reports do not have distinct identities")
    validator = ferric / "proofs/m1/evidence/validate-tcb-report.py"
    for (subject, _, path), record in zip(reports, tcb, strict=True):
        raw = path.read_bytes()
        artifact_id = f"artifact.{subject}"
        artifact = {
            "id": artifact_id,
            "kind": "TcbReport",
            "path": f"artifacts/{artifact_id}.tcb-report.json",
            "sha256": digest_bytes(raw),
            "size_bytes": len(raw),
        }
        context = {
            "artifact": artifact,
            "artifact_absolute_path": str(path),
            "format": "ferric.m1-evidence-index.v1",
            "requirements_sha256": plan["requirements"]["sha256"],
            "sources": plan["sources"],
            "subject": f"tcb:{subject}",
            "tcb": tcb,
            "tcb_record": record,
        }
        payload = json.dumps(
            context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
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
            f"PASS: {PROTOCOL} artifact_sha256={artifact['sha256']} "
            f"context_sha256={digest_bytes(payload)}\n"
        ).encode("ascii")
        if result.returncode != 0 or result.stdout != expected:
            fail(
                f"trusted validator rejected produced {subject} report: "
                f"exit={result.returncode}, output={result.stdout!r}"
            )


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


def load_producer(producer: Path) -> ModuleType:
    sys.dont_write_bytecode = True
    specification = importlib.util.spec_from_file_location("m1_tcb_producer", producer)
    if specification is None or specification.loader is None:
        fail("cannot load TCB producer for publication-race policy")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def expect_publication_race_failure(
    module: ModuleType,
    plan: Path,
    replacement: Callable[[], Path],
    expected: str,
) -> Path:
    plan_fd = module.open_private_directory(plan, "test M1 evidence plan directory")
    original = module.create_new_report_at
    redirected: Path | None = None

    def intercept(artifact_fd: int, name: str, value: bytes) -> int:
        nonlocal redirected
        redirected = replacement()
        return original(artifact_fd, name, value)

    module.create_new_report_at = intercept
    errors = io.StringIO()
    try:
        try:
            with contextlib.redirect_stderr(errors):
                module.publish_report(
                    plan, plan_fd, "tcb.compiler", b'{"fixture":"held"}\n'
                )
        except SystemExit:
            pass
        else:
            fail("TCB publication accepted a replaced directory")
    finally:
        module.create_new_report_at = original
        os.close(plan_fd)
    if expected not in errors.getvalue() or redirected is None:
        fail(
            f"TCB publication race did not fail closed; expected {expected!r}:\n"
            f"{errors.getvalue()}"
        )
    return redirected


def exercise_publication_replacements(
    root: Path, baseline: Path, producer: Path
) -> int:
    module = load_producer(producer)
    report_name = "artifact.tcb.compiler.tcb-report.json"

    parent = copy_plan(baseline, root / "replaced-plan-root")
    held_parent = parent.with_name(f"{parent.name}-held")

    def replace_parent() -> Path:
        parent.rename(held_parent)
        parent.mkdir(mode=0o700)
        return held_parent / "artifacts"

    redirected = expect_publication_race_failure(
        module,
        parent,
        replace_parent,
        "M1 evidence plan directory was replaced",
    )
    if (parent / "artifacts" / report_name).exists() or not (
        redirected / report_name
    ).is_file():
        fail("replaced plan root redirected descriptor-relative TCB publication")

    artifact = copy_plan(baseline, root / "replaced-artifact-root")
    held_artifact = artifact / "artifacts-held"

    def replace_artifact() -> Path:
        current = artifact / "artifacts"
        current.rename(held_artifact)
        current.mkdir(mode=0o700)
        return held_artifact

    redirected = expect_publication_race_failure(
        module,
        artifact,
        replace_artifact,
        "M1 artifact directory was replaced",
    )
    if (artifact / "artifacts" / report_name).exists() or not (
        redirected / report_name
    ).is_file():
        fail("replaced artifact root redirected descriptor-relative TCB publication")
    return 2


def exercise_hostile_inputs(
    root: Path,
    baseline: Path,
    producer: Path,
    ferric: Path,
    fe2o3: Path,
) -> int:
    cases = 0

    plan_sha = copy_plan(baseline, root / "plan-sha")
    plan = read_json(plan_sha / "plan.json")
    plan["fe2o3_pins"] = []
    write_json(plan_sha / "plan.json", plan)
    expect_producer_failure(
        producer, ferric, fe2o3, plan_sha, "tcb.compiler", "work queue identity"
    )
    cases += 1

    pins = copy_plan(baseline, root / "semantic-fe2o3-pins")
    mutate_plan(pins, lambda value: value["fe2o3_pins"]["direct"].pop())
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        pins,
        "tcb.compiler",
        "candidate plan or complete work queue differs from exact rederivation",
    )
    cases += 1

    bindings = copy_plan(baseline, root / "semantic-binding-slots")
    mutate_plan(bindings, lambda value: value["binding_slots"].pop())
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        bindings,
        "tcb.compiler",
        "candidate plan or complete work queue differs from exact rederivation",
    )
    cases += 1

    obligations = copy_plan(baseline, root / "semantic-obligation-slots")
    mutate_plan(obligations, lambda value: value["obligation_slots"].pop())
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        obligations,
        "tcb.compiler",
        "candidate plan or complete work queue differs from exact rederivation",
    )
    cases += 1

    complete_queue = copy_plan(baseline, root / "semantic-complete-queue")
    queue_path = complete_queue / "missing-work.json"
    queue = read_json(queue_path)
    non_tcb = next(
        item for item in queue["items"] if not item["id"].startswith("work.tcb.")
    )
    non_tcb["subject"] = f"{non_tcb['subject']}.hostile"
    write_json(queue_path, queue)
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        complete_queue,
        "tcb.compiler",
        "candidate plan or complete work queue differs from exact rederivation",
    )
    cases += 1

    work_command = copy_plan(baseline, root / "work-command")
    queue_path = work_command / "missing-work.json"
    queue = read_json(queue_path)
    tcb_item = next(
        item for item in queue["items"] if item["id"] == "work.tcb.compiler"
    )
    tcb_item["producer"]["command"][-1] = "tcb.runtime"
    write_json(queue_path, queue)
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        work_command,
        "tcb.compiler",
        "TCB work-item producer contract drifted",
    )
    cases += 1

    requirements = copy_plan(baseline, root / "requirements")
    mutate_plan(
        requirements,
        lambda value: value["requirements"].__setitem__(
            "sha256", digest_bytes(b"hostile requirements")
        ),
    )
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        requirements,
        "tcb.compiler",
        "requirements identity drifted",
    )
    cases += 1

    validators = copy_plan(baseline, root / "validators")
    mutate_plan(
        validators,
        lambda value: value["trusted_validators"][0].__setitem__(
            "source_sha256", digest_bytes(b"hostile validator")
        ),
    )
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        validators,
        "tcb.compiler",
        "trusted-validator roster drifted",
    )
    cases += 1

    closure = copy_plan(baseline, root / "source-closure")
    with (closure / "source-closures/source.ferric.records").open("ab") as output:
        output.write(b"hostile\n")
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        closure,
        "tcb.compiler",
        "source closure bytes drifted",
    )
    cases += 1

    symlink = copy_plan(baseline, root / "artifact-symlink")
    target = root / "artifact-target"
    target.mkdir()
    (symlink / "artifacts").symlink_to(target, target_is_directory=True)
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        symlink,
        "tcb.compiler",
        "M1 artifact directory",
    )
    cases += 1

    permissions = copy_plan(baseline, root / "artifact-permissions")
    (permissions / "artifacts").mkdir(mode=0o755)
    (permissions / "artifacts").chmod(0o755)
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        permissions,
        "tcb.compiler",
        "owner-private 0700 directory",
    )
    cases += 1

    plan_permissions = copy_plan(baseline, root / "plan-permissions")
    plan_permissions.chmod(0o755)
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        plan_permissions,
        "tcb.compiler",
        "owner-private 0700 directory",
    )
    cases += 1

    wrong_subject = copy_plan(baseline, root / "wrong-subject")
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        wrong_subject,
        "tcb.other",
        "unknown M1 TCB subject",
    )
    cases += 1

    closure_output = copy_plan(baseline, root / "closure-output")
    (closure_output / "evidence-index.json").write_text("forbidden\n", encoding="ascii")
    expect_producer_failure(
        producer,
        ferric,
        fe2o3,
        closure_output,
        "tcb.compiler",
        "plan containing a closure output",
    )
    cases += 1
    return cases


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_OBJECT_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    fe2o3_source = Path(sys.argv[2]).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-tcb-producer-policy-") as raw:
        temporary = Path(raw)
        ferric = temporary / "ferric"
        clone_at(repo, ferric)
        shutil.copytree(
            repo / "proofs/m1-qualification",
            ferric / "proofs/m1-qualification",
            dirs_exist_ok=True,
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )
        status = command(
            [
                "git",
                "-C",
                str(ferric),
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
            ],
            "inspect Ferric TCB fixture",
        )
        if status:
            commit_fixture(ferric, "add M1 TCB-report producer")

        cargo = (ferric / "Cargo.toml").read_text(encoding="utf-8")
        marker = 'fe2o3-amdhsa-loader = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "'
        if cargo.count(marker) != 1:
            fail("cannot locate pinned fe2o3 revision in TCB producer fixture")
        fe2o3_commit = cargo.split(marker, 1)[1].split('"', 1)[0]
        fe2o3 = temporary / "fe2o3"
        clone_at(fe2o3_source, fe2o3, fe2o3_commit)

        planner = ferric / "proofs/m1-qualification/planner.py"
        producer = ferric / "proofs/m1-qualification/produce-tcb-report.py"
        baseline = temporary / "baseline-plan"
        run_planner(planner, ferric, fe2o3, baseline)

        first = copy_plan(baseline, temporary / "first")
        second = copy_plan(baseline, temporary / "second")
        for subject, _ in SUBJECTS:
            expect_producer_pass(producer, ferric, fe2o3, first, subject)
            expect_producer_pass(producer, ferric, fe2o3, second, subject)
            if (
                report_path(first, subject).read_bytes()
                != report_path(second, subject).read_bytes()
            ):
                fail(f"TCB report production is nondeterministic: {subject}")
        validate_reports(ferric, first)
        validate_reports(ferric, second)
        if any(
            (first / name).exists() for name in ("evidence-index.json", "receipt.json")
        ):
            fail("TCB producer emitted a forbidden closure output")

        expect_producer_failure(
            producer,
            ferric,
            fe2o3,
            first,
            "tcb.compiler",
            "without replacement",
        )
        hostile_count = exercise_hostile_inputs(
            temporary / "hostile", baseline, producer, ferric, fe2o3
        )
        hostile_count += exercise_publication_replacements(
            temporary / "hostile", baseline, producer
        )
        ferric_status = command(
            [
                "git",
                "-C",
                str(ferric),
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
            ],
            "recheck Ferric TCB fixture",
        )
        fe2o3_status = command(
            [
                "git",
                "-C",
                str(fe2o3),
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
            ],
            "recheck fe2o3 TCB fixture",
        )
        if ferric_status or fe2o3_status:
            fail("TCB production dirtied an exact source repository")
    print(
        "PASS: M1 TCB producer emitted 3 deterministic validator-accepted reports "
        f"and rejected {hostile_count + 1} hostile inputs"
    )


if __name__ == "__main__":
    main()
