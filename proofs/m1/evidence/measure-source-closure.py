#!/usr/bin/env python3
"""Measure the exact committed source closure used by the M1 evidence checker."""

from __future__ import annotations

import hashlib
from pathlib import Path
import stat
import subprocess
import sys
from typing import NoReturn


EXCLUDED_DIRECTORIES = {".git", ".ruff_cache", "__pycache__", "target"}
EXCLUDED_SUFFIXES = {".pyc", ".receipt"}


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def git(repo: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        fail(f"Git source-closure query failed: {result.stderr.strip()}")
    return result.stdout


def included(name: str) -> bool:
    path = Path(name)
    return not any(part in EXCLUDED_DIRECTORIES for part in path.parts) and (
        path.suffix not in EXCLUDED_SUFFIXES
    )


def main() -> None:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} REPO OUTPUT", file=sys.stderr)
        raise SystemExit(2)
    repo = Path(sys.argv[1]).resolve(strict=True)
    output = Path(sys.argv[2])
    if git(repo, "status", "--porcelain=v1", "--untracked-files=all"):
        fail("M1 source closure requires a clean worktree")

    records: list[str] = []
    members: set[str] = set()
    try:
        candidates = sorted(
            repo.rglob("*"), key=lambda path: path.relative_to(repo).as_posix()
        )
        for path in candidates:
            relative = path.relative_to(repo)
            name = relative.as_posix()
            if not included(name):
                continue
            if path.is_symlink():
                fail(f"M1 source closure contains a symlink: {name}")
            if path.is_dir():
                continue
            if not path.is_file():
                fail(f"M1 source closure contains a special entry: {name}")
            metadata = path.stat()
            mode = stat.S_IMODE(metadata.st_mode)
            records.append(f"{name}|{mode:o}|{metadata.st_size}|{digest(path)}")
            members.add(name)
    except (OSError, ValueError) as error:
        fail(f"cannot measure M1 source closure: {error}")
    tracked = {
        name
        for name in git(repo, "ls-tree", "-r", "--name-only", "HEAD").splitlines()
        if included(name)
    }
    if not records or members != tracked:
        fail("M1 source closure is not the exact committed tree")
    try:
        output.write_text("\n".join(records) + "\n", encoding="utf-8")
    except OSError as error:
        fail(f"cannot write M1 source closure: {error}")
    print(
        f"PASS: measured exact M1 source closure "
        f"({len(records)} files, sha256={digest(output)})"
    )


if __name__ == "__main__":
    main()
