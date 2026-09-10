#!/usr/bin/env python3
"""Prepare a source-equal SDK workspace overlay for engineering TP extraction."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib


REV = "3546d54d2c4a913f5d079701aed557d0a378bba8"
GIT = "https://github.com/harsh-nod/fe2o3.git"


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def regular(path):
    require(path.is_file() and not path.is_symlink(), f"not a regular file: {path}")
    return path.read_bytes()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ferric", type=Path)
    parser.add_argument("fe2o3", type=Path)
    parser.add_argument("vendor", type=Path)
    args = parser.parse_args()
    for root in [args.ferric, args.fe2o3, args.vendor]:
        require(root.is_absolute() and root.resolve(strict=True) == root, "canonical paths required")
    require(not git(args.ferric, "status", "--porcelain"), "dirty Ferric source")
    require(not git(args.fe2o3, "status", "--porcelain"), "dirty compiler source")
    require(git(args.fe2o3, "rev-parse", "HEAD") == REV, "compiler revision drifted")
    manifest = tomllib.loads(regular(args.ferric / "device/qwen3-tp-kernels-v1/Cargo.toml").decode())
    device = manifest["dependencies"]["fe2o3-device"]
    host = manifest["target"]['cfg(not(target_arch = "amdgpu"))']["dependencies"]["fe2o3-host"]
    for dependency in [device, host]:
        require(dependency == {"git": GIT, "rev": REV, "version": "=0.1.0"}, "TP SDK pin drifted")
    source = args.fe2o3 / "crates/fe2o3-device"
    package = args.vendor / "fe2o3-device-0.1.0"
    source_paths = {path.relative_to(source) for path in (source / "src").rglob("*.rs")}
    vendor_paths = {path.relative_to(package) for path in (package / "src").rglob("*.rs")}
    require(source_paths == vendor_paths and bool(source_paths), "SDK source inventory drifted")
    for path in source_paths:
        require(regular(source / path) == regular(package / path), "SDK Rust body drifted")
    sdk_manifest = regular(source / "Cargo.toml")
    checksum_path = package / ".cargo-checksum.json"
    checksum = json.loads(regular(checksum_path))
    require(set(checksum) == {"files", "package"} and checksum["package"] is None
            and isinstance(checksum["files"], dict) and "Cargo.toml" in checksum["files"],
            "vendor checksum schema drifted")
    checksum["files"]["Cargo.toml"] = hashlib.sha256(sdk_manifest).hexdigest()
    workspace = args.vendor / "Cargo.toml"
    require(not workspace.exists(), "overlay already exists")
    template = regular(args.ferric / "proofs/m1-qualification/ENGINEERING_VENDOR_WORKSPACE_V1.toml")
    (package / "Cargo.toml").write_bytes(sdk_manifest)
    temporary = package / ".ferric-tp-checksum.tmp"
    with temporary.open("xb") as output:
        output.write(json.dumps(checksum, sort_keys=True, separators=(",", ":")).encode())
    os.replace(temporary, checksum_path)
    with workspace.open("xb") as output:
        output.write(template)
    print(f"PASS: engineering-only TP vendor overlay; SDK Rust files={len(source_paths)}; revision={REV}; no protected authority")


if __name__ == "__main__":
    main()
