#!/usr/bin/env python3
"""Exercise all 74 planner-bound M1 artifact-identity producers."""

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


PROTOCOL = "ferric.m1-validator.artifact-identity.v1"
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
            "user.name=M1 Artifact Identity Policy",
            "-c",
            "user.email=m1-artifact-identity@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "add M1 artifact-identity producer",
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
            f"artifact producer accepted hostile input; expected {expected!r}:\n"
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
        fail(f"planner rejected artifact producer fixture:\n{result.stdout}")


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


def identity_slots(plan: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        slot
        for slot in plan["binding_slots"]
        if slot["binding"]["evidence_kind"] == "artifact-identity"
    ]


def end_to_end_slots(slots: list[dict[str, Any]]) -> list[dict[str, Any]]:
    uncovered = {
        *(f"class:{name}" for name in ("Assurance", "Roadmap")),
        *(f"source:{name}" for name in ("source.fe2o3", "source.ferric")),
        *(
            f"profile:{name}"
            for name in (
                "admission",
                "authentication",
                "composition",
                "kernel",
                "qualification",
                "runtime",
            )
        ),
    }
    selected = []
    remaining = slots.copy()
    while uncovered:
        ranked = []
        for slot in remaining:
            binding = slot["binding"]
            coverage = {
                f"class:{binding['obligation_class']}",
                f"source:{binding['source_identity_id']}",
                f"profile:{binding['profile_id']}",
            }
            ranked.append((len(coverage & uncovered), binding["id"], slot, coverage))
        ranked.sort(key=lambda item: (-item[0], item[1]))
        score, _, slot, coverage = ranked[0]
        if score == 0:
            fail(f"cannot cover canonical producer dimensions: {sorted(uncovered)}")
        selected.append(slot)
        remaining.remove(slot)
        uncovered -= coverage
    return selected


def materialize_remaining(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan_root: Path,
    completed: set[str],
) -> None:
    module = load_producer(producer)
    ferric_fd = module.open_directory(ferric, "policy Ferric repository")
    fe2o3_fd = module.open_directory(fe2o3, "policy fe2o3 repository")
    plan_fd = module.open_private_directory(plan_root, "policy M1 plan")
    artifact_fd = module.open_private_directory_at(
        plan_fd, "artifacts", "policy M1 artifact directory"
    )
    tcb_files: list[tuple[str, str, Any, os.stat_result, bytes]] = []
    try:
        requirements, plan, queue, sources, validators, _, _ = module.validate_plan(
            ferric, fe2o3, plan_fd
        )
        tcb, tcb_files = module.authenticate_tcb_reports(
            artifact_fd, ferric, requirements, sources, validators
        )
        repositories = {
            "source.fe2o3": (fe2o3, fe2o3_fd),
            "source.ferric": (ferric, ferric_fd),
        }
        for slot in identity_slots(plan):
            binding_id = slot["binding"]["id"]
            if binding_id in completed:
                continue
            selected, resolution = module.select_artifact_identity_binding(
                plan, queue, binding_id
            )
            source_path, source_fd = repositories[
                selected["binding"]["source_identity_id"]
            ]
            payload = module.read_source_file(
                source_path,
                source_fd,
                resolution["path"],
                f"policy selected source {binding_id}",
            )
            report = module.canonical_bytes(
                module.artifact_identity_report(
                    plan["requirements"]["sha256"],
                    requirements,
                    sources,
                    tcb,
                    selected,
                    resolution,
                    payload,
                )
            )
            module.publish_identity(
                plan_root,
                plan_fd,
                artifact_fd,
                selected["binding"]["artifact_id"],
                payload,
                report,
                lambda: None,
            )
    finally:
        for _, _, source, _, _ in tcb_files:
            source.close()
        os.close(artifact_fd)
        os.close(plan_fd)
        os.close(fe2o3_fd)
        os.close(ferric_fd)


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
    fe2o3: Path,
) -> None:
    resolutions = {row["id"]: row for row in plan["path_resolutions"]}
    tcb = outer_tcb(plan_root)
    repositories = {"source.fe2o3": fe2o3, "source.ferric": ferric}
    observed = []
    for slot in identity_slots(plan):
        binding = slot["binding"]
        artifact = slot["expected_artifact"]
        artifact_id = artifact["id"]
        report_path = plan_root / artifact["path"]
        payload_path = plan_root / "identified-artifacts" / f"{artifact_id}.bin"
        source_path = (
            repositories[binding["source_identity_id"]]
            / resolutions[binding["path_id"]]["path"]
        )
        if payload_path.read_bytes() != source_path.read_bytes():
            fail(f"producer did not copy exact selected source bytes: {binding['id']}")
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
        len(observed) != 74
        or digest_bytes(("\n".join(observed) + "\n").encode("ascii"))
        != "036a350d44c964bd96c44328087d541db7116452093ed9067987fa8497e57258"
    ):
        fail("validated artifact-identity roster drifted")


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


