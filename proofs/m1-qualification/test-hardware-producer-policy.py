#!/usr/bin/env python3
"""Exercise all 58 planner-bound M1 MI300X hardware producers."""

from __future__ import annotations

import contextlib
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
from types import ModuleType
from typing import Any, Callable, NoReturn


VALIDATOR_PROTOCOL = "ferric.m1-validator.hardware-transcript.v1"
TEST_PROTOCOL = "ferric.m1.mi300x-hardware-test.v1"
GPU_UNIQUE_ID = 0x123456789ABCDEF0
TOOL_SOURCE_PATHS = {
    "cargo_lock": "Cargo.lock",
    "hardware_harness": "crates/ferric-engine/src/bin/ferric-m1-hardware-harness.rs",
    "package_manifest": "crates/ferric-engine/Cargo.toml",
    "packet_execution": "crates/ferric-engine/src/m1_packet_diagnostic_execution.rs",
    "persisted_kernel_artifacts": "crates/ferric-engine/src/persisted_kernel_artifacts.rs",
}
TCB = (
    ("tcb.compiler", "Compiler"),
    ("tcb.hardware", "Hardware"),
    ("tcb.runtime", "Runtime"),
)


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def compact_bytes(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")


def amd_smi_uuid(gpu_unique_id: int) -> str:
    return (
        f"{gpu_unique_id >> 56:02x}ff74a1-0000-1000-80"
        f"{(gpu_unique_id >> 48) & 0xFF:02x}-"
        f"{gpu_unique_id & 0x0000FFFFFFFFFFFF:012x}"
    )


def read_json(path: Path) -> dict[str, Any]:
    raw = path.read_bytes()
    value = json.loads(raw)
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"fixture is not canonical JSON: {path}")
    return value


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_bytes(canonical_bytes(value))


def command(arguments: list[str], description: str) -> str:
    result = subprocess.run(
        arguments,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=300,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"{description} failed (status {result.returncode}):\n{result.stdout}")
    return result.stdout.strip()


def clone_at(source: Path, destination: Path, revision: str = "HEAD") -> None:
    command(
        ["git", "clone", "--quiet", "--no-hardlinks", str(source), str(destination)],
        f"clone {source}",
    )
    command(
        ["git", "-C", str(destination), "checkout", "--quiet", "--detach", revision],
        f"check out {revision}",
    )


def commit_fixture(repository: Path) -> None:
    command(["git", "-C", str(repository), "add", "-A"], "stage fixture")
    command(
        [
            "git",
            "-C",
            str(repository),
            "-c",
            "user.name=M1 Hardware Producer Policy",
            "-c",
            "user.email=m1-hardware-producer@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "add M1 hardware producer",
        ],
        "commit fixture",
    )


def invoke(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    plan: Path,
    harness: Path,
    kernels: Path,
    environment: Path,
    binding_id: str,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            "-I",
            str(producer),
            str(ferric),
            str(fe2o3),
            str(plan),
            str(harness),
            str(kernels),
            str(environment),
            binding_id,
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=1200,
        env={"PATH": os.environ.get("PATH", "")},
    )


def run_planner(planner: Path, ferric: Path, fe2o3: Path, output: Path) -> None:
    result = subprocess.run(
        [sys.executable, "-I", str(planner), str(ferric), str(fe2o3), str(output)],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=300,
        env={"PATH": os.environ.get("PATH", "")},
    )
    if result.returncode != 0:
        fail(f"planner rejected hardware producer fixture:\n{result.stdout}")


def materialize_tcb(producer: Path, ferric: Path, fe2o3: Path, plan: Path) -> None:
    for subject, _ in TCB:
        result = subprocess.run(
            [
                sys.executable,
                "-I",
                str(producer),
                str(ferric),
                str(fe2o3),
                str(plan),
                subject,
            ],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=300,
            env={"PATH": os.environ.get("PATH", "")},
        )
        if result.returncode != 0:
            fail(f"TCB prerequisite failed for {subject}:\n{result.stdout}")


def hardware_slots(plan: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        slot
        for slot in plan["binding_slots"]
        if slot["binding"]["evidence_kind"] == "hardware-test"
    ]


