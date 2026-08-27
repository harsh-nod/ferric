#!/usr/bin/env python3
"""Require a negative mutator to change exactly its attested package source."""

from hashlib import sha256
from pathlib import Path
import stat
import sys


COPIED_ROOTS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "benches",
    "crates",
    "proofs/m1",
)


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"FAIL: {message}")


def snapshot(root: Path) -> dict[str, tuple[str, str]]:
    entries: dict[str, tuple[str, str]] = {}
    for relative_root in COPIED_ROOTS:
        path = root / relative_root
        if not path.exists() and not path.is_symlink():
            fail(f"mutation source closure is unavailable: {relative_root}")
        paths = [path]
        if path.is_dir():
            paths.extend(sorted(path.rglob("*")))
        for candidate in paths:
            relative = candidate.relative_to(root).as_posix()
            metadata = candidate.lstat()
            if stat.S_ISREG(metadata.st_mode):
                entries[relative] = ("file", sha256(candidate.read_bytes()).hexdigest())
            elif stat.S_ISDIR(metadata.st_mode):
                entries[relative] = ("directory", "")
            elif stat.S_ISLNK(metadata.st_mode):
                entries[relative] = ("symlink", candidate.readlink().as_posix())
            else:
                fail(f"unsupported mutation source entry: {relative}")
    return entries


def main() -> None:
    if len(sys.argv) != 5:
        fail("usage: check-mutation.py REPO MUTATED_COPY MARKER PACKAGE")
    repo = Path(sys.argv[1]).resolve(strict=True)
    mutated = Path(sys.argv[2]).resolve(strict=True)
    marker = Path(sys.argv[3])
    package = sys.argv[4]
    if package not in {"ferric-spec", "ferric-engine"}:
        fail(f"unsupported negative mutation package: {package}")

    lines = marker.read_text(encoding="utf-8").splitlines()
    source_lines = [line for line in lines if line.startswith("MUTATED_SOURCE=")]
    if len(source_lines) != 1:
        fail("mutator must emit exactly one MUTATED_SOURCE attestation")
    attested = source_lines[0].removeprefix("MUTATED_SOURCE=")
    attested_path = Path(attested)
    package_root = Path("crates") / package / "src"
    if (
        not attested
        or attested_path.is_absolute()
        or ".." in attested_path.parts
        or attested_path.suffix != ".rs"
        or not attested_path.is_relative_to(package_root)
    ):
        fail(f"mutator attested source outside {package}: {attested!r}")

    clean_entries = snapshot(repo)
    mutated_entries = snapshot(mutated)
    if clean_entries.get(attested_path.as_posix(), (None,))[0] != "file":
        fail(f"mutator attested source is not an existing clean Rust file: {attested}")
    changed = sorted(
        path
        for path in clean_entries.keys() | mutated_entries.keys()
        if clean_entries.get(path) != mutated_entries.get(path)
    )
    if changed != [attested_path.as_posix()]:
        rendered = ", ".join(changed) if changed else "none"
        fail(
            "mutator changed source outside its exact attestation: "
            f"attested={attested_path.as_posix()} changed={rendered}"
        )
    if mutated_entries.get(attested_path.as_posix(), (None,))[0] != "file":
        fail(f"mutator attested source is not a regular file: {attested}")


if __name__ == "__main__":
    main()
