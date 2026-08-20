#!/usr/bin/env python3
"""Validate the negative-evidence registry and emit its active rows."""

from pathlib import Path
import re
import stat
import sys


FORMAT = "format=FERRIC-NEGATIVE-COMPONENTS-V2"
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]+\Z")
SAFE_TARGET = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*\Z")
VERUS_BODY = re.compile(r"verus\s*!\s*\{")


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"FAIL: {message}")


def regular_file(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {path}: {error}")
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        fail(f"{description} must be a regular file: {path}")


def safe_relative(value: str, description: str) -> Path:
    path = Path(value)
    if not value or path.is_absolute() or ".." in path.parts or "|" in value:
        fail(f"unsafe {description}: {value!r}")
    return path


def main() -> None:
    if len(sys.argv) != 4:
        fail("usage: check-registry.py REPO REGISTRY ACTIVE_OUTPUT")
    repo = Path(sys.argv[1]).resolve(strict=True)
    registry = Path(sys.argv[2])
    output = Path(sys.argv[3])
    regular_file(registry, "negative component registry")
    if output.exists():
        fail(f"active registry output already exists: {output}")

    lines = registry.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != FORMAT:
        fail("unsupported negative component registry")
    if len(lines) == 1:
        fail("negative component registry selected no mutations")

    active: list[tuple[str, str, str, str, str, str]] = []
    names: set[str] = set()
    for line in lines[1:]:
        if not line:
            fail("empty negative component registry record")
        if line.startswith("always="):
            fields = line.removeprefix("always=").split("|")
            if len(fields) != 6:
                fail(f"malformed negative component record: {line}")
            name, package, mutator, marker, module, function = fields
            enabled = True
        elif line.startswith("when-verus="):
            fields = line.removeprefix("when-verus=").split("|")
            if len(fields) != 7:
                fail(f"malformed conditional negative component: {line}")
            source, name, package, mutator, marker, module, function = fields
            source_path = repo / safe_relative(source, "conditional mutation source")
            regular_file(source_path, "conditional mutation source")
            enabled = VERUS_BODY.search(source_path.read_text(encoding="utf-8")) is not None
        else:
            fail(f"malformed negative component record: {line}")

        for value, description in (
            (name, "negative component name"),
            (package, "negative component package"),
            (mutator, "negative mutator name"),
        ):
            if SAFE_NAME.fullmatch(value) is None:
                fail(f"unsafe {description}: {value!r}")
        if marker not in {"proof", "no-cheating"}:
            fail(f"unknown negative failure marker: {marker}")
        if SAFE_TARGET.fullmatch(module) is None:
            fail(f"unsafe Verus module target: {module!r}")
        if SAFE_TARGET.fullmatch(function) is None:
            fail(f"unsafe Verus function target: {function!r}")
        if name in names:
            fail(f"duplicate negative component: {name}")
        names.add(name)

        mutator_path = repo / "proofs/negative/components" / safe_relative(
            mutator, "negative mutator path"
        )
        regular_file(mutator_path, "negative mutator")
        if enabled:
            active.append((name, package, mutator, marker, module, function))

    if not active:
        fail("negative component registry selected no mutations")
    active.sort()
    output.write_text(
        "".join("|".join(row) + "\n" for row in active), encoding="utf-8"
    )


if __name__ == "__main__":
    main()
