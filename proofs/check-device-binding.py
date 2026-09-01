#!/usr/bin/env python3
"""Verify one canonical cargo-fe2o3 host binding and explicit fallback."""

from __future__ import annotations

import hashlib
import re
import shlex
import sys
import tomllib
from pathlib import Path
from typing import NoReturn


BINDING_DOMAIN = b"fe2o3.crate-binding.v1\0"
HEX_64 = re.compile(r"^[0-9a-f]{64}$")
ANSI_SGR = re.compile(r"\x1b\[[0-9;]*m")


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def update_field(digest: object, value: str) -> None:
    encoded = value.encode("utf-8")
    digest.update(len(encoded).to_bytes(8, "little"))
    digest.update(encoded)


def derive_binding(crate_name: str, metadata: list[str]) -> str:
    digest = hashlib.sha256()
    digest.update(BINDING_DOMAIN)
    update_field(digest, crate_name)
    digest.update(len(metadata).to_bytes(8, "little"))
    for value in metadata:
        if not value:
            fail("rustc metadata value is empty")
        update_field(digest, value)
    return digest.hexdigest()


def codegen_values(argv: list[str], name: str) -> list[str]:
    values: list[str] = []
    index = 1
    while index < len(argv):
        argument = argv[index]
        if argument == "--":
            break
        value: str | None = None
        if argument in {"-C", "--codegen"}:
            if index + 1 >= len(argv):
                fail(f"split {argument} option has no value")
            value = argv[index + 1]
            index += 2
        elif argument.startswith("-C"):
            value = argument[2:]
            index += 1
        elif argument.startswith("--codegen="):
            value = argument.removeprefix("--codegen=")
            index += 1
        else:
            index += 1
        if value is not None and value.startswith(f"{name}="):
            selected = value.removeprefix(f"{name}=")
            if not selected:
                fail(f"rustc {name} value is empty")
            values.append(selected)
    return values


def option_value(argv: list[str], name: str) -> str:
    joined = f"{name}="
    values: list[str] = []
    for index, argument in enumerate(argv):
        if argument == name:
            if index + 1 >= len(argv):
                fail(f"{name} has no value")
            values.append(argv[index + 1])
        elif argument.startswith(joined):
            values.append(argument.removeprefix(joined))
    if len(values) != 1:
        fail(f"expected one {name} value, found {len(values)}")
    return values[0]


def running_argv(line: str) -> list[str] | None:
    line = ANSI_SGR.sub("", line)
    marker = "Running `"
    if marker not in line or not line.rstrip().endswith("`"):
        return None
    command = line.split(marker, 1)[1].rstrip()[:-1]
    try:
        return shlex.split(command)
    except ValueError as error:
        fail(f"cannot parse Cargo verbose command: {error}")


def canonical_invocation(transcript: Path, crate_name: str) -> list[str]:
    matches: list[list[str]] = []
    for line in transcript.read_text(encoding="utf-8").splitlines():
        argv = running_argv(line)
        if argv is None or "--crate-name" not in argv:
            continue
        if option_value(argv, "--crate-name") != crate_name:
            continue
        crate_types = option_value(argv, "--crate-type").split(",")
        if "lib" in crate_types:
            matches.append(argv)
    if len(matches) != 1:
        fail(
            f"expected one canonical lib rustc invocation for {crate_name}, "
            f"found {len(matches)}"
        )
    return matches[0]


def fallback_binding(workspace: Path) -> str:
    matches = re.findall(r'"([0-9a-f]{64})"', (workspace / "build.rs").read_text())
    if len(matches) != 1:
        fail(f"expected one canonical fallback binding, found {len(matches)}")
    return matches[0]


def verify(workspace: Path, transcript: Path) -> None:
    manifest = tomllib.loads((workspace / "Cargo.toml").read_text(encoding="utf-8"))
    crate_name = manifest.get("lib", {}).get("name")
    if not isinstance(crate_name, str) or not crate_name:
        fail("device manifest has no exact lib name")
    argv = canonical_invocation(transcript, crate_name)
    metadata = codegen_values(argv, "metadata")
    if not metadata:
        fail("canonical rustc invocation has no explicit metadata")
    derived = derive_binding(crate_name, metadata)
    fallback = fallback_binding(workspace)
    extra_filename = codegen_values(argv, "extra-filename")
    if len(extra_filename) != 1:
        fail(f"expected one rustc extra-filename, found {len(extra_filename)}")
    out_dir = Path(option_value(argv, "--out-dir"))
    metadata_artifact = out_dir / f"lib{crate_name}{extra_filename[0]}.rmeta"
    if not metadata_artifact.is_file():
        fail(f"canonical metadata artifact is missing: {metadata_artifact}")
    if derived.encode("ascii") not in metadata_artifact.read_bytes():
        fail("wrapper-derived binding is absent from the canonical metadata artifact")
    print(
        "PASS: canonical device binding embedded "
        f"crate={crate_name} metadata={','.join(metadata)} "
        f"compiler_binding={derived} fallback_binding={fallback}"
    )


def self_test() -> None:
    argv = [
        "rustc",
        "-C",
        "metadata=first",
        "-Cmetadata=second",
        "--codegen",
        "metadata=first",
        "--codegen=metadata=third",
    ]
    if codegen_values(argv, "metadata") != ["first", "second", "first", "third"]:
        fail("metadata parser did not retain all four spellings in order")
    colored = "\x1b[1m\x1b[92m     Running\x1b[0m `rustc --crate-name demo --crate-type lib`"
    if running_argv(colored) != ["rustc", "--crate-name", "demo", "--crate-type", "lib"]:
        fail("Cargo ANSI presentation bytes changed command parsing")
    golden = derive_binding("ferric_qwen3_gemm_device_v1", ["d3a576c41b0b5cf4"])
    if golden != "e74f99e6ef7616bc5baa58242567f3a181137796c0ed7d53c827d054a5fc19f1":
        fail("crate-binding derivation drifted from the canonical golden vector")
    hostile = "f" + golden[1:]
    if hostile == golden or HEX_64.fullmatch(hostile) is None:
        fail("one-nibble hostile binding fixture is invalid")
    print("PASS: device binding parser and canonical derivation golden vector matched")


def main() -> None:
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} DEVICE_WORKSPACE CARGO_VERBOSE_TRANSCRIPT")
    verify(Path(sys.argv[1]).resolve(strict=True), Path(sys.argv[2]).resolve(strict=True))


if __name__ == "__main__":
    main()