def outer_tcb(plan_root: Path) -> list[dict[str, str]]:
    result = []
    for subject, kind in TCB:
        artifact_id = f"artifact.{subject}"
        raw = (plan_root / "artifacts" / f"{artifact_id}.tcb-report.json").read_bytes()
        result.append(
            {
                "artifact_id": artifact_id,
                "id": subject,
                "identity_sha256": digest_bytes(raw),
                "kind": kind,
            }
        )
    return result


def validate_all(
    validator: Path,
    plan_root: Path,
    plan: dict[str, Any],
    ferric: Path,
) -> None:
    resolutions = {row["id"]: row for row in plan["path_resolutions"]}
    tcb = outer_tcb(plan_root)
    observed = []
    for slot in hardware_slots(plan):
        binding = slot["binding"]
        artifact = slot["expected_artifact"]
        report_path = plan_root / artifact["path"]
        raw = report_path.read_bytes()
        context = {
            "artifact": {
                **artifact,
                "sha256": digest_bytes(raw),
                "size_bytes": len(raw),
            },
            "artifact_absolute_path": str(report_path),
            "binding": binding,
            "format": "ferric.m1-evidence-index.v1",
            "path_resolution": resolutions[binding["path_id"]],
            "requirements_sha256": plan["requirements"]["sha256"],
            "sources": plan["sources"],
            "subject": f"binding:{binding['id']}",
            "tcb": tcb,
        }
        payload = compact_bytes(context)
        result = subprocess.run(
            [sys.executable, "-I", str(validator), VALIDATOR_PROTOCOL],
            cwd=ferric,
            check=False,
            input=payload + b"\n",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=60,
            env={"PATH": os.environ.get("PATH", "")},
        )
        expected = (
            f"PASS: {VALIDATOR_PROTOCOL} artifact_sha256={digest_bytes(raw)} "
            f"context_sha256={digest_bytes(payload)}\n"
        ).encode("ascii")
        if result.returncode != 0 or result.stdout != expected:
            fail(
                f"trusted validator rejected {binding['id']}: "
                f"exit={result.returncode}, output={result.stdout!r}"
            )
        observed.append(binding["id"])
    if (
        len(observed) != 58
        or digest_bytes(("\n".join(observed) + "\n").encode("ascii"))
        != "50ab14c739eb88d8ded5becc86ccf5420386e905ab2d583463da4dfbf82f17cb"
    ):
        fail("validated hardware binding roster drifted")


def make_environment(path: Path) -> None:
    write_json(
        path,
        {
            "device": {
                "device_count": 1,
                "device_uuid": amd_smi_uuid(GPU_UNIQUE_ID),
                "marketing_name": "AMD Instinct MI300X",
                "pci_bdf": "0000:41:00.0",
                "processor": "gfx942",
                "vendor_id": "1002",
                "xnack": "disabled",
            },
            "driver": {
                "module_sha256": digest_bytes(b"measured-amdgpu-module"),
                "name": "amdgpu",
                "version": "6.14.14-2061179.el9",
            },
            "firmware": {
                "bundle_sha256": digest_bytes(b"measured-amdgpu-firmware"),
                "package_version": "20250613-2.el9",
            },
            "format": "FERRIC-M1-HARDWARE-ENVIRONMENT-V1",
            "gpu_unique_id": GPU_UNIQUE_ID,
            "rocm": {
                "installation_sha256": digest_bytes(b"measured-rocm-7.0.0"),
                "version": "7.0.0",
            },
            "target": "gfx942:xnack-",
        },
    )


