#!/usr/bin/env python3
"""Authenticate the complete Verus Git revision selected by Cargo.lock."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

COMMIT = "b677dd5a766f25f56e9aa1e32621aa4e53304b47"
SOURCE = f"git+https://github.com/verus-lang/verus.git?rev=b677dd5#{COMMIT}"
PACKAGES = {
    "verus_builtin",
    "verus_builtin_macros",
    "verus_prettyplease",
    "verus_state_machines_macros",
    "verus_syn",
    "vstd",
}


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} Cargo.lock", file=sys.stderr)
        raise SystemExit(2)
    path = Path(sys.argv[1])
    try:
        lock = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot parse {path}: {error}")

    packages = lock.get("package")
    if not isinstance(packages, list):
        fail("Cargo.lock has no package array")
    selected: dict[str, str] = {}
    for package in packages:
        if not isinstance(package, dict):
            fail("Cargo.lock contains a malformed package")
        name = package.get("name")
        source = package.get("source")
        if isinstance(source, str) and source.startswith(
            "git+https://github.com/verus-lang/verus.git"
        ):
            if not isinstance(name, str):
                fail("Verus package has no name")
            if name in selected:
                fail(f"duplicate Verus package {name}")
            selected[name] = source

    if set(selected) != PACKAGES:
        missing = sorted(PACKAGES - set(selected))
        extra = sorted(set(selected) - PACKAGES)
        fail(f"Verus package closure drifted (missing={missing}, extra={extra})")
    drifted = sorted(name for name, source in selected.items() if source != SOURCE)
    if drifted:
        fail(f"Verus packages do not resolve the full admitted commit: {drifted}")
    print(f"PASS: Cargo.lock resolves {len(selected)} Verus packages at {COMMIT}")


if __name__ == "__main__":
    main()
