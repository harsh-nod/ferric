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
PORTABLE_METADATA_DOMAIN = b"FE2O3/PORTABLE-SELECTED-RUSTC-METADATA/V1\0"
MANAGED_BINDING_WRAPPER = "/proc/self/fd/200"
MANAGED_RUSTC = "/proc/self/fd/194"
PORTABLE_CODEGEN_KEYS = frozenset(
    {
        "code-model",
        "codegen-units",
        "debuginfo",
        "debug-assertions",
        "embed-bitcode",
        "force-frame-pointers",
        "instrument-coverage",
        "lto",
        "no-redzone",
        "opt-level",
        "panic",
        "relocation-model",
        "soft-float",
        "strip",
        "target-cpu",
        "target-feature",
    }
)
HEX_64 = re.compile(r"^[0-9a-f]{64}$")
ANSI_SGR = re.compile(r"\x1b\[[0-9;]*m")
SHELL_ASSIGNMENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=.*$")


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


def update_portable_field(digest: object, key: str, value: str) -> None:
    update_field(digest, key)
    update_field(digest, value)


def derive_portable_metadata(
    argv: list[str],
    *,
    package_name: str,
    package_version: str,
    manifest_sha256: str,
    crate_name: str,
) -> str:
    if not argv:
        fail("rustc argument vector is empty")
    if option_value(argv, "--crate-name") != crate_name:
        fail("portable crate name differs from the canonical rustc invocation")
    if not package_name or not package_version:
        fail("portable package identity has an empty field")
    if HEX_64.fullmatch(manifest_sha256) is None:
        fail("portable manifest SHA-256 is malformed")

    cfgs: list[str] = []
    crate_types: list[str] = []
    identity_fields: list[tuple[str, str]] = []

    def record_option(key: str, value: str) -> None:
        if key == "cfg":
            cfgs.append(value)
        elif key == "crate-type":
            crate_types.append(value)
        elif key in {"edition", "target"}:
            identity_fields.append((key, value))
        else:
            fail(f"unsupported portable rustc option {key}")

    def record_codegen(value: str) -> None:
        key, separator, selected = value.partition("=")
        if separator and key in PORTABLE_CODEGEN_KEYS:
            identity_fields.append((key, selected))

    separate_options = {
        "--cfg": "cfg",
        "--crate-type": "crate-type",
        "--edition": "edition",
        "--target": "target",
    }
    joined_options = {
        "--cfg=": "cfg",
        "--crate-type=": "crate-type",
        "--edition=": "edition",
        "--target=": "target",
    }
    index = 1
    while index < len(argv):
        argument = argv[index]
        if argument == "--":
            break
        if argument in separate_options:
            value_index = index + 1
            if value_index >= len(argv):
                fail(f"{argument} has no value")
            record_option(separate_options[argument], argv[value_index])
            index += 2
            continue
        if argument in {"-C", "--codegen"}:
            value_index = index + 1
            if value_index >= len(argv):
                fail(f"{argument} has no value")
            record_codegen(argv[value_index])
            index += 2
            continue
        for prefix, key in joined_options.items():
            if argument.startswith(prefix):
                record_option(key, argument.removeprefix(prefix))
        if argument.startswith("-C"):
            record_codegen(argument[2:])
        elif argument.startswith("--codegen="):
            record_codegen(argument.removeprefix("--codegen="))
        index += 1

    digest = hashlib.sha256()
    digest.update(PORTABLE_METADATA_DOMAIN)
    for key, value in [
        ("package-name", package_name),
        ("package-version", package_version),
        ("manifest-sha256", manifest_sha256),
        ("crate-name", crate_name),
    ]:
        update_portable_field(digest, key, value)
    for cfg in sorted(set(cfgs)):
        update_portable_field(digest, "cfg", cfg)
    for crate_type in sorted(set(crate_types)):
        update_portable_field(digest, "crate-type", crate_type)
    for key, value in identity_fields:
        update_portable_field(digest, key, value)
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


def running_managed_invocation(line: str) -> tuple[dict[str, str], list[str]] | None:
    command = running_argv(line)
    if command is None or "--crate-name" not in command:
        return None
    index = 0
    environment: dict[str, str] = {}
    while index < len(command) and SHELL_ASSIGNMENT.fullmatch(command[index]):
        name, value = command[index].split("=", 1)
        if name in environment:
            fail(f"Cargo verbose command repeats environment assignment {name}")
        environment[name] = value
        index += 1
    if (
        len(command) - index < 2
        or command[index] != MANAGED_BINDING_WRAPPER
        or command[index + 1] != MANAGED_RUSTC
    ):
        return None
    rustc = command[index + 1 :]
    if "--crate-name" not in rustc:
        fail("Cargo verbose wrapper did not retain the rustc compile arguments")
    return environment, rustc