def make_harness(
    directory: Path,
    counter: Path,
    tool_source_sha256s: dict[str, str],
    mode_file: Path,
) -> Path:
    directory.mkdir(parents=True)
    harness = directory / "ferric-m1-hardware-harness"
    source = f"""#!/usr/bin/env python3
import hashlib, json, pathlib, sys
mode = pathlib.Path({str(mode_file)!r}).read_text(encoding="ascii").strip()
request = json.loads(sys.stdin.buffer.read())
environment = json.loads(pathlib.Path(sys.argv[2]).read_bytes())
if not sys.argv[1].startswith("/proc/self/fd/") or not sys.argv[2].startswith("/proc/self/fd/"):
    raise SystemExit("kernel artifacts or environment were not descriptor-bound")
pathlib.Path(sys.argv[1], "m1-kernel-artifacts.manifest.bin").read_bytes()
if mode == "exit":
    print("injected harness failure", file=sys.stderr)
    raise SystemExit(7)
case = request["case"]
manifest = hashlib.sha256(b"authenticated-manifest-identity").hexdigest()
catalog = hashlib.sha256(b"authenticated-program-catalog").hexdigest()
generation = 7
observation = (
    "ferric-m1-k7-observation-v1|" + case["binding_sha256"] + "|" +
    case["case_id"] + "|" + case["procedure_sha256"] + "|" + manifest +
    "|" + catalog + "|" + environment["device"]["device_uuid"] + "|" +
    environment["device"]["pci_bdf"] + "|7|10,11,12,13,14\\n"
)
result = {{
    "case_result": {{
        "binding_sha256": case["binding_sha256"],
        "case_id": case["case_id"],
        "completion_count": 1,
        "generation": generation,
        "gpu_observation_sha256": hashlib.sha256(observation.encode("ascii")).hexdigest(),
        "grid": [64, 1, 1],
        "launch_count": 1,
        "output_tokens": [10, 11, 12, 13, 14],
        "output_verified": True,
        "procedure_sha256": case["procedure_sha256"],
        "program": "k7-speculative-token-assembly-s1k4",
        "queue_released": True,
        "workgroup": [64, 1, 1]
    }},
    "device": environment["device"],
    "environment": {{
        "driver": environment["driver"],
        "firmware": environment["firmware"],
        "rocm": environment["rocm"]
    }},
    "finished_at_utc": "2026-08-24T12:00:01Z",
    "format": "FERRIC-M1-HARDWARE-HARNESS-RESULT-V1",
    "gpu_work_completed": True,
    "gpu_work_submitted": True,
    "kernel_catalog_sha256": catalog,
    "kernel_manifest_sha256": manifest,
    "no_gpu_work": False,
    "protocol": "ferric.m1.mi300x-hardware-test.v1",
    "run_id": "run." + case["case_id"].removeprefix("case.k7."),
    "started_at_utc": "2026-08-24T12:00:00Z",
    "status": "pass",
    "target": "gfx942:xnack-",
    "tool_source_sha256s": {tool_source_sha256s!r},
    "tool_version": "0.1.0"
}}
if mode == "device":
    result["device"]["pci_bdf"] = "0000:42:00.0"
elif mode == "no-gpu":
    result["gpu_work_submitted"] = False
elif mode == "echo":
    result["case_result"]["binding_sha256"] = hashlib.sha256(b"wrong").hexdigest()
elif mode == "observation":
    result["case_result"]["gpu_observation_sha256"] = hashlib.sha256(b"wrong").hexdigest()
elif mode == "extra":
    result["unexpected"] = True
elif mode == "tool-source":
    result["tool_source_sha256s"]["cargo_lock"] = hashlib.sha256(b"wrong").hexdigest()
pathlib.Path({str(counter)!r}).open("a", encoding="ascii").write(case["case_id"] + "\\n")
raw = (json.dumps(result, ensure_ascii=True, indent=2, sort_keys=True) + "\\n").encode("ascii")
if mode == "noncanonical":
    raw += b"\\n"
sys.stdout.buffer.write(raw)
"""
    harness.write_text(source, encoding="ascii")
    harness.chmod(0o700)
    return harness


def expect_failure(
    producer: Path,
    ferric: Path,
    fe2o3: Path,
    baseline: Path,
    root: Path,
    harness: Path,
    kernels: Path,
    environment: Path,
    binding_id: str,
    label: str,
    expected: str,
) -> None:
    plan = root / label
    shutil.copytree(baseline, plan)
    result = invoke(
        producer, ferric, fe2o3, plan, harness, kernels, environment, binding_id
    )
    if result.returncode == 0 or expected not in result.stdout:
        fail(
            f"producer accepted hostile {label}; expected {expected!r}:\n{result.stdout}"
        )
    reports = list((plan / "artifacts").glob("*.hardware-transcript.json"))
    if reports:
        fail(f"hostile {label} left a report completion marker")


