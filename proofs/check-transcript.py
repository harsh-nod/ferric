#!/usr/bin/env python3
"""Require a nonzero, error-free Verus result for every opted-in crate."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    if len(sys.argv) != 4:
        print(f"usage: {sys.argv[0]} METADATA TRANSCRIPT COUNTS_OUT", file=sys.stderr)
        raise SystemExit(2)
    metadata_path, transcript_path, counts_path = map(Path, sys.argv[1:])
    try:
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        transcript = transcript_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        fail(str(error))

    workspace_ids = set(metadata.get("workspace_members", []))
    opted = sorted(
        package["name"]
        for package in metadata.get("packages", [])
        if package.get("id") in workspace_ids
        and package.get("metadata", {}).get("verus", {}).get("verify") is True
    )
    if not opted:
        fail("Cargo metadata contains no opted-in first-party crate")

    compile_pattern = re.compile(r"^\s*Compiling ([A-Za-z0-9_-]+) v[^ ]+", re.MULTILINE)
    result_pattern = re.compile(r"^verification results:: (\d+) verified, (\d+) errors$", re.MULTILINE)
    events = sorted(
        [(match.start(), "compile", match.group(1), None) for match in compile_pattern.finditer(transcript)]
        + [
            (match.start(), "result", match.group(1), match.group(2))
            for match in result_pattern.finditer(transcript)
        ]
    )
    current: str | None = None
    counts: dict[str, tuple[int, int]] = {}
    for _, event, first, second in events:
        if event == "compile":
            current = first
        elif current in opted:
            if current in counts:
                fail(f"multiple verification summaries for {current}")
            counts[current] = (int(first), int(second or "0"))

    if set(counts) != set(opted):
        fail(f"missing first-party proof results: {sorted(set(opted) - set(counts))}")
    for package, (verified, errors) in counts.items():
        if verified <= 0 or errors != 0:
            fail(f"{package} proof result is not admissible: verified={verified}, errors={errors}")
    try:
        counts_path.write_text(
            "".join(f"{package}|{counts[package][0]}|{counts[package][1]}\n" for package in opted),
            encoding="utf-8",
        )
    except OSError as error:
        fail(str(error))
    total = sum(verified for verified, _ in counts.values())
    print(f"PASS: direct first-party proof results matched ({total} verified, crates={','.join(opted)})")


if __name__ == "__main__":
    main()
