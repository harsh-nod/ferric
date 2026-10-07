#!/usr/bin/env python3
"""Test canonical Git modes across every full M1 source-closure writer."""

from __future__ import annotations

import contextlib
import hashlib
import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


ROOT = Path(__file__).resolve().parents[3]
MEASURE = ROOT / "proofs/m1/evidence/measure-source-closure.py"
WRITERS = {
    "evidence-index": ROOT / "proofs/check-m1-evidence-index.py",
    "negative-mutation": (
        ROOT / "proofs/m1/evidence/validate-negative-mutation.py"
    ),
    "qualification-receipt": (
        ROOT / "proofs/m1/evidence/validate-qualification-receipt.py"
    ),
    "evidence-index-policy": ROOT / "proofs/m1-evidence-index/test-policy.py",
}


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def git(repo: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        fail(f"git {' '.join(arguments)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        fail(f"cannot load source-closure writer: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write(path: Path, content: str, mode: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="ascii")
    path.chmod(mode)


def create_repository(repo: Path) -> None:
    repo.mkdir()
    git(repo, "init", "-q")
    git(repo, "config", "user.email", "m1-source-closure@example.invalid")
    git(repo, "config", "user.name", "M1 source-closure policy")
    write(repo / ".gitignore", "ignored.tmp\n", 0o644)
    write(repo / "plain.txt", "plain\n", 0o644)
    write(repo / "script.sh", "#!/bin/sh\nexit 0\n", 0o755)
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "source-closure fixture")


def clone_repository(source: Path, destination: Path) -> None:
    result = subprocess.run(
        ["git", "clone", "-q", "--no-hardlinks", str(source), str(destination)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        fail(f"cannot clone source-closure fixture: {result.stderr.strip()}")


def measure_script(repo: Path, output: Path) -> bytes:
    result = subprocess.run(
        [sys.executable, "-I", "-B", str(MEASURE), str(repo), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if result.returncode != 0:
        fail(f"canonical source-closure producer failed:\n{result.stdout}")
    return output.read_bytes()


def require_measure_failure(repo: Path, output: Path, marker: str) -> None:
    result = subprocess.run(
        [sys.executable, "-I", "-B", str(MEASURE), str(repo), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if result.returncode == 0 or marker not in result.stdout:
        fail(f"source-closure producer accepted hostile input:\n{result.stdout}")


def require_failure(action: Callable[[], Any], marker: str) -> None:
    captured = io.StringIO()
    try:
        with contextlib.redirect_stdout(captured), contextlib.redirect_stderr(captured):
            action()
    except SystemExit as error:
        if error.code == 0 or marker not in captured.getvalue():
            fail(f"unexpected source-closure rejection:\n{captured.getvalue()}")
        return
    fail(f"source-closure authority accepted hostile input requiring {marker!r}")


def require_runtime_failure(action: Callable[[], Any], marker: str) -> None:
    try:
        action()
    except RuntimeError as error:
        if marker not in str(error):
            fail(f"unexpected source-closure rejection:\n{error}")
        return
    fail(f"source-closure authority accepted hostile input requiring {marker!r}")


def all_closures(
    repo: Path, output: Path, modules: dict[str, Any]
) -> dict[str, bytes]:
    values = {"producer": measure_script(repo, output)}
    for name in ("evidence-index", "negative-mutation", "qualification-receipt"):
        values[name] = modules[name].source_closure(repo)[0]
    values["evidence-index-policy"] = modules["evidence-index-policy"].source_closure(
        repo
    )
    if len(set(values.values())) != 1:
        identities = {
            name: hashlib.sha256(value).hexdigest() for name, value in values.items()
        }
        fail(f"M1 source-closure writers disagree: {identities}")
    return values


def require_clean(repo: Path, description: str) -> None:
    if git(repo, "status", "--porcelain=v1", "--untracked-files=all"):
        fail(f"{description} is not a clean Git checkout")


def make_group_writable(repo: Path) -> None:
    for path in repo.rglob("*"):
        if ".git" in path.relative_to(repo).parts:
            continue
        if path.is_dir():
            path.chmod(0o775)
        elif path.is_file():
            owner_executable = bool(path.stat().st_mode & 0o100)
            path.chmod(0o775 if owner_executable else 0o664)


def main() -> None:
    modules = {
        name: load_module(f"ferric_m1_source_closure_{name.replace('-', '_')}", path)
        for name, path in WRITERS.items()
    }
    with tempfile.TemporaryDirectory(prefix="ferric-m1-source-closure-mode.") as raw:
        scratch = Path(raw)
        source = scratch / "source"
        create_repository(source)

        baseline_repo = scratch / "baseline"
        shared_repo = scratch / "shared"
        group_exec_repo = scratch / "group-exec"
        hidden_drift_repo = scratch / "hidden-drift"
        drift_repo = scratch / "drift"
        ignored_repo = scratch / "ignored"
        for destination in (
            baseline_repo,
            shared_repo,
            group_exec_repo,
            hidden_drift_repo,
            drift_repo,
            ignored_repo,
        ):
            clone_repository(source, destination)

        baseline = all_closures(
            baseline_repo, scratch / "baseline.records", modules
        )["producer"]
        rows = baseline.decode("ascii").splitlines()
        if not any(row.startswith("plain.txt|644|") for row in rows):
            fail("M1 source closure did not encode a regular file as 0644")
        if not any(row.startswith("script.sh|755|") for row in rows):
            fail("M1 source closure did not retain owner-executable mode 0755")

        make_group_writable(shared_repo)
        require_clean(shared_repo, "group-writable clone")
        shared = all_closures(shared_repo, scratch / "shared.records", modules)[
            "producer"
        ]
        if shared != baseline:
            fail("M1 evidence changed between 0644/0755 and 0664/0775 clones")

        (group_exec_repo / "plain.txt").chmod(0o654)
        require_clean(group_exec_repo, "group-executable clone")
        group_exec = all_closures(
            group_exec_repo, scratch / "group-exec.records", modules
        )["producer"]
        if group_exec != baseline:
            fail("M1 evidence retained a group-only executable bit")

        git(hidden_drift_repo, "config", "core.fileMode", "false")
        (hidden_drift_repo / "plain.txt").chmod(0o755)
        require_clean(hidden_drift_repo, "core.fileMode=false drift clone")
        hidden_drift = all_closures(
            hidden_drift_repo, scratch / "hidden-drift.records", modules
        )["producer"]
        if hidden_drift != baseline:
            fail("M1 evidence encoded owner-executable drift hidden by Git")

        git(hidden_drift_repo, "update-index", "--chmod=+x", "plain.txt")
        if not git(
            hidden_drift_repo, "status", "--porcelain=v1", "--untracked-files=all"
        ):
            fail("committed Git mode transition was not staged")
        git(
            hidden_drift_repo,
            "-c",
            "user.email=m1-source-closure@example.invalid",
            "-c",
            "user.name=M1 source-closure policy",
            "commit",
            "-q",
            "-m",
            "commit owner executable mode with fileMode disabled",
        )
        require_clean(hidden_drift_repo, "committed core.fileMode=false clone")
        committed_hidden_drift = all_closures(
            hidden_drift_repo,
            scratch / "committed-hidden-drift.records",
            modules,
        )["producer"]
        if (
            committed_hidden_drift == baseline
            or b"plain.txt|755|" not in committed_hidden_drift
        ):
            fail("M1 evidence ignored a committed Git owner-executable mode")

        (drift_repo / "plain.txt").chmod(0o755)
        if not git(drift_repo, "status", "--porcelain=v1", "--untracked-files=all"):
            fail("owner-executable drift did not dirty the Git checkout")
        require_measure_failure(
            drift_repo, scratch / "dirty-drift.records", "requires a clean worktree"
        )
        require_failure(
            lambda: modules["evidence-index"].git_identity(drift_repo),
            "source repository is not the exact clean Git tree",
        )
        require_failure(
            lambda: modules["negative-mutation"].qualified_source_identity(
                drift_repo
            ),
            "qualified Ferric source worktree is not clean",
        )
        require_failure(
            lambda: modules["qualification-receipt"].git_identity(drift_repo),
            "source repository is not the exact clean Git tree",
        )
        git(drift_repo, "add", "plain.txt")
        git(
            drift_repo,
            "-c",
            "user.email=m1-source-closure@example.invalid",
            "-c",
            "user.name=M1 source-closure policy",
            "commit",
            "-q",
            "-m",
            "retain owner executable mode",
        )
        committed_drift = all_closures(
            drift_repo, scratch / "committed-drift.records", modules
        )["producer"]
        if committed_drift == baseline or b"plain.txt|755|" not in committed_drift:
            fail("M1 source closure did not retain committed owner-executable drift")

        write(ignored_repo / "ignored.tmp", "ignored\n", 0o644)
        require_clean(ignored_repo, "clone with ignored untracked content")
        require_measure_failure(
            ignored_repo,
            scratch / "ignored.records",
            "source closure is not the exact committed tree",
        )
        require_failure(
            lambda: modules["evidence-index"].source_closure(ignored_repo),
            "source closure is not the exact committed tree",
        )
        require_failure(
            lambda: modules["negative-mutation"].qualified_source_identity(
                ignored_repo
            ),
            "source closure is not the exact committed tree",
        )
        require_failure(
            lambda: modules["qualification-receipt"].source_closure(ignored_repo),
            "source closure is not the exact committed tree",
        )
        require_runtime_failure(
            lambda: modules["evidence-index-policy"].source_closure(ignored_repo),
            "source closure is not the exact committed tree",
        )

    print("PASS: all M1 source-closure writers use canonical Git permission modes")


if __name__ == "__main__":
    main()