def load_producer(path: Path) -> ModuleType:
    sys.dont_write_bytecode = True
    specification = importlib.util.spec_from_file_location("hardware_producer", path)
    if specification is None or specification.loader is None:
        fail("cannot load hardware producer race policy")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def expect_direct_failure(action: Callable[[], None], expected: str) -> None:
    errors = io.StringIO()
    try:
        with contextlib.redirect_stderr(errors):
            action()
    except SystemExit:
        pass
    else:
        fail("hardware producer race policy accepted a replacement")
    if expected not in errors.getvalue():
        fail(f"race failure mismatch, expected {expected!r}: {errors.getvalue()}")


def publication_policy(root: Path, producer: Path) -> int:
    module = load_producer(producer)
    cases = 0
    for mode in (
        "success",
        "report-failure",
        "roster-overwrite",
        "attacker-replacement",
        "plan-parent-replacement",
    ):
        plan = root / f"publication-{mode}"
        plan.mkdir(mode=0o700)
        for name in ("artifacts", "hardware-rosters", "hardware-transcripts"):
            (plan / name).mkdir(mode=0o700)
        plan_custody = module.authenticate_absolute_directory(
            plan, "publication plan", private=True
        )
        plan_fd = module.directory_custody_fd(plan_custody)
        artifact_fd = module.open_private_directory_at(
            plan_fd, "artifacts", "artifacts"
        )
        roster_fd = module.open_private_directory_at(
            plan_fd, "hardware-rosters", "rosters"
        )
        transcript_fd = module.open_private_directory_at(
            plan_fd, "hardware-transcripts", "transcripts"
        )
        calls: list[str] = []
        custody_calls = 0
        rebound_plan: Path | None = None
        original = module.create_new_file_at

        def intercept(
            directory_fd: int, name: str, value: bytes, description: str
        ) -> int:
            calls.append(description)
            if mode == "report-failure" and len(calls) == 3:
                module.fail("injected report-last failure")
            if mode == "attacker-replacement" and len(calls) == 3:
                transcript = plan / "hardware-transcripts" / f"{artifact_id}.json"
                transcript.rename(transcript.with_suffix(".owned"))
                transcript.write_bytes(b"attacker replacement\n")
                transcript.chmod(0o600)
                module.fail("injected attacker replacement")
            return original(directory_fd, name, value, description)

        artifact_id = "artifact.binding.00019"
        if mode == "roster-overwrite":
            target = plan / "hardware-rosters" / f"{artifact_id}.json"
            target.write_bytes(b"hostile\n")
            target.chmod(0o600)
        module.create_new_file_at = intercept
        try:

            def custody_check() -> None:
                nonlocal custody_calls, rebound_plan
                custody_calls += 1
                if mode == "plan-parent-replacement" and custody_calls == 2:
                    rebound_plan = plan.with_suffix(".owned")
                    plan.rename(rebound_plan)
                    plan.mkdir(mode=0o700)
                module.revalidate_absolute_directory(plan_custody, private=True)

            def action() -> None:
                module.publish_hardware_transcript(
                    plan_custody,
                    plan_fd,
                    artifact_fd,
                    roster_fd,
                    transcript_fd,
                    artifact_id,
                    b'{"roster":true}\n',
                    b'{"transcript":true}\n',
                    b'{"report":true}\n',
                    custody_check,
                )

            if mode == "success":
                action()
                if calls != [
                    "M1 hardware case roster",
                    "M1 hardware run transcript",
                    "M1 hardware-transcript report",
                ]:
                    fail("hardware report was not published last")
            else:
                expected = (
                    "injected report-last failure"
                    if mode == "report-failure"
                    else (
                        "preexisting output"
                        if mode == "roster-overwrite"
                        else (
                            "cannot remove replaced failed M1 hardware run transcript"
                            if mode == "attacker-replacement"
                            else "was replaced after it was opened"
                        )
                    )
                )
                expect_direct_failure(action, expected)
                report = plan / "artifacts" / f"{artifact_id}.hardware-transcript.json"
                transcript = plan / "hardware-transcripts" / f"{artifact_id}.json"
                if report.exists() or (
                    transcript.exists() and mode != "attacker-replacement"
                ):
                    fail(f"{mode} left a false hardware completion")
                if (
                    mode in ("report-failure", "attacker-replacement")
                    and (plan / "hardware-rosters" / f"{artifact_id}.json").exists()
                ):
                    fail("report failure did not roll back its exact roster")
                if (
                    mode == "attacker-replacement"
                    and transcript.read_bytes() != b"attacker replacement\n"
                ):
                    fail("rollback removed or altered an attacker-replaced inode")
                if mode == "plan-parent-replacement" and (
                    rebound_plan is None
                    or (
                        rebound_plan / "hardware-rosters" / f"{artifact_id}.json"
                    ).exists()
                ):
                    fail("plan parent replacement did not roll back the exact roster")
            cases += 1
        finally:
            module.create_new_file_at = original
            os.close(transcript_fd)
            os.close(roster_fd)
            os.close(artifact_fd)
            module.close_absolute_directory(plan_custody)
    return cases