def load_producer(path: Path) -> ModuleType:
    sys.dont_write_bytecode = True
    specification = importlib.util.spec_from_file_location("artifact_producer", path)
    if specification is None or specification.loader is None:
        fail("cannot load artifact producer race policy")
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
    for race in ("plan", "artifact", "payload"):
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
                elif race == "artifact":
                    current = plan / "artifacts"
                    current.rename(plan / "artifacts-held")
                    current.mkdir(mode=0o700)
                else:
                    current = plan / "identified-artifacts"
                    current.rename(plan / "identified-artifacts-held")
                    current.mkdir(mode=0o700)
            return original(directory_fd, name, value, description)

        module.create_new_file_at = intercept
        expected = {
            "plan": "plan directory was replaced",
            "artifact": "artifact directory was replaced",
            "payload": "identified-artifact directory was replaced",
        }[race]
        try:
            expect_direct_failure(
                lambda: module.publish_identity(
                    plan,
                    plan_fd,
                    artifact_fd,
                    "artifact.binding.00000",
                    b"payload\n",
                    b"{}\n",
                    lambda: None,
                ),
                expected,
            )
        finally:
            module.create_new_file_at = original
            os.close(artifact_fd)
            os.close(plan_fd)
        report_name = "artifact.binding.00000.artifact-identity.json"
        candidates = [plan / "artifacts" / report_name]
        if race == "plan":
            candidates.append(
                plan.with_name(f"{plan.name}-held") / "artifacts" / report_name
            )
        elif race == "artifact":
            candidates.append(plan / "artifacts-held" / report_name)
        if any(path.exists() for path in candidates):
            fail(f"{race} replacement left a false artifact-identity completion marker")
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

        artifact_id = "artifact.binding.00000"
        report = plan / "artifacts" / f"{artifact_id}.artifact-identity.json"
        payload = plan / "identified-artifacts" / f"{artifact_id}.bin"
        try:
            expect_direct_failure(
                lambda: module.publish_identity(
                    plan,
                    plan_fd,
                    artifact_fd,
                    artifact_id,
                    b"payload\n",
                    b"{}\n",
                    reject_completion,
                ),
                f"injected {phase} input drift",
            )
            if report.exists() or not payload.is_file():
                fail(f"{phase} failure left a false report or lost durable payload")
            expect_direct_failure(
                lambda: module.publish_identity(
                    plan,
                    plan_fd,
                    artifact_fd,
                    artifact_id,
                    b"payload\n",
                    b"{}\n",
                    lambda: None,
                ),
                "preexisting output",
            )
            if report.exists():
                fail(f"{phase} retry created a false completion marker")
        finally:
            os.close(artifact_fd)
            os.close(plan_fd)
        cases += 2
    return cases


