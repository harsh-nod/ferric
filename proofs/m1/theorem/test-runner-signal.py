#!/usr/bin/env python3
"""Verify that runner termination reaps its active Cargo process group."""

from __future__ import annotations

import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
from typing import NoReturn


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def process_group_exists(pgid: int) -> bool:
    try:
        os.killpg(pgid, 0)
    except ProcessLookupError:
        return False
    except PermissionError as error:
        fail(f"cannot audit fake Cargo process group {pgid}: {error}")
    return True


def terminate_group(pgid: int) -> None:
    try:
        os.killpg(pgid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} REPO VERUS_ROOT")
    repo = Path(sys.argv[1]).resolve(strict=True)
    verus_root = Path(sys.argv[2]).resolve(strict=True)
    runner = repo / "proofs/m1/theorem/run-same-source.sh"
    if subprocess.run(
        ["git", "-C", str(repo), "status", "--porcelain=v1", "--untracked-files=all"],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout:
        fail("runner signal audit requires a clean source worktree")

    with tempfile.TemporaryDirectory(prefix="ferric-m1-theorem-signal.") as raw:
        root = Path(raw)
        fake_bin = root / "bin"
        fake_bin.mkdir()
        marker = root / "fake-cargo-processes"
        fake_cargo = fake_bin / "cargo"
        fake_cargo.write_text(
            "#!/bin/sh\n"
            "set -eu\n"
            ': "${FERRIC_M1_SIGNAL_MARKER:?}"\n'
            "sleep 600 &\n"
            "sleep_pid=$!\n"
            "pgid=$(ps -o pgid= -p $$ | tr -d ' ')\n"
            'printf \'%s|%s|%s\\n\' "$$" "$sleep_pid" "$pgid" '
            '>"$FERRIC_M1_SIGNAL_MARKER"\n'
            'wait "$sleep_pid"\n',
            encoding="ascii",
        )
        fake_cargo.chmod(0o755)
        output = root / "result"
        temporary = root / "tmp"
        temporary.mkdir()
        environment = os.environ.copy()
        environment.update(
            {
                "FERRIC_M1_SIGNAL_MARKER": str(marker),
                "FERRIC_M1_THEOREM_TIMEOUT_SECONDS": "1200",
                "PATH": f"{fake_bin}:{environment['PATH']}",
                "TMPDIR": str(temporary),
            }
        )
        process = subprocess.Popen(
            [
                str(runner),
                str(repo),
                str(verus_root),
                str(output),
                "batching-publish-once",
            ],
            cwd=repo,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        child_pgid: int | None = None
        try:
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline and not marker.exists():
                if process.poll() is not None:
                    output_text = (
                        process.stdout.read() if process.stdout else b""
                    ).decode(errors="replace")
                    compile_path = output / "ferric-spec.compile.transcript"
                    compile_text = (
                        compile_path.read_text(encoding="utf-8", errors="replace")
                        if compile_path.exists()
                        else "<compile transcript absent>"
                    )
                    fail(
                        "runner exited before fake Cargo started:\n"
                        f"{output_text}\n{compile_text}"
                    )
                time.sleep(0.05)
            if not marker.exists():
                fail("runner did not start fake Cargo within 60 seconds")
            fields = marker.read_text(encoding="ascii").strip().split("|")
            if len(fields) != 3 or any(not value.isdecimal() for value in fields):
                fail("fake Cargo process marker is malformed")
            cargo_pid, sleep_pid, child_pgid = map(int, fields)
            if child_pgid in (os.getpgrp(), process.pid) or not process_group_exists(
                child_pgid
            ):
                fail("runner did not isolate fake Cargo in a live child process group")

            process.terminate()
            try:
                status = process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                fail("runner did not terminate after SIGTERM")
            if status != 143:
                output_text = (process.stdout.read() if process.stdout else b"").decode(
                    errors="replace"
                )
                fail(
                    f"runner returned {status}, expected 143 after SIGTERM:\n{output_text}"
                )
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline and process_group_exists(child_pgid):
                time.sleep(0.05)
            if process_group_exists(child_pgid):
                fail(
                    "runner left its fake Cargo process group alive "
                    f"(cargo={cargo_pid}, sleep={sleep_pid}, pgid={child_pgid})"
                )
            if list(temporary.glob("ferric-m1-theorem.*")):
                fail("runner left its theorem scratch directory after SIGTERM")
        finally:
            if process.poll() is None:
                terminate_group(process.pid)
                process.wait(timeout=5)
            if child_pgid is not None:
                terminate_group(child_pgid)

    print("PASS: M1 theorem runner reaped its active process group on SIGTERM")


if __name__ == "__main__":
    main()