def custody_races(root: Path, producer: Path, kernels: Path) -> int:
    module = load_producer(producer)
    cases = 0

    def absolute_file_rebind(label: str, filename: str, *, executable: bool) -> None:
        nonlocal cases
        parent = root / f"{label}-parent"
        parent.mkdir()
        target = parent / filename
        target.write_bytes(b"held\n")
        target.chmod(0o700 if executable else 0o600)
        custody = module.authenticate_absolute_component_file(
            target, 1024, label, executable=executable
        )
        old_parent = root / f"{label}-parent-old"
        parent.rename(old_parent)
        parent.mkdir()
        replacement = parent / filename
        replacement.write_bytes(b"held\n")
        replacement.chmod(0o700 if executable else 0o600)
        try:
            expect_direct_failure(
                lambda: module.revalidate_component_file(custody),
                "was replaced after it was opened",
            )
        finally:
            module.close_component_file(custody)
        cases += 1

    def absolute_file_parent_symlink(
        label: str, filename: str, *, executable: bool
    ) -> None:
        nonlocal cases
        real_parent = root / f"{label}-real"
        real_parent.mkdir()
        target = real_parent / filename
        target.write_bytes(b"held\n")
        target.chmod(0o700 if executable else 0o600)
        linked_parent = root / f"{label}-link"
        linked_parent.symlink_to(real_parent, target_is_directory=True)
        expect_direct_failure(
            lambda: module.authenticate_absolute_component_file(
                linked_parent / filename,
                1024,
                label,
                executable=executable,
            ),
            "is unavailable",
        )
        cases += 1

    absolute_file_rebind(
        "harness parent custody",
        "ferric-m1-hardware-harness",
        executable=True,
    )
    absolute_file_parent_symlink(
        "harness parent symlink",
        "ferric-m1-hardware-harness",
        executable=True,
    )
    absolute_file_rebind(
        "environment parent custody", "hardware-environment.json", executable=False
    )
    absolute_file_parent_symlink(
        "environment parent symlink",
        "hardware-environment.json",
        executable=False,
    )

    repo = root / "tool-source-repo"
    (repo / "sources").mkdir(parents=True)
    (repo / "sources/source.rs").write_bytes(b"source\n")
    repo_custody = module.authenticate_absolute_directory(repo, "tool source repo")
    repo_fd = module.directory_custody_fd(repo_custody)
    tool = module.authenticate_relative_component_file(
        repo_fd, "sources/source.rs", 1024, "tool source"
    )
    (repo / "sources").rename(repo / "sources-old")
    (repo / "sources").mkdir()
    (repo / "sources/source.rs").write_bytes(b"source\n")
    try:
        expect_direct_failure(
            lambda: module.revalidate_component_file(tool),
            "was replaced after it was opened",
        )
    finally:
        module.close_component_file(tool)
        module.close_absolute_directory(repo_custody)
    cases += 1

    static_repo = root / "tool-source-static-repo"
    (static_repo / "real").mkdir(parents=True)
    (static_repo / "real/source.rs").write_bytes(b"source\n")
    (static_repo / "link").symlink_to(static_repo / "real", target_is_directory=True)
    static_repo_custody = module.authenticate_absolute_directory(
        static_repo, "static tool source repo"
    )
    try:
        expect_direct_failure(
            lambda: module.authenticate_relative_component_file(
                module.directory_custody_fd(static_repo_custody),
                "link/source.rs",
                1024,
                "static tool source",
            ),
            "is unavailable",
        )
    finally:
        module.close_absolute_directory(static_repo_custody)
    cases += 1

    custody = module.authenticate_kernel_tree(kernels)
    injected = kernels / "objects/injected"
    injected.write_bytes(b"injected")
    try:
        expect_direct_failure(
            lambda: module.revalidate_kernel_tree(custody),
            "membership changed",
        )
    finally:
        module.close_kernel_tree(custody)
        injected.unlink()
    cases += 1

    kernel_parent = root / "kernel-parent"
    kernel_target = kernel_parent / "kernels"
    shutil.copytree(kernels, kernel_target)
    kernel_custody = module.authenticate_kernel_tree(kernel_target)
    kernel_parent.rename(root / "kernel-parent-old")
    kernel_parent.mkdir()
    shutil.copytree(kernels, kernel_target)
    try:
        expect_direct_failure(
            lambda: module.revalidate_kernel_tree(kernel_custody),
            "was replaced after it was opened",
        )
    finally:
        module.close_kernel_tree(kernel_custody)
    cases += 1

    real_kernel_parent = root / "kernel-static-real"
    shutil.copytree(kernels, real_kernel_parent / "kernels")
    kernel_link = root / "kernel-static-link"
    kernel_link.symlink_to(real_kernel_parent, target_is_directory=True)
    expect_direct_failure(
        lambda: module.authenticate_kernel_tree(kernel_link / "kernels"),
        "is unavailable",
    )
    cases += 1

    plan_parent = root / "plan-parent"
    plan = plan_parent / "plan"
    plan.mkdir(parents=True, mode=0o700)
    plan.chmod(0o700)
    plan_custody = module.authenticate_absolute_directory(
        plan, "plan parent custody", private=True
    )
    plan_parent.rename(root / "plan-parent-old")
    replacement_plan = plan_parent / "plan"
    replacement_plan.mkdir(parents=True, mode=0o700)
    replacement_plan.chmod(0o700)
    try:
        expect_direct_failure(
            lambda: module.revalidate_absolute_directory(plan_custody, private=True),
            "was replaced after it was opened",
        )
    finally:
        module.close_absolute_directory(plan_custody)
    cases += 1

    real_plan_parent = root / "plan-static-real"
    real_plan = real_plan_parent / "plan"
    real_plan.mkdir(parents=True, mode=0o700)
    real_plan.chmod(0o700)
    plan_link = root / "plan-static-link"
    plan_link.symlink_to(real_plan_parent, target_is_directory=True)
    expect_direct_failure(
        lambda: module.authenticate_absolute_directory(
            plan_link / "plan", "plan parent symlink", private=True
        ),
        "is unavailable",
    )
    cases += 1

    component_root = root / "single-component"
    component_root.mkdir()
    component_fd = os.open(component_root, os.O_RDONLY | os.O_DIRECTORY)
    try:
        expect_direct_failure(
            lambda: module.open_regular_at(
                component_fd, "nested/name", "single-component policy"
            ),
            "must be a single path component",
        )
    finally:
        os.close(component_fd)
    cases += 1
    return cases