def running_rustc_argv(line: str) -> list[str] | None:
    invocation = running_managed_invocation(line)
    return None if invocation is None else invocation[1]


def require_workspace_environment(
    environment: dict[str, str],
    *,
    workspace: Path,
    crate_name: str,
    package_name: str,
    package_version: str,
) -> None:
    expected = {
        "CARGO_CRATE_NAME": crate_name,
        "CARGO_MANIFEST_DIR": str(workspace),
        "CARGO_MANIFEST_PATH": str(workspace / "Cargo.toml"),
        "CARGO_PKG_NAME": package_name,
        "CARGO_PKG_VERSION": package_version,
        "CARGO_PRIMARY_PACKAGE": "1",
    }
    for name, value in expected.items():
        if environment.get(name) != value:
            fail(f"canonical rustc invocation has wrong {name}")


def canonical_invocation(
    transcript: Path,
    workspace: Path,
    crate_name: str,
    package_name: str,
    package_version: str,
) -> list[str]:
    matches: list[list[str]] = []
    for line in transcript.read_text(encoding="utf-8").splitlines():
        invocation = running_managed_invocation(line)
        if invocation is None:
            continue
        environment, argv = invocation
        if option_value(argv, "--crate-name") != crate_name:
            continue
        crate_types = option_value(argv, "--crate-type").split(",")
        if "lib" in crate_types:
            require_workspace_environment(
                environment,
                workspace=workspace,
                crate_name=crate_name,
                package_name=package_name,
                package_version=package_version,
            )
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


def binding_parity_error(derived: str, fallback: str) -> str | None:
    if derived == fallback:
        return None
    return (
        "compiler-derived binding differs from the explicit fallback: "
        f"compiler={derived} fallback={fallback}"
    )


