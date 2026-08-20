#!/usr/bin/env python3
"""Authenticate the complete Verus Git revision selected by Cargo.lock."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path

COMMIT = "b677dd5a766f25f56e9aa1e32621aa4e53304b47"
SOURCE = f"git+https://github.com/verus-lang/verus.git?rev=b677dd5#{COMMIT}"
ROOT_PACKAGES = {
    "verus_builtin",
    "verus_builtin_macros",
    "verus_prettyplease",
    "verus_state_machines_macros",
    "verus_syn",
    "vstd",
}
SOURCE_GATE_PACKAGES = {"verus_syn"}
FE2O3_COMMIT = "a6fa86b5ccf8f0438925cfec8f48a5d713874da3"
FE2O3_SOURCE = (
    "git+https://github.com/harsh-nod/fe2o3.git?"
    f"rev={FE2O3_COMMIT}#{FE2O3_COMMIT}"
)
PROPERTY_BINDER_GIT_PACKAGES = {"fe2o3-proof-contracts": FE2O3_SOURCE}


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def check(path: Path, expected: set[str], label: str) -> None:
    try:
        lock = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot parse {path}: {error}")

    packages = lock.get("package")
    if not isinstance(packages, list):
        fail(f"{label} Cargo.lock has no package array")
    selected: dict[str, str] = {}
    for package in packages:
        if not isinstance(package, dict):
            fail(f"{label} Cargo.lock contains a malformed package")
        name = package.get("name")
        source = package.get("source")
        if isinstance(source, str) and source.startswith(
            "git+https://github.com/verus-lang/verus.git"
        ):
            if not isinstance(name, str):
                fail(f"{label} Verus package has no name")
            if name in selected:
                fail(f"duplicate {label} Verus package {name}")
            selected[name] = source

    if set(selected) != expected:
        missing = sorted(expected - set(selected))
        extra = sorted(set(selected) - expected)
        fail(f"{label} Verus package closure drifted (missing={missing}, extra={extra})")
    drifted = sorted(name for name, source in selected.items() if source != SOURCE)
    if drifted:
        fail(f"{label} Verus packages do not resolve the full admitted commit: {drifted}")
    print(f"PASS: {label} lock resolves {len(selected)} Verus packages at {COMMIT}")


def check_property_binder(path: Path) -> None:
    try:
        lock = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot parse {path}: {error}")
    packages = lock.get("package")
    if not isinstance(packages, list):
        fail("property-binder Cargo.lock has no package array")
    selected: dict[str, str] = {}
    for package in packages:
        if not isinstance(package, dict):
            fail("property-binder Cargo.lock contains a malformed package")
        name = package.get("name")
        source = package.get("source")
        if isinstance(source, str) and source.startswith("git+"):
            if not isinstance(name, str):
                fail("property-binder Git package has no name")
            if name in selected:
                fail(f"duplicate property-binder Git package {name}")
            selected[name] = source
    if selected != PROPERTY_BINDER_GIT_PACKAGES:
        fail(f"property-binder Git dependency closure drifted: {selected!r}")
    print(
        "PASS: property-binder lock resolves fe2o3-proof-contracts at "
        f"{FE2O3_COMMIT}"
    )


def main() -> None:
    if len(sys.argv) != 4:
        print(
            f"usage: {sys.argv[0]} ROOT_Cargo.lock SOURCE_GATE_Cargo.lock "
            "PROPERTY_BINDER_Cargo.lock",
            file=sys.stderr,
        )
        raise SystemExit(2)
    check(Path(sys.argv[1]), ROOT_PACKAGES, "runtime")
    check(Path(sys.argv[2]), SOURCE_GATE_PACKAGES, "source-gate")
    check_property_binder(Path(sys.argv[3]))


if __name__ == "__main__":
    main()
