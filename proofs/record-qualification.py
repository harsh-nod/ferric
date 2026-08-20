#!/usr/bin/env python3
"""Write an identity-bound Ferric proof/release qualification receipt."""

from __future__ import annotations

import hashlib
import json
import shutil
import stat
import subprocess
import sys
from pathlib import Path

COMMIT = "b677dd5a766f25f56e9aa1e32621aa4e53304b47"
COMMAND = (
    "for each compiler-rooted package in dependency order: cargo-verus build -p PACKAGE "
    "--locked --release --target-dir FRESH --fwd-verus-args-to roots -j 1 -- "
    "--no-cheating --output-json"
)


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def identity(path: Path) -> str:
    mode = stat.S_IMODE(path.stat().st_mode)
    return f"{path.name}|{mode:o}|{path.stat().st_size}|{digest(path)}"


def artifact_identity(path: Path) -> str:
    """Artifact transport does not preserve Unix permission bits."""
    return f"{path.name}|{path.stat().st_size}|{digest(path)}"


def tool_output(arguments: list[str]) -> str:
    return subprocess.run(arguments, check=True, text=True, stdout=subprocess.PIPE).stdout.strip().replace("\n", " / ")


def main() -> None:
    if len(sys.argv) != 12:
        print(
            f"usage: {sys.argv[0]} REPO VERUS_ROOT TARGET METADATA TRANSCRIPT COUNTS "
            "CLOSURE_TRANSCRIPT NEGATIVE_DIR SOURCE_RECORDS SOURCE_GATE RECEIPT_DIR",
            file=sys.stderr,
        )
        raise SystemExit(2)
    repo, verus_root, target, metadata_path, transcript, counts_path, closure_log, negative_dir, source_records_input, source_gate, receipt_dir = map(
        Path, sys.argv[1:]
    )
    if receipt_dir.exists() and any(receipt_dir.iterdir()):
        fail(f"receipt directory is not empty: {receipt_dir}")
    receipt_dir.mkdir(parents=True, exist_ok=True)
    if not source_records_input.is_file():
        fail(f"pre-build source records are unavailable: {source_records_input}")

    try:
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        counts = [line.split("|") for line in counts_path.read_text(encoding="utf-8").splitlines()]
        verified_manifest = repo / "proofs/VERIFIED_MODULES"
        verified_compiler_paths = [
            line.removeprefix("verified=")
            for line in verified_manifest.read_text(encoding="utf-8").splitlines()
            if line.startswith("verified=")
        ]
        manifest_fields = dict(
            line.split("=", 1)
            for line in (repo / "proofs/verus/VERUS_CLOSURE_MANIFEST").read_text(encoding="utf-8").splitlines()
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        fail(str(error))
    if any(len(row) != 4 for row in counts):
        fail("proof count evidence is malformed")
    if len({row[0] for row in counts}) != len(counts):
        fail("proof count evidence contains duplicate packages")

    source_records = receipt_dir / "source-closure.records"
    shutil.copyfile(source_records_input, source_records)

    workspace_ids = set(metadata["workspace_members"])
    opted_packages = sorted(
        [package
        for package in metadata["packages"]
        if package["id"] in workspace_ids
        and package.get("metadata", {}).get("verus", {}).get("verify") is True
        ],
        key=lambda package: package["name"],
    )
    count_packages = {row[0] for row in counts}
    opted_names = {package["name"] for package in opted_packages}
    if count_packages != opted_names:
        fail("proof count evidence does not cover exactly the opted packages")
    if len(set(verified_compiler_paths)) != len(verified_compiler_paths):
        fail("verified compiler path evidence contains duplicates")
    admitted_counts = {name: 0 for name in opted_names}
    for record in verified_compiler_paths:
        fields = record.split("|")
        if len(fields) != 3 or fields[0] not in admitted_counts:
            fail("verified compiler path evidence is malformed")
        admitted_counts[fields[0]] += 1
    artifacts_dir = receipt_dir / "artifacts"
    artifacts_dir.mkdir()
    artifacts: list[str] = []
    for package in opted_packages:
        library_targets = [target_item for target_item in package["targets"] if "lib" in target_item["kind"]]
        if len(library_targets) != 1:
            fail(f"expected one library target for {package['name']}")
        artifact_name = f"lib{library_targets[0]['name'].replace('-', '_')}.rlib"
        source = target / "release" / artifact_name
        if not source.is_file():
            fail(f"qualified artifact is missing: {source}")
        destination = artifacts_dir / artifact_name
        shutil.copyfile(source, destination)
        if digest(source) != digest(destination):
            fail(f"artifact copy drifted: {artifact_name}")
        artifacts.append(f"{package['name']}|{artifact_identity(destination)}")

    evidence = {
        "cargo-metadata.json": metadata_path,
        "proof-build.transcript": transcript,
        "proof-counts.txt": counts_path,
        "verus-closure.transcript": closure_log,
    }
    for name, source in evidence.items():
        shutil.copyfile(source, receipt_dir / name)
    if negative_dir.is_dir():
        destination = receipt_dir / "negative"
        shutil.copytree(negative_dir, destination)

    tools = {
        name: verus_root / name
        for name in ("cargo-verus", "verus", "rust_verify", "z3")
    }
    tools["ferric-source-gate"] = source_gate
    for name, path in tools.items():
        if not path.is_file():
            fail(f"missing authenticated tool {name}")

    try:
        verification_queries = sum(int(row[1]) for row in counts)
        direct_total = sum(int(row[3]) for row in counts)
    except ValueError as error:
        fail(f"proof count evidence is malformed: {error}")
    if any(row[2] != "0" for row in counts) or verification_queries <= 0 or direct_total <= 0:
        fail("proof count evidence does not attest nonzero error-free direct coverage")
    if any(int(row[1]) <= 0 or int(row[3]) <= 0 for row in counts):
        fail("every opted package must have nonzero direct verified coverage")
    if any(
        int(row[3]) != admitted_counts[row[0]] or int(row[1]) < int(row[3])
        for row in counts
    ):
        fail("proof count evidence does not match admitted compiler paths")
    rustc_path = Path(tool_output(["rustc", "--print", "sysroot"])) / "bin/rustc"
    cargo_path = Path(tool_output(["rustc", "--print", "sysroot"])) / "bin/cargo"
    host_tools = {}
    for name in (
        "sh",
        "awk",
        "cat",
        "chmod",
        "cmp",
        "cp",
        "dirname",
        "grep",
        "mkdir",
        "mktemp",
        "python3",
        "rm",
        "sed",
        "sha256sum",
        "sort",
        "timeout",
        "tr",
        "uname",
    ):
        resolved = shutil.which(name)
        if resolved is None:
            fail(f"qualification host tool is unavailable: {name}")
        host_tools[name] = Path(resolved)
    fields = [
        "format=FERRIC-QUALIFICATION-RECEIPT-V1",
        "status=PASS",
        "qualification-entrypoint=proofs/qualify-release.sh",
        f"command={COMMAND}",
        "cargo-profile=release",
        "fresh-target=true",
        "read-only-source-snapshot=true",
        "qualification-hermetic=false",
        "source-before-after-equality=required",
        "no-cheating=first-party-roots",
        "erasure-check=enabled-default",
        "dependency-trust=pinned-vstd-closure",
        f"verus-commit={COMMIT}",
        f"verus-version={tool_output([str(tools['verus']), '--version'])}",
        f"verus-archive-sha256={(repo / 'proofs/verus/VERUS_ARCHIVE_SHA256').read_text(encoding='ascii').strip()}",
        f"verus-closure-manifest-sha256={digest(repo / 'proofs/verus/VERUS_CLOSURE_MANIFEST')}",
        f"verus-closure-file-count={manifest_fields['file-count']}",
        f"verus-closure-sha256={manifest_fields['closure-sha256']}",
        f"vstd-closure={manifest_fields['subtree']}",
        f"cargo-lock-sha256={digest(repo / 'Cargo.lock')}",
        f"source-gate-lock-sha256={digest(repo / 'proofs/source-gate/Cargo.lock')}",
        f"source-gate-tcb-sha256={digest(repo / 'proofs/source-gate/DEPENDENCY_TCB')}",
        f"unverified-bodies-sha256={digest(repo / 'proofs/UNVERIFIED_BODIES')}",
        f"verified-modules-sha256={digest(verified_manifest)}",
        f"cargo-metadata-sha256={digest(metadata_path)}",
        f"source-closure-sha256={digest(source_records)}",
        f"proof-transcript-sha256={digest(transcript)}",
        f"verification-queries={verification_queries}",
        f"direct-verified-bodies={direct_total}",
        f"opted-packages={','.join(package['name'] for package in opted_packages)}",
        f"rustc={tool_output(['rustc', '-vV'])}",
        f"rustc-binary={identity(rustc_path)}",
        f"cargo={tool_output(['cargo', '-V'])}",
        f"cargo-binary={identity(cargo_path)}",
        "claim-boundary=verified Rust source bodies plus default Verus erasure checks",
        "nonclaim=rustc linker runtime GPU execution and machine-code refinement remain outside this proof",
        "qualification-host-tcb=ambient Rust/Cargo, Python, POSIX shell/coreutils, OS, filesystem, and process supervision are contracted",
        "nonclaim=qualification evidence production is identity-bound but not hermetic or independently reproduced",
        "source-gate-tcb=source-gate binary and complete locked dependency closure including proc macros and build scripts",
    ]
    fields.extend(f"tool={name}|{identity(path)}" for name, path in tools.items())
    fields.extend(f"host-tool={name}|{identity(path)}" for name, path in host_tools.items())
    fields.extend(f"proof-count={'|'.join(row)}" for row in counts)
    fields.extend(f"verified-compiler-path={path}" for path in verified_compiler_paths)
    fields.extend(f"artifact={artifact}" for artifact in artifacts)
    negative_files = sorted((receipt_dir / "negative").glob("*")) if (receipt_dir / "negative").is_dir() else []
    fields.extend(
        f"negative-evidence={path.name}|{digest(path)}" for path in negative_files if path.is_file()
    )

    receipt = receipt_dir / "qualification.receipt"
    receipt.write_text("\n".join(fields) + "\n", encoding="utf-8")
    receipt_hash = digest(receipt)
    (receipt_dir / "qualification.receipt.sha256").write_text(
        f"{receipt_hash}  qualification.receipt\n", encoding="ascii"
    )
    for path in receipt_dir.rglob("*"):
        if path.is_file():
            path.chmod(0o444)
    print(f"RECEIPT={receipt}")
    print(f"RECEIPT_SHA256={receipt_hash}")
    for artifact in artifacts:
        print(f"ARTIFACT={artifact}")


if __name__ == "__main__":
    main()
