#!/usr/bin/env python3
"""Bind one structured Verus root result to admitted executable bodies."""

from __future__ import annotations

import json
import sys
from pathlib import Path


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def manifest_functions(path: Path, package: str) -> set[str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "format=FERRIC-VERIFIED-MODULES-V2":
        fail("unsupported compiler-rooted coverage manifest")
    expected = set()
    for line in lines[1:]:
        if line.startswith("verified="):
            fields = line.removeprefix("verified=").split("|")
            if len(fields) != 3:
                fail("malformed verified function record")
            if fields[0] == package:
                expected.add(fields[2])
    return expected


def json_objects(transcript: str) -> list[dict[str, object]]:
    decoder = json.JSONDecoder()
    objects = []
    cursor = 0
    while cursor < len(transcript):
        opening = transcript.find("{", cursor)
        if opening < 0:
            break
        try:
            value, end = decoder.raw_decode(transcript, opening)
        except json.JSONDecodeError:
            cursor = opening + 1
            continue
        if isinstance(value, dict) and "verification-results" in value:
            objects.append(value)
        cursor = end
    return objects


def main() -> None:
    if len(sys.argv) != 7:
        print(
            f"usage: {sys.argv[0]} PACKAGE CRATE MANIFEST TRANSCRIPT COUNTS_OUT EXPECTED_TOOL_VERSION",
            file=sys.stderr,
        )
        raise SystemExit(2)
    package, crate_name = sys.argv[1:3]
    manifest_path, transcript_path, counts_path = map(Path, sys.argv[3:6])
    expected_tool_version = sys.argv[6]
    try:
        transcript = transcript_path.read_text(encoding="utf-8")
        expected = manifest_functions(manifest_path, package)
    except (OSError, UnicodeError) as error:
        fail(str(error))
    objects = json_objects(transcript)
    if not expected:
        fail(f"{package} has no admitted directly verified executable bodies")
    if len(objects) != 1:
        fail(f"{package} transcript contains {len(objects)} structured root results, expected one")
    result = objects[0].get("verification-results")
    details = objects[0].get("func-details")
    verus = objects[0].get("verus")
    if not isinstance(result, dict) or not isinstance(details, dict) or not isinstance(verus, dict):
        fail(f"{package} structured Verus result is malformed")
    if (
        result.get("success") is not True
        or result.get("is-verifying-entire-crate") is not True
        or result.get("encountered-error") is not False
        or result.get("encountered-vir-error") is not False
        or result.get("errors") != 0
    ):
        fail(f"{package} structured Verus result is not an error-free whole-crate proof")
    if verus.get("version") != expected_tool_version or verus.get("profile") != "release":
        fail(f"{package} structured result came from the wrong Verus build")
    verified = result.get("verified")
    if not isinstance(verified, int) or verified <= 0 or verified < len(expected):
        fail(
            f"{package} verified count cannot cover admitted executable bodies: "
            f"verified={verified!r}, expected={len(expected)}"
        )
    missing = sorted(expected - set(details))
    if missing:
        fail(f"{package} admitted executable bodies are absent from compiler results: {missing}")
    own_paths = {name for name in details if name.startswith(crate_name + "::")}
    if expected and not own_paths:
        fail(f"{package} structured result contains no paths from crate {crate_name}")
    try:
        counts_path.write_text(
            f"{package}|{verified}|0|{len(expected)}\n",
            encoding="utf-8",
        )
    except OSError as error:
        fail(str(error))
    print(
        f"PASS: {package} structured proof covers {len(expected)} admitted executable bodies "
        f"({verified} verification queries)"
    )


if __name__ == "__main__":
    main()
