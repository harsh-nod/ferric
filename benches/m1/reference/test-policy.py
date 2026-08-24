#!/usr/bin/env python3
"""Static policy gate for the Ferric M1 reference package."""

from __future__ import annotations

import ast
import hashlib
import json
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent
EXPECTED_FILES = {
    "implementation.json",
    "protocol.json",
    "pyproject.toml",
    "run.py",
    "test-policy.py",
    "test_reference.py",
    "uv.lock",
}
DIRECT_PINS = {
    "numpy": "1.26.4",
    "safetensors": "0.5.3",
    "tokenizers": "0.21.4",
    "torch": "2.12.1+rocm7.2",
    "transformers": "4.51.0",
    "triton-rocm": "3.7.1",
}
ROCM_INDEX = "https://download.pytorch.org/whl/rocm7.2"
COMPLETION_WAIT_POLICY = {
    "id": "ferric-m1-completion-progress-wait-v2",
    "max_consecutive_scans_without_progress": 8_192,
    "minimum_pending_scan_pause_micros": 10_000,
    "timeout_basis": "paced-completion-signal-scans",
    "total_scan_bound_rule": "(packet-count+1)*max-consecutive-scans-without-progress",
}


def fail(message: str) -> None:
    raise SystemExit(f"reference policy: {message}")


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(
            value, allow_nan=False, ensure_ascii=True, indent=2, sort_keys=True
        ).encode("ascii")
        + b"\n"
    )


def load_canonical(name: str) -> tuple[Any, bytes]:
    data = (ROOT / name).read_bytes()
    if not data.isascii():
        fail(f"{name} is not ASCII")
    try:
        value = json.loads(data)
    except json.JSONDecodeError as error:
        fail(f"cannot parse {name}: {error}")
    if canonical_bytes(value) != data:
        fail(f"{name} is not canonical JSON")
    return value, data


def dependency_policy() -> None:
    project = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="ascii"))
    if project["project"]["requires-python"] != "==3.12.*":
        fail("Python version is not pinned to 3.12")
    expected = {
        f"{name}=={version}"
        if name != "triton-rocm"
        else f"{name}=={version}; sys_platform == 'linux'"
        for name, version in DIRECT_PINS.items()
    }
    if set(project["project"]["dependencies"]) != expected:
        fail("direct dependency roster or pin drifted")
    indexes = project["tool"]["uv"]["index"]
    if indexes != [{"name": "pytorch-rocm", "url": ROCM_INDEX, "explicit": True}]:
        fail("explicit ROCm package index drifted")
    sources = project["tool"]["uv"]["sources"]
    if sources != {
        "torch": {"index": "pytorch-rocm"},
        "triton-rocm": {"index": "pytorch-rocm"},
    }:
        fail("ROCm package source bindings drifted")

    lock = tomllib.loads((ROOT / "uv.lock").read_text(encoding="utf-8"))
    if lock.get("version") != 1 or lock.get("requires-python") != "==3.12.*":
        fail("uv lock schema or Python constraint drifted")
    packages = {package["name"]: package for package in lock["package"]}
    for name, version in DIRECT_PINS.items():
        package = packages.get(name)
        if package is None or package.get("version") != version:
            fail(f"uv lock does not contain exact {name}=={version}")
    for name in ("torch", "triton-rocm"):
        if packages[name].get("source") != {"registry": ROCM_INDEX}:
            fail(f"uv lock does not source {name} from the ROCm 7.2 index")


def implementation_policy() -> None:
    manifest, manifest_data = load_canonical("implementation.json")
    if set(manifest) != {"authority", "files", "format", "python"}:
        fail("implementation manifest field roster drifted")
    if manifest["authority"] != "source-reviewed-reference-implementation-closure-only":
        fail("implementation manifest authority drifted")
    if manifest["format"] != "FERRIC-M1-REFERENCE-IMPLEMENTATION-V1":
        fail("implementation manifest format drifted")
    if manifest["python"] != "3.12":
        fail("implementation manifest Python binding drifted")
    if [item["path"] for item in manifest["files"]] != [
        "pyproject.toml",
        "run.py",
        "uv.lock",
    ]:
        fail("implementation manifest file roster drifted")
    for item in manifest["files"]:
        if set(item) != {"bytes", "path", "sha256"}:
            fail("implementation manifest file fields drifted")
        data = (ROOT / item["path"]).read_bytes()
        if (
            item["bytes"] != len(data)
            or item["sha256"] != hashlib.sha256(data).hexdigest()
        ):
            fail(f"implementation manifest no longer authenticates {item['path']}")
    if not manifest_data:
        fail("implementation manifest is empty")


def protocol_policy() -> None:
    protocol, _ = load_canonical("protocol.json")
    if protocol["format"] != "FERRIC-M1-REFERENCE-PROTOCOL-V3":
        fail("protocol format drifted")
    if protocol["dependencies"] != {"python": "3.12", **DIRECT_PINS}:
        fail("protocol dependency contract drifted")
    execution = protocol["execution"]
    expected = {
        "attention_implementation": "sdpa",
        "completion_wait_policy": COMPLETION_WAIT_POLICY,
        "determinism": "two-byte-identical-executions-per-case",
        "input_encoding": "lane-major-u32-le",
        "lane_execution": "sequential-full-context-per-lane-twice",
        "model_forward": "model.model(use_cache=false)",
        "network": "offline",
        "package_provenance": "active-non-base-virtualenv-only",
        "projection": "model.lm_head(last_hidden_state[:, -1:, :])",
        "python_isolation": "isolated-ignore-environment-no-user-site-safe-path",
        "remote_code": False,
        "row_order": "declared-lane-order",
    }
    if execution != expected:
        fail("protocol execution semantics drifted")
    if protocol["output"].get("bundle_naming") != "KIND.reference.bundle":
        fail("protocol reference bundle naming drifted")


def source_policy() -> None:
    source = (ROOT / "run.py").read_text(encoding="ascii")
    tree = ast.parse(source, filename="run.py")
    forbidden_imports = {"requests", "urllib", "huggingface_hub"}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names = {alias.name.split(".")[0] for alias in node.names}
            if names & forbidden_imports:
                fail("runner imports a network-capable module")
        if (
            isinstance(node, ast.ImportFrom)
            and (node.module or "").split(".")[0] in forbidden_imports
        ):
            fail("runner imports a network-capable module")
    if "subprocess" in source or "os.system" in source:
        fail("runner contains a subprocess escape hatch")
    for required in (
        'os.environ["HF_HUB_OFFLINE"] = "1"',
        'os.environ["TRANSFORMERS_OFFLINE"] = "1"',
        "require_isolated_python()",
        "require_virtual_environment()",
        "validate_dependency_provenance(",
        "validate_gpu_target(torch)",
        '"gcnArchName"',
        "model.model(",
        "model.lm_head(final_hidden)",
        "local_files_only=True",
        "trust_remote_code=False",
        "use_safetensors=True",
        'attn_implementation="sdpa"',
        "repeated_logits != logits",
        "rename_noreplace(",
    ):
        if required not in source:
            fail(f"runner lost required source pattern: {required}")
    if "http://" in source or "https://" in source:
        fail("runner contains a network location")


def main() -> None:
    actual = {path.name for path in ROOT.iterdir()}
    if actual != EXPECTED_FILES:
        fail(f"package file roster drifted: {sorted(actual ^ EXPECTED_FILES)}")
    dependency_policy()
    implementation_policy()
    protocol_policy()
    source_policy()
    print("reference policy: PASS")


if __name__ == "__main__":
    main()