def plan_file_races(root: Path, baseline: Path, producer: Path) -> int:
    module = load_producer(producer)
    cases = 0
    for race, name, artifact_id in (
        ("replacement", "plan.json", "artifact.binding.00002"),
        ("in-place", "missing-work.json", "artifact.binding.00005"),
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

        report = plan / "artifacts" / f"{artifact_id}.artifact-identity.json"
        payload = plan / "identified-artifacts" / f"{artifact_id}.bin"
        try:
            expect_direct_failure(
                lambda: module.publish_identity(
                    plan,
                    plan_fd,
                    artifact_fd,
                    artifact_id,
                    b"payload\n",
                    b"{}\n",
                    race_plan_file,
                ),
                f"race {name} changed after authentication",
            )
            if report.exists() or not payload.is_file():
                fail(f"{race} {name} race left a false completion marker")
        finally:
            held[1].close()
            os.close(artifact_fd)
            os.close(plan_fd)
        cases += 1
    return cases


def published_byte_races(root: Path, baseline: Path, producer: Path) -> int:
    module = load_producer(producer)
    cases = 0
    for target_kind, artifact_id in (
        ("payload", "artifact.binding.00007"),
        ("report", "artifact.binding.00009"),
    ):
        plan = copy_plan(baseline, root / f"race-published-{target_kind}")
        plan_fd = module.open_private_directory(plan, f"{target_kind} byte-race plan")
        artifact_fd = module.open_private_directory_at(
            plan_fd, "artifacts", f"{target_kind} byte-race artifact directory"
        )
        calls = 0

        def mutate_after_readback() -> None:
            nonlocal calls
            calls += 1
            if calls != 2:
                return
            if target_kind == "payload":
                target = plan / "identified-artifacts" / f"{artifact_id}.bin"
            else:
                target = plan / "artifacts" / f"{artifact_id}.artifact-identity.json"
            raw = target.read_bytes()
            hostile = bytes([raw[0] ^ 1]) + raw[1:]
            with target.open("r+b") as output:
                output.write(hostile)
                output.flush()
                os.fsync(output.fileno())

        report = plan / "artifacts" / f"{artifact_id}.artifact-identity.json"
        payload = plan / "identified-artifacts" / f"{artifact_id}.bin"
        description = (
            "M1 identified artifact payload"
            if target_kind == "payload"
            else "M1 artifact-identity report"
        )
        try:
            expect_direct_failure(
                lambda: module.publish_identity(
                    plan,
                    plan_fd,
                    artifact_fd,
                    artifact_id,
                    b"payload\n",
                    b"{}\n",
                    mutate_after_readback,
                ),
                f"published {description} bytes or binding changed",
            )
            if report.exists() or not payload.is_file():
                fail(
                    f"same-size {target_kind} overwrite left a false completion marker"
                )
        finally:
            os.close(artifact_fd)
            os.close(plan_fd)
        cases += 1
    return cases


def source_races(root: Path, producer: Path) -> int:
    module = load_producer(producer)
    source = root / "source-race"
    source.mkdir(mode=0o700)
    directory = source / "dir"
    directory.mkdir()
    (directory / "file.bin").write_bytes(b"original\n")
    source_fd = module.open_directory(source, "race source")
    original = module.open_regular_at
    replaced = False

    def intercept(
        directory_fd: int, name: str, description: str, *, writable: bool = False
    ) -> tuple[Any, os.stat_result]:
        nonlocal replaced
        opened = original(directory_fd, name, description, writable=writable)
        if not replaced:
            replaced = True
            directory.rename(source / "dir-held")
            directory.mkdir()
            (directory / "file.bin").write_bytes(b"replacement\n")
        return opened

    module.open_regular_at = intercept
    try:
        expect_direct_failure(
            lambda: module.read_source_file(
                source, source_fd, "dir/file.bin", "race source file"
            ),
            "source file directory was replaced",
        )
    finally:
        module.open_regular_at = original
        os.close(source_fd)

    file_source = root / "source-file-race"
    file_source.mkdir(mode=0o700)
    selected_file = file_source / "file.bin"
    selected_file.write_bytes(b"original\n")
    file_source_fd = module.open_directory(file_source, "race file source")
    original = module.open_regular_at
    replaced = False

    def replace_file(
        directory_fd: int, name: str, description: str, *, writable: bool = False
    ) -> tuple[Any, os.stat_result]:
        nonlocal replaced
        opened = original(directory_fd, name, description, writable=writable)
        if not replaced:
            replaced = True
            selected_file.rename(file_source / "file-held.bin")
            selected_file.write_bytes(b"replacement\n")
        return opened

    module.open_regular_at = replace_file
    try:
        expect_direct_failure(
            lambda: module.read_source_file(
                file_source, file_source_fd, "file.bin", "race selected file"
            ),
            "race selected file changed",
        )
    finally:
        module.open_regular_at = original
        os.close(file_source_fd)

    symlink_source = root / "source-symlink"
    symlink_source.mkdir(mode=0o700)
    (symlink_source / "target.bin").write_bytes(b"target\n")
    (symlink_source / "link.bin").symlink_to("target.bin")
    symlink_fd = module.open_directory(symlink_source, "symlink source")
    try:
        expect_direct_failure(
            lambda: module.read_source_file(
                symlink_source, symlink_fd, "link.bin", "symlink source file"
            ),
            "symlink source file is unavailable",
        )
    finally:
        os.close(symlink_fd)
    return 3


def tcb_replacement_race(
    root: Path, baseline: Path, producer: Path, ferric: Path
) -> int:
    module = load_producer(producer)
    plan_root = copy_plan(baseline, root / "race-tcb-report")
    plan = read_json(plan_root / "plan.json")
    requirements = read_json(ferric / "proofs/M1_REQUIREMENTS.json")
    validators = module.trusted_validators(ferric)[1]
    plan_fd = module.open_private_directory(plan_root, "race TCB plan")
    artifact_fd = module.open_private_directory_at(
        plan_fd, "artifacts", "race TCB artifact directory"
    )
    held: list[tuple[str, str, Any, os.stat_result, bytes]] = []
    try:
        _, held = module.authenticate_tcb_reports(
            artifact_fd, ferric, requirements, plan["sources"], validators
        )
        report = plan_root / "artifacts/artifact.tcb.compiler.tcb-report.json"
        raw = report.read_bytes()
        report.rename(report.with_suffix(".held"))
        report.write_bytes(raw)
        report.chmod(0o600)
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
    return 1


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

    output_permissions = copy_plan(baseline, root / "output-permissions")
    (output_permissions / "identified-artifacts").mkdir(mode=0o755)
    (output_permissions / "identified-artifacts").chmod(0o755)
    expect_failure(
        producer, ferric, fe2o3, output_permissions, binding_id, "owner-private 0700"
    )
    cases += 1

    output_symlink = copy_plan(baseline, root / "output-symlink")
    target_dir = root / "output-symlink-target"
    target_dir.mkdir(mode=0o700)
    (output_symlink / "identified-artifacts").symlink_to(target_dir)
    expect_failure(
        producer,
        ferric,
        fe2o3,
        output_symlink,
        binding_id,
        "identified-artifact directory",
    )
    cases += 1

    closure = copy_plan(baseline, root / "closure-output")
    (closure / "receipt.json").write_text("hostile\n", encoding="ascii")
    expect_failure(producer, ferric, fe2o3, closure, binding_id, "closure output")
    cases += 1

    selected = next(
        slot
        for slot in identity_slots(read_json(baseline / "plan.json"))
        if slot["binding"]["id"] == binding_id
    )
    artifact_id = selected["binding"]["artifact_id"]
    preexisting_report = copy_plan(baseline, root / "preexisting-report")
    (preexisting_report / selected["expected_artifact"]["path"]).write_text(
        "hostile\n", encoding="ascii"
    )
    expect_failure(
        producer, ferric, fe2o3, preexisting_report, binding_id, "preexisting output"
    )
    cases += 1

    partial = copy_plan(baseline, root / "partial-payload")
    payload_dir = partial / "identified-artifacts"
    payload_dir.mkdir(mode=0o700)
    (payload_dir / f"{artifact_id}.bin").write_bytes(b"hostile\n")
    expect_failure(producer, ferric, fe2o3, partial, binding_id, "preexisting output")
    cases += 1
    return cases


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_OBJECT_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    fe2o3_source = Path(sys.argv[2]).resolve(strict=True)
    producer_source = (
        repo / "proofs/m1-qualification/produce-artifact-identity.py"
    ).read_text(encoding="ascii")
    forbidden_copy_paths = (
        "def create_new_report_at(",
        "def publish_report(",
        "produced M1 TCB report",
        "TCB producer created a forbidden closure output",
        "tcb.compiler|tcb.hardware|tcb.runtime",
    )
    if any(token in producer_source for token in forbidden_copy_paths):
        fail("artifact producer retains a stale TCB publication path")
    with tempfile.TemporaryDirectory(prefix="ferric-m1-artifact-producer-") as raw:
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
        producer = ferric / "proofs/m1-qualification/produce-artifact-identity.py"
        validator = ferric / "proofs/m1/evidence/validate-artifact-identity.py"
        baseline = root / "baseline"
        run_planner(planner, ferric, fe2o3, baseline)
        materialize_tcb(tcb_producer, ferric, fe2o3, baseline)
        plan = read_json(baseline / "plan.json")
        slots = identity_slots(plan)
        if len(slots) != 74:
            fail("planner did not expose exactly 74 artifact-identity bindings")

        canonical = copy_plan(baseline, root / "canonical")
        e2e = end_to_end_slots(slots)
        for ordinal, slot in enumerate(e2e, 1):
            binding_id = slot["binding"]["id"]
            result = invoke(producer, ferric, fe2o3, canonical, binding_id)
            if result.returncode != 0:
                fail(
                    f"producer rejected canonical binding {ordinal}/{len(e2e)} "
                    f"{binding_id}:\n"
                    f"{result.stdout}"
                )
        materialize_remaining(
            producer,
            ferric,
            fe2o3,
            canonical,
            {slot["binding"]["id"] for slot in e2e},
        )
        validate_all_reports(validator, canonical, plan, ferric, fe2o3)
        if any(
            (canonical / name).exists()
            for name in ("evidence-index.json", "receipt.json")
        ):
            fail("artifact producer emitted a forbidden closure output")

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
        hostile_count += published_byte_races(hostile_root, baseline, producer)
        hostile_count += source_races(hostile_root, producer)
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
            fail("artifact production dirtied an exact source repository")
    print(
        "PASS: M1 artifact-identity producer emitted and validated all 74 bindings "
        f"and rejected {hostile_count} hostile inputs"
    )


if __name__ == "__main__":
    main()
