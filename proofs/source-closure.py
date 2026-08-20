#!/usr/bin/env python3
"""Measure Ferric's proof, manifest, and executable source closure."""

from __future__ import annotations

import hashlib
import stat
import sys
from pathlib import Path


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def source_paths(repo: Path) -> list[Path]:
    paths = [
        repo / "Cargo.toml",
        repo / "Cargo.lock",
        repo / "rust-toolchain.toml",
        repo / ".github/workflows/verus.yml",
    ]
    for root in (repo / "crates", repo / "proofs", repo / "docs"):
        for path in root.rglob("*"):
            if path.is_symlink():
                fail(f"source closure contains a symlink: {path}")
            if path.is_dir():
                if path.name in {"__pycache__", "target"}:
                    fail(f"source closure contains a generated directory: {path}")
                continue
            if not path.is_file():
                fail(f"source closure contains a special entry: {path}")
            if path.suffix in {".pyc", ".receipt"}:
                fail(f"source closure contains a forbidden generated input: {path}")
            paths.append(path)
    paths = sorted(set(paths), key=lambda path: path.relative_to(repo).as_posix())
    if any(not path.is_file() for path in paths):
        fail("source closure contains a missing file")
    return paths


def main() -> None:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} REPO OUTPUT", file=sys.stderr)
        raise SystemExit(2)
    repo = Path(sys.argv[1]).resolve()
    output = Path(sys.argv[2])
    records = []
    try:
        for path in source_paths(repo):
            relative = path.relative_to(repo).as_posix()
            mode = stat.S_IMODE(path.stat().st_mode)
            records.append(f"{relative}|{mode:o}|{path.stat().st_size}|{digest(path)}")
        output.write_text("\n".join(records) + "\n", encoding="utf-8")
    except (OSError, UnicodeError, ValueError) as error:
        fail(str(error))
    print(f"PASS: measured Ferric source closure ({len(records)} files, sha256={digest(output)})")


if __name__ == "__main__":
    main()
