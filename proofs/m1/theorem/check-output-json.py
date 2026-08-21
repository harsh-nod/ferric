#!/usr/bin/env python3
"""Check one selected-function Verus output-json success result."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import sys
from typing import Any, NoReturn


VERUS_COMMIT = "b677dd5a766f25f56e9aa1e32621aa4e53304b47"
SAFE_TARGET = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*\Z")
ROOT_KEYS = {"func-details", "verification-results", "verus"}
RESULT_KEYS = {
    "encountered-error",
    "encountered-vir-error",
    "errors",
    "is-verifying-entire-crate",
    "verified",
}
VERUS_KEYS = {"commit", "platform", "profile", "toolchain", "version"}
PLATFORM_KEYS = {"arch", "os"}
DETAIL_KEYS = {"failed_proof_notes", "obligation_proof_notes"}


def fail(message: str) -> NoReturn:
    raise SystemExit(f"FAIL: {message}")


def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            fail(f"duplicate Verus output-json key: {key}")
        value[key] = item
    return value


def structured_objects(transcript: str) -> list[dict[str, Any]]:
    decoder = json.JSONDecoder(object_pairs_hook=reject_duplicates)
    objects: list[dict[str, Any]] = []
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


def exact_object(value: Any, keys: set[str], description: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        fail(f"{description} fields drifted")
    return value


def main() -> None:
    if len(sys.argv) != 8:
        fail(
            "usage: check-output-json.py PACKAGE MODULE FUNCTION TRANSCRIPT "
            "SUMMARY EXPECTED_VERSION EXPECTED_TOOLCHAIN"
        )
    package, module, function = sys.argv[1:4]
    transcript_path = Path(sys.argv[4])
    summary_path = Path(sys.argv[5])
    expected_version, expected_toolchain = sys.argv[6:8]
    if any(SAFE_TARGET.fullmatch(value) is None for value in (module, function)):
        fail("unsafe selected Verus target")
    compiler_path = f"{package.replace('-', '_')}::{module}::{function}"
    try:
        transcript_raw = transcript_path.read_bytes()
        transcript = transcript_raw.decode("utf-8")
    except (OSError, UnicodeError) as error:
        fail(str(error))
    objects = structured_objects(transcript)
    if len(objects) != 1:
        fail(f"selected-function transcript has {len(objects)} structured root results")
    root = exact_object(objects[0], ROOT_KEYS, "structured Verus root")
    result = exact_object(
        root["verification-results"], RESULT_KEYS, "structured verification result"
    )
    details = root["func-details"]
    verus = exact_object(root["verus"], VERUS_KEYS, "structured Verus identity")
    platform = exact_object(verus["platform"], PLATFORM_KEYS, "Verus platform")
    if (
        result["is-verifying-entire-crate"] is not False
        or result["encountered-error"] is not False
        or result["encountered-vir-error"] is not False
        or result["errors"] != 0
        or not isinstance(result["errors"], int)
        or isinstance(result["errors"], bool)
        or not isinstance(result["verified"], int)
        or isinstance(result["verified"], bool)
        or result["verified"] != 1
    ):
        fail("selected-function structured Verus result is not an error-free proof")
    if not isinstance(details, dict) or compiler_path not in details:
        fail("selected compiler function is absent from Verus func-details")
    for detail_path, detail_value in details.items():
        if not isinstance(detail_path, str) or not detail_path:
            fail("structured function detail path is invalid")
        detail = exact_object(detail_value, DETAIL_KEYS, "structured function detail")
        if detail["failed_proof_notes"] != [] or detail["obligation_proof_notes"] != []:
            fail("structured function detail contains unresolved proof notes")
    if (
        verus["commit"] != VERUS_COMMIT
        or verus["version"] != expected_version
        or verus["profile"] != "release"
        or verus["toolchain"] != expected_toolchain
        or platform != {"arch": "x86_64", "os": "linux"}
    ):
        fail("structured Verus tool identity drifted")
    if summary_path.exists() or summary_path.is_symlink():
        fail("structured Verus summary output already exists")
    summary_path.write_text(
        "FORMAT=FERRIC-M1-POSITIVE-OUTPUT-V1\n"
        f"COMPILER_PATH={compiler_path}\n"
        f"TRANSCRIPT_SHA256={hashlib.sha256(transcript_raw).hexdigest()}\n"
        f"VERIFIED_COUNT={result['verified']}\n"
        f"DETAILS_COUNT={len(details)}\n"
        "IS_VERIFYING_ENTIRE_CRATE=false\n"
        "ENCOUNTERED_ERROR=false\n"
        "ENCOUNTERED_VIR_ERROR=false\n"
        "ERRORS=0\n"
        "RESULT=success\n",
        encoding="ascii",
    )
    print(
        f"PASS: structured Verus proved selected compiler path {compiler_path} "
        f"({result['verified']} queries)"
    )


if __name__ == "__main__":
    main()