def verify(workspace: Path, transcript: Path) -> None:
    manifest_bytes = (workspace / "Cargo.toml").read_bytes()
    manifest = tomllib.loads(manifest_bytes.decode("utf-8"))
    package_name = manifest.get("package", {}).get("name")
    package_version = manifest.get("package", {}).get("version")
    if not isinstance(package_name, str) or not package_name:
        fail("device manifest has no exact package name")
    if not isinstance(package_version, str) or not package_version:
        fail("device manifest has no exact package version")
    crate_name = manifest.get("lib", {}).get("name")
    if not isinstance(crate_name, str) or not crate_name:
        fail("device manifest has no exact lib name")
    argv = canonical_invocation(
        transcript,
        workspace,
        crate_name,
        package_name,
        package_version,
    )
    raw_metadata = codegen_values(argv, "metadata")
    if not raw_metadata:
        fail("canonical rustc invocation has no explicit metadata")
    portable_metadata = derive_portable_metadata(
        argv,
        package_name=package_name,
        package_version=package_version,
        manifest_sha256=hashlib.sha256(manifest_bytes).hexdigest(),
        crate_name=crate_name,
    )
    derived = derive_binding(crate_name, [portable_metadata])
    extra_filename = codegen_values(argv, "extra-filename")
    if len(extra_filename) != 1:
        fail(f"expected one rustc extra-filename, found {len(extra_filename)}")
    out_dir = Path(option_value(argv, "--out-dir"))
    metadata_artifact = out_dir / f"lib{crate_name}{extra_filename[0]}.rmeta"
    if not metadata_artifact.is_file():
        fail(f"canonical metadata artifact is missing: {metadata_artifact}")
    if derived.encode("ascii") not in metadata_artifact.read_bytes():
        fail("wrapper-derived binding is absent from the canonical metadata artifact")
    fallback = fallback_binding(workspace)
    if parity_error := binding_parity_error(derived, fallback):
        fail(parity_error)
    print(
        "PASS: canonical device binding embedded "
        f"crate={crate_name} raw_metadata={','.join(raw_metadata)} "
        f"portable_metadata={portable_metadata} "
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
    colored = (
        "\x1b[1m\x1b[92m     Running\x1b[0m "
        "`CARGO_CRATE_NAME=demo CARGO_MANIFEST_DIR=/workspace "
        "CARGO_MANIFEST_PATH=/workspace/Cargo.toml CARGO_PKG_NAME=demo "
        "CARGO_PKG_VERSION=0.1.0 CARGO_PRIMARY_PACKAGE=1 "
        f"{MANAGED_BINDING_WRAPPER} {MANAGED_RUSTC} "
        "--crate-name demo --crate-type lib`"
    )
    if running_rustc_argv(colored) != [
        MANAGED_RUSTC,
        "--crate-name",
        "demo",
        "--crate-type",
        "lib",
    ]:
        fail("Cargo environment, wrapper, or ANSI bytes changed command parsing")
    managed = running_managed_invocation(colored)
    if managed is None:
        fail("managed Cargo invocation was not retained")
    require_workspace_environment(
        managed[0],
        workspace=Path("/workspace"),
        crate_name="demo",
        package_name="demo",
        package_version="0.1.0",
    )
    direct = "     Running `rustc --crate-name demo --crate-type lib`"
    if running_rustc_argv(direct) is not None:
        fail("direct rustc invocation was mistaken for a managed fe2o3 build")
    golden = derive_binding("ferric_qwen3_gemm_device_v1", ["d3a576c41b0b5cf4"])
    if golden != "e74f99e6ef7616bc5baa58242567f3a181137796c0ed7d53c827d054a5fc19f1":
        fail("crate-binding derivation drifted from the canonical golden vector")
    hostile = "f" + golden[1:]
    if hostile == golden or HEX_64.fullmatch(hostile) is None:
        fail("one-nibble hostile binding fixture is invalid")
    if binding_parity_error(golden, golden) is not None:
        fail("equal compiler and fallback bindings did not pass parity")
    if binding_parity_error(golden, hostile) is None:
        fail("hostile fallback binding did not fail parity")

    portable_argv = [
        "rustc",
        "--crate-name",
        "unit",
        "unit.rs",
        "--target=amdgcn-amd-amdhsa",
        "--crate-type=lib",
        "--edition=2024",
        '--cfg=feature="kernel"',
        "-Ctarget-cpu=gfx942",
        "-Ctarget-feature=-wavefrontsize32,+wavefrontsize64,-xnack",
        "-Copt-level=3",
        "-Cmetadata=cargo-salt",
        "-Cextra-filename=-cargo-salt",
    ]
    portable = derive_portable_metadata(
        portable_argv,
        package_name="package",
        package_version="1.0.0",
        manifest_sha256="01" * 32,
        crate_name="unit",
    )
    if portable != "dd3e0082f3ac34a728c000c43c98bc8321362f6712224fb06fbefdd5d670324c":
        fail("portable metadata derivation drifted from the fe2o3 golden vector")
    portable_binding = derive_binding("unit", [portable])
    if portable_binding != "2cdd9f4075fe23cada4c8310aecabd2db9252509381155a5ed26a09e786190f0":
        fail("portable crate binding drifted from the fe2o3 composition vector")

    split = derive_portable_metadata(
        [
            "rustc",
            "--crate-name",
            "unit",
            "unit.rs",
            "--cfg",
            'feature="kernel"',
            "--crate-type",
            "lib",
            "--edition",
            "2024",
            "--target",
            "amdgcn-amd-amdhsa",
            "--codegen",
            "target-cpu=gfx942",
            "-C",
            "opt-level=3",
        ],
        package_name="package",
        package_version="1.0.0",
        manifest_sha256="01" * 32,
        crate_name="unit",
    )
    joined = derive_portable_metadata(
        [
            "rustc",
            "--crate-name",
            "unit",
            "/different/checkout/unit.rs",
            '--cfg=feature="kernel"',
            "--crate-type=lib",
            "--edition=2024",
            "--target=amdgcn-amd-amdhsa",
            "--codegen=target-cpu=gfx942",
            "-Copt-level=3",
            "-Cmetadata=unrelated-cargo-salt",
            "-Cextra-filename=-unrelated-cargo-salt",
            "--out-dir=/different/target",
        ],
        package_name="package",
        package_version="1.0.0",
        manifest_sha256="01" * 32,
        crate_name="unit",
    )
    if split != joined:
        fail("portable metadata changed across split options or checkout paths")
    after_terminator = derive_portable_metadata(
        [*portable_argv, "--", "-Copt-level=0", "--target=hostile"],
        package_name="package",
        package_version="1.0.0",
        manifest_sha256="01" * 32,
        crate_name="unit",
    )
    if after_terminator != portable:
        fail("arguments after the rustc terminator changed portable metadata")
    changed_target = portable_argv.copy()
    changed_target[4] = "--target=x86_64-unknown-linux-gnu"
    if (
        derive_portable_metadata(
            changed_target,
            package_name="package",
            package_version="1.0.0",
            manifest_sha256="01" * 32,
            crate_name="unit",
        )
        == portable
    ):
        fail("target substitution did not change portable metadata")
    print("PASS: device binding parser and portable derivation vectors matched")


def main() -> None:
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} DEVICE_WORKSPACE CARGO_VERBOSE_TRANSCRIPT")
    verify(Path(sys.argv[1]).resolve(strict=True), Path(sys.argv[2]).resolve(strict=True))


if __name__ == "__main__":
    main()