def main() -> None:
    if len(sys.argv) != 3:
        fail(f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_OBJECT_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    fe2o3_source = Path(sys.argv[2]).resolve(strict=True)
    source = (
        repo / "proofs/m1-qualification/produce-hardware-transcript.py"
    ).read_text(encoding="ascii")
    if "validate-hardware-transcript.py" in source:
        fail("hardware producer must not invoke the trusted validator")
    required_custody_fragments = (
        'required = ("O_NOFOLLOW", "O_DIRECTORY", "O_CLOEXEC")',
        "with os.scandir(directory_fd) as entries:",
        "procedure_file = authenticate_relative_component_file(",
        "tool_source_sha256s, tool_source_files = authenticate_tool_sources(ferric_fd)",
        'env={"PATH": os.environ.get("PATH", "")}',
        "harness_file = authenticate_absolute_component_file(",
        "environment_file = authenticate_absolute_component_file(",
        "kernel_fd = kernel_root_fd(kernel)",
        "revalidate_absolute_directory(plan_custody, private=True)",
    )
    if any(fragment not in source for fragment in required_custody_fragments):
        fail("hardware producer descriptor-custody structure drifted")
    kernel_custody_source = source.split("def authenticate_kernel_tree", 1)[1].split(
        "def digest_file", 1
    )[0]
    if any(
        fragment in kernel_custody_source
        for fragment in ("os.walk(", "Path.walk(", ".lstat(", "os.scandir(")
    ):
        fail("kernel custody regained path-based enumeration or inspection")
    with tempfile.TemporaryDirectory(prefix="ferric-m1-hardware-producer-") as raw:
        root = Path(raw)
        ferric = root / "ferric"
        clone_at(repo, ferric)
        shutil.copytree(
            repo / "proofs/m1-qualification",
            ferric / "proofs/m1-qualification",
            dirs_exist_ok=True,
            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"),
        )
        for relative in (
            "proofs/check-m1-evidence-index.py",
            "proofs/m1/evidence/validate-hardware-transcript.py",
            "proofs/m1/evidence/validate-qualification-receipt.py",
        ):
            shutil.copy2(repo / relative, ferric / relative)
        for relative in TOOL_SOURCE_PATHS.values():
            destination = ferric / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(repo / relative, destination)
        counter = root / "launches"
        mode_file = root / "harness-mode"
        mode_file.write_text("valid", encoding="ascii")
        tool_source_sha256s = {
            key: digest_bytes((ferric / relative).read_bytes())
            for key, relative in TOOL_SOURCE_PATHS.items()
        }
        harness = make_harness(
            root / "valid-harness", counter, tool_source_sha256s, mode_file
        )
        procedure_path = ferric / "proofs/m1-qualification/hardware-k7-procedure.json"
        procedure = read_json(procedure_path)
        harness_bytes = harness.read_bytes()
        procedure["harness_binary"] = {
            "sha256": digest_bytes(harness_bytes),
            "size_bytes": len(harness_bytes),
        }
        write_json(procedure_path, procedure)
        if command(
            [
                "git",
                "-C",
                str(ferric),
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
            ],
            "inspect Ferric fixture",
        ):
            commit_fixture(ferric)
        cargo = (ferric / "Cargo.toml").read_text(encoding="utf-8")
        marker = 'fe2o3-amdhsa-loader = { git = "https://github.com/harsh-nod/fe2o3.git", rev = "'
        if cargo.count(marker) != 1:
            fail("cannot locate exact fe2o3 revision")
        revision = cargo.split(marker, 1)[1].split('"', 1)[0]
        fe2o3 = root / "fe2o3"
        clone_at(fe2o3_source, fe2o3, revision)

        planner = ferric / "proofs/m1-qualification/planner.py"
        producer = ferric / "proofs/m1-qualification/produce-hardware-transcript.py"
        tcb_producer = ferric / "proofs/m1-qualification/produce-tcb-report.py"
        validator = ferric / "proofs/m1/evidence/validate-hardware-transcript.py"
        baseline = root / "baseline"
        run_planner(planner, ferric, fe2o3, baseline)
        materialize_tcb(tcb_producer, ferric, fe2o3, baseline)
        plan = read_json(baseline / "plan.json")
        slots = hardware_slots(plan)
        if len(slots) != 58:
            fail("planner did not expose exactly 58 hardware bindings")

        kernels = root / "kernels"
        (kernels / "objects/sha256").mkdir(parents=True)
        (kernels / "m1-kernel-artifacts.manifest.bin").write_bytes(b"manifest\n")
        (kernels / "objects/sha256/object").write_bytes(b"code-object\n")
        environment = root / "hardware-environment.json"
        make_environment(environment)

        canonical = root / "canonical"
        shutil.copytree(baseline, canonical)
        with ThreadPoolExecutor(max_workers=4) as executor:
            pending = {
                executor.submit(
                    invoke,
                    producer,
                    ferric,
                    fe2o3,
                    canonical,
                    harness,
                    kernels,
                    environment,
                    slot["binding"]["id"],
                ): (ordinal, slot["binding"]["id"])
                for ordinal, slot in enumerate(slots, 1)
            }
            for future in as_completed(pending):
                ordinal, binding_id = pending[future]
                result = future.result()
                if result.returncode != 0:
                    fail(
                        f"producer rejected binding {ordinal}/58 {binding_id}:\n"
                        f"{result.stdout}"
                    )
        synthetic_invocations = counter.read_text(encoding="ascii").splitlines()
        expected_cases = [
            f"case.k7.{slot['binding']['id'].replace('.', '-')}" for slot in slots
        ]
        if (
            sorted(synthetic_invocations) != sorted(expected_cases)
            or len(set(synthetic_invocations)) != 58
        ):
            fail("public producer did not issue one synthetic invocation per binding")
        validate_all(validator, canonical, plan, ferric)
        for slot in slots:
            artifact_id = slot["binding"]["artifact_id"]
            paths = (
                canonical / f"hardware-rosters/{artifact_id}.json",
                canonical / f"hardware-transcripts/{artifact_id}.json",
                canonical / f"artifacts/{artifact_id}.hardware-transcript.json",
            )
            if any(stat.S_IMODE(path.stat().st_mode) != 0o600 for path in paths):
                fail("hardware triple is not exact owner-private publication")

        hostile_root = root / "hostile"
        hostile_root.mkdir()
        hostile_count = 0
        first = slots[0]["binding"]["id"]
        failures = {
            "device": "device or measured environment disagrees",
            "no-gpu": "singleton K7 result drifted",
            "echo": "singleton K7 result drifted",
            "observation": "observation identity drifted",
            "extra": "result fields drifted",
            "noncanonical": "not a canonical JSON object",
            "exit": "hardware harness failed with status 7",
            "tool-source": "source identities disagree",
        }
        for mode, expected in failures.items():
            mode_file.write_text(mode, encoding="ascii")
            expect_failure(
                producer,
                ferric,
                fe2o3,
                baseline,
                hostile_root,
                harness,
                kernels,
                environment,
                first,
                f"harness-{mode}",
                expected,
            )
            hostile_count += 1
        mode_file.write_text("valid", encoding="ascii")

        unreviewed_dir = root / "unreviewed-harness"
        unreviewed_dir.mkdir()
        unreviewed_harness = unreviewed_dir / "ferric-m1-hardware-harness"
        unreviewed_harness.write_bytes(harness_bytes + b"\n")
        unreviewed_harness.chmod(0o700)
        before_unreviewed = counter.read_text(encoding="ascii")
        expect_failure(
            producer,
            ferric,
            fe2o3,
            baseline,
            hostile_root,
            unreviewed_harness,
            kernels,
            environment,
            first,
            "unreviewed-harness",
            "reviewed procedure binary pin",
        )
        if counter.read_text(encoding="ascii") != before_unreviewed:
            fail("producer invoked an unreviewed hardware harness")
        hostile_count += 1

        bad_environment = root / "bad-environment.json"
        bad = read_json(environment)
        bad["device"]["device_uuid"] = "00000000-0000-0000-0000-000000000000"
        write_json(bad_environment, bad)
        expect_failure(
            producer,
            ferric,
            fe2o3,
            baseline,
            hostile_root,
            harness,
            kernels,
            bad_environment,
            first,
            "placeholder-device",
            "selected device is not exactly",
        )
        hostile_count += 1

        mismatched_uuid_environment = root / "mismatched-uuid-environment.json"
        mismatched = read_json(environment)
        mismatched["device"]["device_uuid"] = amd_smi_uuid(GPU_UNIQUE_ID ^ 1)
        write_json(mismatched_uuid_environment, mismatched)
        expect_failure(
            producer,
            ferric,
            fe2o3,
            baseline,
            hostile_root,
            harness,
            kernels,
            mismatched_uuid_environment,
            first,
            "mismatched-derived-device",
            "UUID does not match its KFD GPU unique ID",
        )
        hostile_count += 1

        hardlink_dir = root / "hardlink-harness"
        hardlink_dir.mkdir()
        hardlink = hardlink_dir / "ferric-m1-hardware-harness"
        os.link(harness, hardlink)
        expect_failure(
            producer,
            ferric,
            fe2o3,
            baseline,
            hostile_root,
            hardlink,
            kernels,
            environment,
            first,
            "hardlink-harness",
            "metadata is outside",
        )
        hostile_count += 1

        hostile_count += publication_policy(hostile_root, producer)
        hostile_count += custody_races(root, producer, kernels)
        if command(
            [
                "git",
                "-C",
                str(ferric),
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
            ],
            "recheck Ferric fixture",
        ) or command(
            [
                "git",
                "-C",
                str(fe2o3),
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
            ],
            "recheck fe2o3 fixture",
        ):
            fail("hardware production dirtied an exact source repository")
    print(
        "PASS: M1 hardware producer emitted and validated all 58 triples with "
        "58 binding-local synthetic harness invocations and rejected "
        f"{hostile_count} hostile/race cases"
    )


if __name__ == "__main__":
    main()
