#!/usr/bin/env python3
"""Exercise all 52 planner-bound M1 fe2o3-contract producers."""

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


PROTOCOL = "ferric.m1-validator.fe2o3-contract.v1"
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
            "user.name=M1 Fe2o3 Contract Policy",
            "-c",
            "user.email=m1-fe2o3-contract@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "add M1 fe2o3-contract producer",
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
            f"fe2o3-contract producer accepted hostile input; expected {expected!r}:\n"
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
        fail(f"planner rejected fe2o3-contract producer fixture:\n{result.stdout}")


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


def fe2o3_contract_slots(plan: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        slot
        for slot in plan["binding_slots"]
        if slot["binding"]["evidence_kind"] == "fe2o3-contract"
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
    for slot in fe2o3_contract_slots(plan):
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
        len(observed) != 52
        or digest_bytes(("\n".join(observed) + "\n").encode("ascii"))
        != "3a9caeaddd98840035fb55233aa1b3ccf53993313a955f95248a03f831cd45a9"
    ):
        fail("validated fe2o3-contract roster drifted")


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
    specification = importlib.util.spec_from_file_location(
        "fe2o3_contract_producer", path
    )
    if specification is None or specification.loader is None:
        fail("cannot load fe2o3-contract producer race policy")
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
        lambda value: set_binding_field(value, 64, "path_id", "device-cache"),
    )
    expect_failure(
        producer, ferric, fe2o3, wrong_path, binding_id, "exact rederivation"
    )
    cases += 1

    wrong_source = copy_plan(baseline, root / "wrong-source")
    mutate_plan(
        wrong_source,
        lambda value: set_binding_field(
            value, 64, "source_identity_id", "source.ferric"
        ),
    )
    expect_failure(
        producer, ferric, fe2o3, wrong_source, binding_id, "exact rederivation"
    )
    cases += 1

    queue_command = copy_plan(baseline, root / "queue-command")
    mutate_queue(
        queue_command,
        lambda value: value["items"][64]["producer"].__setitem__("command", None),
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
    expect_failure(producer, ferric, fe2o3, plan_link, binding_id, "Not a directory")
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
        for slot in fe2o3_contract_slots(read_json(baseline / "plan.json"))
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

    for directory, suffix in (
        ("contracts", "fe2o3-contract-body.json"),
        ("contract-sets", "fe2o3-contract-set.json"),
    ):
        hostile = copy_plan(baseline, root / f"preexisting-{directory}")
        (hostile / directory).mkdir(mode=0o700)
        (
            hostile / directory / f"{selected['binding']['artifact_id']}.{suffix}"
        ).write_text("hostile\n", encoding="ascii")
        expect_failure(
            producer, ferric, fe2o3, hostile, binding_id, "preexisting output"
        )
        cases += 1

    preexisting_symlink = copy_plan(baseline, root / "preexisting-symlink")
    target_report = root / "hostile-fe2o3-contract-report"
    target_report.write_bytes(b"hostile\n")
    (preexisting_symlink / selected["expected_artifact"]["path"]).symlink_to(
        target_report
    )
    expect_failure(
        producer, ferric, fe2o3, preexisting_symlink, binding_id, "preexisting output"
    )
    cases += 1

    dirty = ferric / "fe2o3-contract-hostile-untracked"
    dirty.write_bytes(b"hostile\n")
    try:
        expect_failure(
            producer, ferric, fe2o3, baseline, binding_id, "exact clean worktree"
        )
    finally:
        dirty.unlink()
    cases += 1

    dirty = fe2o3 / "fe2o3-contract-hostile-untracked"
    dirty.write_bytes(b"hostile\n")
    try:
        expect_failure(
            producer, ferric, fe2o3, baseline, binding_id, "exact clean worktree"
        )
    finally:
        dirty.unlink()
    cases += 1
    return cases


def three_file_publication_races(
    root: Path, baseline: Path, producer: Path, artifact_id: str
) -> int:
    module = load_producer(producer)
    cases = 0

    plan = copy_plan(baseline, root / "race-contract-parent")
    custody = module.authenticate_absolute_directory(plan, "race plan", private=True)
    plan_fd = module.directory_custody_fd(custody)
    artifact_fd = module.open_private_directory_at(
        plan_fd, "artifacts", "race artifacts"
    )
    body_fd, _ = module.ensure_private_child_directory(
        plan_fd, "contracts", "race bodies"
    )
    set_fd, _ = module.ensure_private_child_directory(
        plan_fd, "contract-sets", "race sets"
    )
    original = module.create_new_file_at
    rebound = False

    def rebind_parent(
        directory_fd: int, name: str, value: bytes, description: str
    ) -> int:
        nonlocal rebound
        if not rebound:
            rebound = True
            (plan / "contracts").rename(plan / "contracts-held")
            (plan / "contracts").mkdir(mode=0o700)
        return original(directory_fd, name, value, description)

    module.create_new_file_at = rebind_parent

    def custody_check() -> None:
        module.revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "race artifacts"
        )
        module.revalidate_child_directory(plan_fd, "contracts", body_fd, "race bodies")
        module.revalidate_child_directory(plan_fd, "contract-sets", set_fd, "race sets")
        module.revalidate_absolute_directory(custody, private=True)

    expect_direct_failure(
        lambda: module.publish_fe2o3_contract(
            custody,
            plan_fd,
            artifact_fd,
            body_fd,
            set_fd,
            artifact_id,
            b"body\n",
            b"set\n",
            b"report\n",
            custody_check,
        ),
        "race bodies was replaced after it was opened",
    )
    if list((plan / "contracts-held").iterdir()):
        fail("parent-rebinding failure left a false fe2o3 contract completion")
    if not (plan / "contracts").is_dir():
        fail("parent-rebinding rollback removed the rebound directory inode")
    module.create_new_file_at = original
    os.close(set_fd)
    os.close(body_fd)
    os.close(artifact_fd)
    module.close_absolute_directory(custody)
    cases += 1

    plan = copy_plan(baseline, root / "race-rebound-body")
    custody = module.authenticate_absolute_directory(
        plan, "rollback plan", private=True
    )
    plan_fd = module.directory_custody_fd(custody)
    artifact_fd = module.open_private_directory_at(
        plan_fd, "artifacts", "rollback artifacts"
    )
    body_fd, _ = module.ensure_private_child_directory(
        plan_fd, "contracts", "rollback bodies"
    )
    set_fd, _ = module.ensure_private_child_directory(
        plan_fd, "contract-sets", "rollback sets"
    )
    calls = 0
    attacker = b"attacker-owned-rebound\n"

    def rebind_body_then_fail(
        directory_fd: int, name: str, value: bytes, description: str
    ) -> int:
        nonlocal calls
        calls += 1
        if calls == 2:
            body_name = f"{artifact_id}.fe2o3-contract-body.json"
            os.rename(
                body_name, body_name + ".held", src_dir_fd=body_fd, dst_dir_fd=body_fd
            )
            descriptor = os.open(
                body_name,
                os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW,
                0o600,
                dir_fd=body_fd,
            )
            os.write(descriptor, attacker)
            os.close(descriptor)
            raise OSError("injected second-publication failure")
        return original(directory_fd, name, value, description)

    module.create_new_file_at = rebind_body_then_fail
    expect_direct_failure(
        lambda: module.publish_fe2o3_contract(
            custody,
            plan_fd,
            artifact_fd,
            body_fd,
            set_fd,
            artifact_id,
            b"body\n",
            b"set\n",
            b"report\n",
            lambda: None,
        ),
        "cannot remove replaced failed M1 fe2o3 contract body publication",
    )
    rebound_path = plan / "contracts" / f"{artifact_id}.fe2o3-contract-body.json"
    if rebound_path.read_bytes() != attacker:
        fail("rollback deleted or changed a rebound attacker inode")
    if (plan / "artifacts" / f"{artifact_id}.fe2o3-contract.json").exists():
        fail("rollback failure left a false report completion marker")
    module.create_new_file_at = original
    os.close(set_fd)
    os.close(body_fd)
    os.close(artifact_fd)
    module.close_absolute_directory(custody)
    cases += 1
    return cases


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_OBJECT_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    fe2o3_source = Path(sys.argv[2]).resolve(strict=True)
    producer_source = (
        repo / "proofs/m1-qualification/produce-fe2o3-contract.py"
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
        "proofs/m1/evidence/validate-fe2o3-contract.py",
        "ferric.m1-validator.fe2o3-contract.v1",
        "cargo ",
        "ContractSetV1::validate_closed()",
        "RATIONALES",
        "RATIONALE_KEYS",
        "nonclaim-only",
        "publish_rationale",
        "ferric-m1-rationale",
    )
    if any(token in producer_source for token in forbidden_copy_paths):
        fail("fe2o3-contract producer retains a stale copied publication path")
    with tempfile.TemporaryDirectory(
        prefix="ferric-m1-fe2o3-contract-producer-"
    ) as raw:
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
        producer = ferric / "proofs/m1-qualification/produce-fe2o3-contract.py"
        validator = ferric / "proofs/m1/evidence/validate-fe2o3-contract.py"
        baseline = root / "baseline"
        run_planner(planner, ferric, fe2o3, baseline)
        materialize_tcb(tcb_producer, ferric, fe2o3, baseline)
        plan = read_json(baseline / "plan.json")
        slots = fe2o3_contract_slots(plan)
        if len(slots) != 52:
            fail("planner did not expose exactly 52 fe2o3-contract bindings")

        deterministic_outputs = []
        first = slots[0]
        for name in ("determinism-a", "determinism-b"):
            candidate = copy_plan(baseline, root / name)
            result = invoke(producer, ferric, fe2o3, candidate, first["binding"]["id"])
            if result.returncode != 0:
                fail(f"producer rejected deterministic replay {name}:\n{result.stdout}")
            artifact_id = first["binding"]["artifact_id"]
            deterministic_outputs.append(
                (
                    (
                        candidate
                        / "contracts"
                        / f"{artifact_id}.fe2o3-contract-body.json"
                    ).read_bytes(),
                    (
                        candidate
                        / "contract-sets"
                        / f"{artifact_id}.fe2o3-contract-set.json"
                    ).read_bytes(),
                    (candidate / first["expected_artifact"]["path"]).read_bytes(),
                )
            )
        if deterministic_outputs[0] != deterministic_outputs[1]:
            fail("fe2o3-contract producer is not byte-deterministic")

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
        outputs = [
            path
            for directory in ("artifacts", "contracts", "contract-sets")
            for path in (canonical / directory).glob("*fe2o3-contract*.json")
        ]
        if len(outputs) != 156:
            fail(f"fe2o3-contract producer output count drifted: {len(outputs)}")
        if any(
            (canonical / name).exists()
            for name in ("evidence-index.json", "receipt.json")
        ):
            fail("fe2o3-contract producer emitted a forbidden closure output")

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
        hostile_count += three_file_publication_races(
            hostile_root, baseline, producer, slots[0]["binding"]["artifact_id"]
        )
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
            fail("fe2o3-contract production dirtied an exact source repository")
    print(
        "PASS: M1 fe2o3-contract producer emitted and validated all 52 bindings "
        f"and rejected {hostile_count} hostile inputs"
    )


if __name__ == "__main__":
    main()
