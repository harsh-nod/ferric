#!/usr/bin/env python3
"""Assemble the source-bound M1 qualification receipt from held gate inputs."""

from __future__ import annotations

from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
import tempfile
from typing import Any, BinaryIO, NoReturn


PLAN_FORMAT = "FERRIC-M1-EVIDENCE-PLAN-V1"
WORK_FORMAT = "FERRIC-M1-EVIDENCE-WORK-QUEUE-V1"
PLAN_AUTHORITY = "planning-only-no-evidence"
PLAN_NONCLAIM = (
    "This bundle allocates external M1 evidence work only. It is not an evidence "
    "index, qualification receipt, validation result, or M1 closure claim."
)
ALLOCATION_SHA256 = "948ad3023df7ad4b1313ed865b54464f63b6bad9406f1510c85e60f9db055bd6"
INTAKE_FORMAT = "FERRIC-M1-QUALIFICATION-RUN-INTAKE-V1"
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
REPORT_FORMAT = "FERRIC-M1-QUALIFICATION-RECEIPT-V1"
TRANSCRIPT_FORMAT = "FERRIC-M1-QUALIFICATION-TRANSCRIPT-V1"
QUALIFICATION_PROTOCOL = "ferric.m1.qualification.v1"
VALIDATOR_PROTOCOL = "ferric.m1-validator.qualification-receipt.v1"
PRE_RECEIPT_PROTOCOL = "ferric.m1-pre-receipt-gate.v1"
COMPLETION_FORMAT = "FERRIC-M1-EVIDENCE-WORK-COMPLETION-V1"
COMPLETION_AUTHORITY = "authenticated-qualification-work-completion"
AUTHORITY = "m1-qualification-receipt-only"
NONCLAIM = (
    "This producer derives a candidate from exact planner output, executes the "
    "source-pinned pre-receipt validators, and assembles their identity-bound "
    "receipt. It does not generate evidence, alter repository requirements, or "
    "claim M1 closure."
)
TARGET = "gfx942:xnack-"
TARGET_VALUE = {
    "architecture": "gfx942",
    "device_count": 1,
    "feature": "xnack-",
    "triple": "amdgcn-amd-amdhsa",
}
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
GATE_IDS = (
    "evidence-index",
    "hardware",
    "performance",
    "proof",
    "quality",
    "source-closure",
    "validators",
)
MEASURED_TOOL_IDS = ("compiler.cargo", "compiler.rustc")
TOOL_IDS = (
    "compiler.cargo",
    "compiler.rustc",
    "compiler.verus",
    "runtime.python",
    "validator.evidence-index",
    "validator.qualification-receipt",
)
VALIDATOR_IDS = (
    "artifact-identity",
    "canonical-structure-check",
    "external-contract",
    "fe2o3-contract",
    "hardware-test",
    "independent-validator",
    "negative-mutation",
    "performance-gate",
    "qualification-receipt",
    "tcb-report",
    "unsupported-rationale",
    "verus-theorem",
)
PLAN_KEYS = {
    "allocation_sha256",
    "authority",
    "binding_slots",
    "counts",
    "fe2o3_pins",
    "finalization",
    "format",
    "nonclaim",
    "obligation_slots",
    "path_resolutions",
    "planner_sha256",
    "requirements",
    "source_closures",
    "sources",
    "target",
    "trusted_validators",
}
WORK_KEYS = {
    "authority",
    "counts",
    "format",
    "items",
    "nonclaim",
    "plan_path",
    "plan_sha256",
    "status",
}
INTAKE_KEYS = {
    "candidate_index_relative_path",
    "environment",
    "format",
    "run_id",
    "tools",
}
INTAKE_TOOL_KEYS = {"binary_absolute_path", "id"}
COMPLETION_KEYS = {
    "authority",
    "candidate_index_sha256",
    "completed_item_ids",
    "counts",
    "format",
    "gate_roster_sha256",
    "plan_path",
    "plan_sha256",
    "queue_path",
    "queue_sha256",
    "status",
}
INDEX_KEYS = {
    "artifacts",
    "evidence_bindings",
    "format",
    "obligations",
    "path_resolutions",
    "requirements_sha256",
    "sources",
    "tcb",
}
ENVIRONMENT_KEYS = {"device", "driver", "firmware", "host", "rocm"}
DEVICE_KEYS = {
    "device_count",
    "device_uuid",
    "marketing_name",
    "pci_bdf",
    "processor",
    "vendor_id",
    "xnack",
}
DRIVER_KEYS = {"module_sha256", "name", "version"}
FIRMWARE_KEYS = {"bundle_sha256", "package_version"}
HOST_KEYS = {"kernel_sha256", "machine", "os_release_sha256"}
ROCM_KEYS = {"installation_sha256", "version"}
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
UTC_TIMESTAMP = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z\Z")
UUID = re.compile(
    r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-"
    r"[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z"
)
PCI_BDF = re.compile(r"[0-9a-f]{4}:[0-9a-f]{2}:[0-9a-f]{2}\.[0-7]\Z")
PYTHON_VERSION = re.compile(r"3\.[0-9]+\.[0-9]+\Z")
MAX_JSON_BYTES = 32_000_000
MAX_FILE_BYTES = 64_000_000
MAX_TOTAL_ARTIFACT_BYTES = 512_000_000


JsonObject = dict[str, Any]
HeldInput = tuple[Path, bytes, os.stat_result]
PublishedFile = tuple[int, str, Path, tuple[int, ...]]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def canonical_digest(value: Any) -> str:
    return digest_bytes(
        json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
    )


def exact_keys(value: Any, expected: set[str], description: str) -> JsonObject:
    if not isinstance(value, dict) or set(value) != expected:
        fail(f"{description} fields drifted")
    return value


def require_sha256(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or SHA256.fullmatch(value) is None
        or len(set(value)) == 1
    ):
        fail(f"invalid {description}")
    return value


def require_id(value: Any, description: str) -> str:
    if not isinstance(value, str) or SAFE_ID.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def metadata_identity(value: os.stat_result) -> tuple[int, ...]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_nlink,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def directory_identity(value: os.stat_result) -> tuple[int, ...]:
    return (value.st_dev, value.st_ino, value.st_mode, value.st_uid)


def read_stable(
    path: Path,
    limit: int,
    description: str,
    *,
    single_link: bool = True,
) -> HeldInput:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        before_path = path.lstat()
        descriptor = os.open(path, flags)
        source: BinaryIO = os.fdopen(descriptor, "rb")
        before = os.fstat(source.fileno())
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    try:
        if (
            stat.S_ISLNK(before_path.st_mode)
            or not stat.S_ISREG(before_path.st_mode)
            or not stat.S_ISREG(before.st_mode)
            or (before_path.st_dev, before_path.st_ino)
            != (before.st_dev, before.st_ino)
            or (single_link and before.st_nlink != 1)
            or before.st_size <= 0
            or before.st_size > limit
        ):
            fail(f"{description} must be a bounded stable regular file")
        raw = source.read(limit + 1)
        after = os.fstat(source.fileno())
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    finally:
        source.close()
    try:
        after_path = path.lstat()
    except OSError as error:
        fail(f"cannot restat {description}: {error}")
    if (
        len(raw) != before.st_size
        or len(raw) > limit
        or metadata_identity(before_path) != metadata_identity(before)
        or metadata_identity(before) != metadata_identity(after)
        or metadata_identity(after) != metadata_identity(after_path)
    ):
        fail(f"{description} changed while it was read")
    return path, raw, before


def unique_object(pairs: list[tuple[str, Any]]) -> JsonObject:
    result: JsonObject = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def read_canonical_json(
    path: Path, description: str, *, single_link: bool = True
) -> tuple[JsonObject, bytes, os.stat_result]:
    _, raw, metadata = read_stable(
        path, MAX_JSON_BYTES, description, single_link=single_link
    )
    try:
        value = json.loads(raw.decode("ascii"), object_pairs_hook=unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse {description}: {error}")
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"{description} is not a canonical JSON object")
    return value, raw, metadata


def require_private_file(metadata: os.stat_result, description: str) -> None:
    if metadata.st_uid != os.geteuid() or stat.S_IMODE(metadata.st_mode) != 0o600:
        fail(f"{description} must be an owner-private 0600 file")


def safe_relative(value: Any, description: str) -> PurePosixPath:
    if not isinstance(value, str) or not value or "\\" in value or "\x00" in value:
        fail(f"invalid {description}")
    path = PurePosixPath(value)
    if (
        path.is_absolute()
        or path.as_posix() != value
        or any(part in {"", ".", ".."} for part in path.parts)
    ):
        fail(f"unsafe {description}")
    return path


def verify_private_directory(path: Path, description: str) -> None:
    try:
        metadata = path.lstat()
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(metadata.st_mode)
        or not stat.S_ISDIR(metadata.st_mode)
        or stat.S_IMODE(metadata.st_mode) != 0o700
        or metadata.st_uid != os.geteuid()
    ):
        fail(f"{description} must be an owner-private 0700 nonsymlink directory")


def exact_directory(argument: str, description: str) -> Path:
    supplied = Path(argument).absolute()
    try:
        resolved = Path(argument).resolve(strict=True)
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if supplied != resolved:
        fail(f"{description} path contains a symlink")
    return resolved


def require_external_root(
    root: Path, ferric: Path, fe2o3: Path, plan_dir: Path
) -> None:
    if any(
        repository == root or repository in root.parents or root in repository.parents
        for repository in (ferric, fe2o3, plan_dir)
    ):
        fail("qualification-run root must be outside both repositories and the plan")


def reject_symlink_components(
    root: Path, relative: PurePosixPath, description: str
) -> Path:
    current = root
    for part in relative.parts:
        current = current / part
        try:
            metadata = current.lstat()
        except OSError as error:
            fail(f"{description} is unavailable: {error}")
        if stat.S_ISLNK(metadata.st_mode):
            fail(f"{description} path contains a symlink")
    return current


def run(
    repo: Path, arguments: list[str], description: str, *, timeout: int = 120
) -> str:
    try:
        result = subprocess.run(
            arguments,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout,
            cwd=repo,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"{description} failed: {error}")
    if result.returncode != 0:
        fail(f"{description} failed: {result.stderr.strip()}")
    return result.stdout


def git(repo: Path, *arguments: str) -> str:
    return run(repo, ["git", "-C", str(repo), *arguments], "Git identity query")


def repository_identity(repo: Path, description: str) -> tuple[str, str]:
    commit = git(repo, "rev-parse", "HEAD^{commit}").strip()
    tree = git(repo, "rev-parse", "HEAD^{tree}").strip()
    if GIT_ID.fullmatch(commit) is None or GIT_ID.fullmatch(tree) is None:
        fail(f"invalid {description} Git identity")
    if git(repo, "status", "--porcelain=v1", "--untracked-files=all"):
        fail(f"{description} repository is not the exact clean Git tree")
    return commit, tree


def parse_time(value: Any, description: str) -> datetime:
    if not isinstance(value, str) or UTC_TIMESTAMP.fullmatch(value) is None:
        fail(f"invalid {description}")
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(
            tzinfo=timezone.utc
        )
    except ValueError as error:
        fail(f"invalid {description}: {error}")


def validate_environment(value: Any) -> JsonObject:
    environment = exact_keys(value, ENVIRONMENT_KEYS, "qualification environment")
    device = exact_keys(environment["device"], DEVICE_KEYS, "qualification device")
    driver = exact_keys(environment["driver"], DRIVER_KEYS, "qualification driver")
    firmware = exact_keys(
        environment["firmware"], FIRMWARE_KEYS, "qualification firmware"
    )
    host = exact_keys(environment["host"], HOST_KEYS, "qualification host")
    rocm = exact_keys(environment["rocm"], ROCM_KEYS, "qualification ROCm")
    if (
        device["device_count"] != 1
        or device["marketing_name"] != "AMD Instinct MI300X"
        or device["processor"] != "gfx942"
        or device["vendor_id"] != "1002"
        or device["xnack"] != "disabled"
        or not isinstance(device["device_uuid"], str)
        or UUID.fullmatch(device["device_uuid"]) is None
        or not isinstance(device["pci_bdf"], str)
        or PCI_BDF.fullmatch(device["pci_bdf"]) is None
        or driver["name"] != "amdgpu"
        or host["machine"] != "x86_64"
    ):
        fail("qualification target device or host identity drifted")
    for record, fields in (
        (driver, ("module_sha256",)),
        (firmware, ("bundle_sha256",)),
        (host, ("kernel_sha256", "os_release_sha256")),
        (rocm, ("installation_sha256",)),
    ):
        for field in fields:
            require_sha256(record[field], f"environment {field}")
    for record, fields in (
        (driver, ("version",)),
        (firmware, ("package_version",)),
        (rocm, ("version",)),
    ):
        for field in fields:
            if not isinstance(record[field], str) or not record[field].isascii():
                fail(f"invalid environment {field}")
    return environment


def validate_plan(
    ferric: Path, fe2o3: Path, plan_dir: Path, *, replay: bool = True
) -> tuple[JsonObject, JsonObject, bytes, JsonObject, bytes]:
    plan, plan_raw, _ = read_canonical_json(plan_dir / "plan.json", "M1 evidence plan")
    queue, queue_raw, _ = read_canonical_json(
        plan_dir / "missing-work.json", "M1 evidence work queue"
    )
    exact_keys(plan, PLAN_KEYS, "M1 evidence plan")
    exact_keys(queue, WORK_KEYS, "M1 evidence work queue")
    if (
        plan["format"] != PLAN_FORMAT
        or plan["authority"] != PLAN_AUTHORITY
        or plan["nonclaim"] != PLAN_NONCLAIM
        or plan["target"] != TARGET
        or plan["allocation_sha256"] != ALLOCATION_SHA256
        or plan["counts"]
        != {
            "assurance_binding_slots": 186,
            "binding_slots": 354,
            "obligation_slots": 50,
            "path_resolutions": 39,
            "roadmap_binding_slots": 168,
            "source_closures": 2,
            "trusted_validators": 12,
        }
        or plan["finalization"]
        != {
            "completion_transition_output": "completed-work.json",
            "evidence_index_output": "requires-authenticated-work-queue-completion-v1",
            "qualification_receipt_output": "requires-authenticated-work-queue-completion-v1",
            "required_validator": "proofs/check-m1-evidence-index.py",
        }
        or queue["format"] != WORK_FORMAT
        or queue["authority"] != PLAN_AUTHORITY
        or queue["nonclaim"] != PLAN_NONCLAIM
        or queue["status"] != "INCOMPLETE"
        or queue["plan_path"] != "plan.json"
        or queue["plan_sha256"] != digest_bytes(plan_raw)
        or queue["counts"]
        != {
            "available_producer_items": 358,
            "missing_items": 358,
            "missing_producer_items": 0,
        }
    ):
        fail("M1 plan or work-queue identity drifted")
    if replay:
        with tempfile.TemporaryDirectory(
            prefix="ferric-m1-finalizer-plan-replay-"
        ) as raw:
            replay_root = Path(raw) / "plan"
            run(
                ferric,
                [
                    sys.executable,
                    "-I",
                    str(ferric / "proofs/m1-qualification/planner.py"),
                    str(ferric),
                    str(fe2o3),
                    str(replay_root),
                ],
                "replay M1 evidence planner",
                timeout=1_800,
            )
            replayed_plan, replayed_plan_raw, _ = read_canonical_json(
                replay_root / "plan.json", "replayed M1 evidence plan"
            )
            replayed_queue, replayed_queue_raw, _ = read_canonical_json(
                replay_root / "missing-work.json", "replayed M1 evidence work queue"
            )
            if (
                replayed_plan != plan
                or replayed_plan_raw != plan_raw
                or replayed_queue != queue
                or replayed_queue_raw != queue_raw
            ):
                fail("M1 plan or work queue differs from an exact planner replay")
    if plan["planner_sha256"] != digest_bytes(
        read_stable(
            ferric / "proofs/m1-qualification/planner.py",
            MAX_FILE_BYTES,
            "planner source",
            single_link=False,
        )[1]
    ):
        fail("M1 planner source identity drifted")
    requirements, requirements_raw, _ = read_canonical_json(
        ferric / "proofs/M1_REQUIREMENTS.json",
        "M1 requirements",
        single_link=False,
    )
    if plan["requirements"] != {
        "format": requirements.get("format"),
        "path": "proofs/M1_REQUIREMENTS.json",
        "sha256": digest_bytes(requirements_raw),
    }:
        fail("M1 requirements identity drifted")
    sources = plan["sources"]
    if not isinstance(sources, list) or [item.get("id") for item in sources] != list(
        SOURCE_IDS
    ):
        fail("M1 source roster drifted")
    for source in sources:
        repo = fe2o3 if source["repository"] == "fe2o3" else ferric
        commit, tree = repository_identity(repo, source["repository"])
        if commit != source["commit"] or tree != source["tree"]:
            fail(f"M1 source identity drifted: {source['repository']}")
    closure_by_id = {
        record["artifact"]["id"]: record for record in plan["source_closures"]
    }
    if set(closure_by_id) != {"artifact.source.fe2o3", "artifact.source.ferric"}:
        fail("M1 source-closure roster drifted")
    with tempfile.TemporaryDirectory(prefix="ferric-m1-finalizer-closure-") as raw:
        for source in sources:
            record = closure_by_id[source["source_closure_artifact_id"]]
            expected_path = plan_dir / record["artifact"]["path"]
            _, expected, _ = read_stable(
                expected_path, MAX_FILE_BYTES, "planned source closure"
            )
            candidate = Path(raw) / f"{source['repository']}.records"
            repo = fe2o3 if source["repository"] == "fe2o3" else ferric
            run(
                ferric,
                [
                    sys.executable,
                    "-I",
                    str(ferric / "proofs/m1/evidence/measure-source-closure.py"),
                    str(repo),
                    str(candidate),
                ],
                f"measure {source['repository']} source closure",
            )
            measured = candidate.read_bytes()
            if (
                measured != expected
                or digest_bytes(expected) != source["source_closure_sha256"]
                or record["artifact"]["sha256"] != source["source_closure_sha256"]
                or record["artifact"]["size_bytes"] != len(expected)
            ):
                fail(f"M1 source closure drifted: {source['repository']}")
    receipt_items = [
        item for item in queue["items"] if item.get("id") == "work.qualification.m1"
    ]
    expected_receipt_item = {
        "expected_artifact": {
            "id": "artifact.qualification.m1",
            "kind": "QualificationReceipt",
            "path": "artifacts/artifact.qualification.m1.qualification-receipt.json",
        },
        "id": "work.qualification.m1",
        "producer": {
            "availability": "available",
            "command": [
                "python3",
                "-I",
                "proofs/m1-qualification/produce-qualification-receipt.py",
                "FERRIC_REPO",
                "FE2O3_REPO",
                "PLAN_DIR",
                "QUALIFICATION_RUN_ROOT",
            ],
            "role": "ferric-m1-qualification-intake-finalizer",
        },
        "state": "blocked-on-all-validated-evidence",
        "subject": "qualification:M1",
    }
    if receipt_items != [expected_receipt_item]:
        fail("M1 qualification-receipt work item drifted")
    return plan, queue, queue_raw, requirements, requirements_raw


def artifact_record(
    plan_dir: Path,
    expected: JsonObject,
    seen_inodes: set[tuple[int, int]],
) -> JsonObject:
    identifier = require_id(expected.get("id"), "artifact id")
    relative = safe_relative(expected.get("path"), f"artifact {identifier} path")
    path = reject_symlink_components(plan_dir, relative, f"artifact {identifier}")
    _, raw, metadata = read_stable(path, MAX_FILE_BYTES, f"artifact {identifier}")
    require_private_file(metadata, f"artifact {identifier}")
    inode = (metadata.st_dev, metadata.st_ino)
    if inode in seen_inodes:
        fail(f"M1 artifact is hard-linked or reused: {identifier}")
    seen_inodes.add(inode)
    return {
        "id": identifier,
        "kind": expected["kind"],
        "path": relative.as_posix(),
        "sha256": digest_bytes(raw),
        "size_bytes": len(raw),
    }


def derive_candidate_index(
    plan_dir: Path,
    plan: JsonObject,
    queue: JsonObject,
    requirements: JsonObject,
    requirements_raw: bytes,
) -> tuple[JsonObject, list[JsonObject]]:
    seen_inodes: set[tuple[int, int]] = set()
    artifacts: list[JsonObject] = []
    closure_by_source = {
        record["artifact"]["id"]: record for record in plan["source_closures"]
    }
    for source in plan["sources"]:
        artifacts.append(
            artifact_record(
                plan_dir,
                closure_by_source[source["source_closure_artifact_id"]]["artifact"],
                seen_inodes,
            )
        )
    queue_items = {item["id"]: item for item in queue["items"]}
    tcb: list[JsonObject] = []
    for tcb_id in TCB_IDS:
        work = queue_items.get(f"work.{tcb_id}")
        if not isinstance(work, dict):
            fail(f"M1 TCB work item is missing: {tcb_id}")
        artifact = artifact_record(plan_dir, work["expected_artifact"], seen_inodes)
        artifacts.append(artifact)
        tcb.append(
            {
                "artifact_id": artifact["id"],
                "id": tcb_id,
                "identity_sha256": artifact["sha256"],
                "kind": TCB_KINDS[tcb_id],
            }
        )
    bindings: list[JsonObject] = []
    for slot in plan["binding_slots"]:
        binding = slot["binding"]
        if (
            slot.get("state") != "missing"
            or slot.get("producer", {}).get("availability") != "available"
        ):
            fail(f"M1 binding work item drifted: {binding.get('id')}")
        artifacts.append(
            artifact_record(plan_dir, slot["expected_artifact"], seen_inodes)
        )
        bindings.append(binding)
    if [record["id"] for record in bindings] != sorted(
        record["id"] for record in bindings
    ):
        fail("M1 binding roster is not canonical")
    total = sum(record["size_bytes"] for record in artifacts)
    if total > MAX_TOTAL_ARTIFACT_BYTES:
        fail("M1 artifact closure exceeds the admitted byte bound")
    assurance_by_id = {
        record["name"]: record for record in requirements["assurance_properties"]
    }
    obligations: list[JsonObject] = []
    for slot in plan["obligation_slots"]:
        common: JsonObject = {
            "closure_status": slot["required_status"],
            "evidence_binding_ids": slot["binding_ids"],
            "id": slot["id"],
            "obligation_class": slot["obligation_class"],
            "path_resolution_ids": slot["path_ids"],
            "statement_sha256": slot["statement_sha256"],
            "tcb_ids": slot["tcb_ids"],
        }
        required = slot["required_artifact_ids"]
        if slot["obligation_class"] == "Roadmap":
            common["assurance_dependencies"] = slot["assurance_dependency_ids"]
            common["receipt_artifact_id"] = required["qualification_receipt"]
        elif slot["required_status"] == "Proved":
            common["mutation_artifact_ids"] = required["mutation"]
            common["proof_artifact_ids"] = required["proof"]
        elif slot["required_status"] == "Validated":
            common["validator_artifact_ids"] = required["validator"]
            common["validator_tcb_ids"] = list(TCB_IDS)
        elif slot["required_status"] == "Unsupported":
            specification = assurance_by_id.get(slot["id"])
            if specification is None:
                fail(f"unsupported M1 assurance is unknown: {slot['id']}")
            common["nonclaim_tcb_ids"] = list(TCB_IDS)
            common["rationale"] = specification["boundary"]
            common["rationale_artifact_ids"] = required["rationale"]
        else:
            fail(f"unknown M1 closure status: {slot['required_status']}")
        obligations.append(common)
    candidate = {
        "artifacts": sorted(artifacts, key=lambda record: record["id"]),
        "evidence_bindings": bindings,
        "format": INDEX_FORMAT,
        "obligations": obligations,
        "path_resolutions": plan["path_resolutions"],
        "requirements_sha256": digest_bytes(requirements_raw),
        "sources": plan["sources"],
        "tcb": tcb,
    }
    return candidate, tcb


def validator_roster(ferric: Path, plan: JsonObject) -> list[JsonObject]:
    result: list[JsonObject] = []
    validators = plan["trusted_validators"]
    if not isinstance(validators, list) or [
        record.get("evidence_kind") for record in validators
    ] != list(VALIDATOR_IDS):
        fail("M1 trusted-validator roster drifted")
    for record in validators:
        relative = safe_relative(record["path"], "trusted validator path")
        _, raw, _ = read_stable(
            ferric.joinpath(*relative.parts),
            MAX_FILE_BYTES,
            f"trusted validator {record['evidence_kind']}",
            single_link=False,
        )
        if digest_bytes(raw) != record["source_sha256"]:
            fail(f"trusted validator source drifted: {record['evidence_kind']}")
        result.append({"availability": "ExistingFoundation", **record})
    return result


def gate_rosters(candidate: JsonObject) -> dict[str, tuple[list[str], list[str]]]:
    bindings = candidate["evidence_bindings"]
    by_kind = {
        kind: [record for record in bindings if record["evidence_kind"] == kind]
        for kind in (
            "artifact-identity",
            "canonical-structure-check",
            "external-contract",
            "fe2o3-contract",
            "hardware-test",
            "independent-validator",
            "negative-mutation",
            "performance-gate",
            "unsupported-rationale",
            "verus-theorem",
        )
    }

    def select(*kinds: str) -> tuple[list[str], list[str]]:
        rows = [record for kind in kinds for record in by_kind[kind]]
        return (
            sorted(record["artifact_id"] for record in rows),
            sorted(record["id"] for record in rows),
        )

    source_ids = sorted(
        record["source_closure_artifact_id"] for record in candidate["sources"]
    )
    all_bindings = sorted(record["id"] for record in bindings)
    all_artifacts = sorted(record["id"] for record in candidate["artifacts"])
    validated_artifacts = sorted(
        {record["artifact_id"] for record in bindings}
        | {record["artifact_id"] for record in candidate["tcb"]}
    )
    return {
        "evidence-index": (all_artifacts, all_bindings),
        "hardware": select("hardware-test"),
        "performance": select("performance-gate"),
        "proof": select("negative-mutation", "verus-theorem"),
        "quality": (source_ids, []),
        "source-closure": (source_ids, []),
        "validators": (validated_artifacts, all_bindings),
    }


def load_intake(
    root: Path, candidate: JsonObject
) -> tuple[JsonObject, JsonObject, list[JsonObject], Path]:
    verify_private_directory(root, "qualification-run root")
    intake, _, intake_metadata = read_canonical_json(
        root / "intake.json", "qualification intake"
    )
    require_private_file(intake_metadata, "qualification intake")
    exact_keys(intake, INTAKE_KEYS, "qualification intake")
    if intake["format"] != INTAKE_FORMAT:
        fail("qualification intake format drifted")
    if (
        not isinstance(intake["run_id"], str)
        or UUID.fullmatch(intake["run_id"]) is None
    ):
        fail("qualification run identity is invalid")
    environment = validate_environment(intake["environment"])
    candidate_relative = safe_relative(
        intake["candidate_index_relative_path"], "candidate index path"
    )
    candidate_path = reject_symlink_components(
        root, candidate_relative, "candidate evidence index"
    )
    supplied_candidate, _, candidate_metadata = read_canonical_json(
        candidate_path, "candidate evidence index"
    )
    require_private_file(candidate_metadata, "candidate evidence index")
    exact_keys(supplied_candidate, INDEX_KEYS, "candidate evidence index")
    if supplied_candidate != candidate:
        fail("candidate evidence index differs from the plan-derived closure")
    tool_inputs = intake["tools"]
    if not isinstance(tool_inputs, list) or len(tool_inputs) != len(MEASURED_TOOL_IDS):
        fail("qualification measured-tool roster is incomplete")
    measured: list[JsonObject] = []
    measured_identities: set[str] = set()
    for record, expected_id in zip(tool_inputs, MEASURED_TOOL_IDS, strict=True):
        exact_keys(record, INTAKE_TOOL_KEYS, f"qualification tool {expected_id}")
        if record["id"] != expected_id:
            fail(f"qualification measured tool drifted: {expected_id}")
        path_value = record["binary_absolute_path"]
        if not isinstance(path_value, str) or not Path(path_value).is_absolute():
            fail(f"qualification tool path is invalid: {expected_id}")
        path = Path(path_value)
        if path.resolve(strict=True) != path.absolute():
            fail(f"qualification tool path contains a symlink: {expected_id}")
        _, raw, metadata = read_stable(
            path, MAX_FILE_BYTES, f"qualification tool {expected_id}", single_link=False
        )
        if stat.S_IMODE(metadata.st_mode) & 0o111 == 0:
            fail(f"qualification tool is not executable: {expected_id}")
        identity = digest_bytes(raw)
        if identity in measured_identities:
            fail("qualification measured-tool identity is duplicated")
        measured_identities.add(identity)
        version_output = run(
            path.parent,
            [str(path), "--version"],
            f"measure qualification tool version {expected_id}",
        ).strip()
        version_parts = version_output.split()
        expected_name = expected_id.removeprefix("compiler.")
        if version_parts[:2] != [expected_name, "1.97.1"]:
            fail(f"qualification tool version drifted: {expected_id}")
        measured.append(
            {
                "authority": "qualification-measured-binary",
                "id": expected_id,
                "identity_sha256": identity,
                "version": "1.97.1",
            }
        )
    return intake, environment, measured, candidate_path


def build_tools(
    ferric: Path, measured: list[JsonObject], validators: list[JsonObject]
) -> list[JsonObject]:
    measured_by_id = {record["id"]: record for record in measured}
    validator_by_id = {record["evidence_kind"]: record for record in validators}
    verus_version_raw = read_stable(
        ferric / "proofs/verus/VERUS_VERSION",
        4096,
        "Verus version pin",
        single_link=False,
    )[1]
    try:
        verus_version = verus_version_raw.decode("ascii").removesuffix("\n")
    except UnicodeDecodeError as error:
        fail(f"Verus version pin is not ASCII: {error}")
    verus_manifest = read_stable(
        ferric / "proofs/verus/VERUS_CLOSURE_MANIFEST",
        MAX_FILE_BYTES,
        "Verus closure manifest",
        single_link=False,
    )[1]
    checker = read_stable(
        ferric / "proofs/check-m1-evidence-index.py",
        MAX_FILE_BYTES,
        "M1 evidence-index checker",
        single_link=False,
    )[1]
    try:
        python_path = Path(sys.executable).resolve(strict=True)
    except OSError as error:
        fail(f"qualification Python executable is unavailable: {error}")
    _, python_raw, python_metadata = read_stable(
        python_path,
        MAX_FILE_BYTES,
        "qualification Python executable",
        single_link=False,
    )
    if stat.S_IMODE(python_metadata.st_mode) & 0o111 == 0:
        fail("qualification Python executable is not executable")
    python_version = (
        f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
    )
    if PYTHON_VERSION.fullmatch(python_version) is None:
        fail("qualification Python version is not exact")
    tools = [
        measured_by_id["compiler.cargo"],
        measured_by_id["compiler.rustc"],
        {
            "authority": "pinned-proof-tool-closure",
            "id": "compiler.verus",
            "identity_sha256": digest_bytes(verus_manifest),
            "version": verus_version,
        },
        {
            "authority": "qualification-measured-binary",
            "id": "runtime.python",
            "identity_sha256": digest_bytes(python_raw),
            "version": python_version,
        },
        {
            "authority": "checker-owned-source",
            "id": "validator.evidence-index",
            "identity_sha256": digest_bytes(checker),
            "version": INDEX_FORMAT,
        },
        {
            "authority": "checker-owned-source",
            "id": "validator.qualification-receipt",
            "identity_sha256": validator_by_id["qualification-receipt"][
                "source_sha256"
            ],
            "version": VALIDATOR_PROTOCOL,
        },
    ]
    if [record["id"] for record in tools] != list(TOOL_IDS) or len(
        {record["identity_sha256"] for record in tools}
    ) != len(tools):
        fail("qualification tool roster is substituted or duplicated")
    return tools


def utc_timestamp(value: datetime) -> str:
    return value.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def execute_pre_receipt_gates(
    ferric: Path,
    fe2o3: Path,
    candidate_path: Path,
    artifact_root: Path,
    candidate: JsonObject,
    tools: list[JsonObject],
) -> tuple[list[JsonObject], datetime, datetime]:
    checker_path = ferric / "proofs/check-m1-evidence-index.py"
    checker_raw = read_stable(
        checker_path,
        MAX_FILE_BYTES,
        "M1 pre-receipt gate checker",
        single_link=False,
    )[1]
    tool_by_id = {record["id"]: record for record in tools}
    checker_sha256 = digest_bytes(checker_raw)
    python_sha256 = tool_by_id["runtime.python"]["identity_sha256"]
    candidate_sha256 = digest_bytes(canonical_bytes(candidate))
    rosters = gate_rosters(candidate)
    gates: list[JsonObject] = []
    run_start: datetime | None = None
    run_end: datetime | None = None
    bootstrap = (
        "import os,sys;"
        "p=sys.argv.pop(1);f=int(sys.argv.pop(1));sys.argv[0]=p;"
        "s=os.fdopen(os.dup(f),'rb').read();"
        "g={'__name__':'__main__','__file__':p,'__package__':None};"
        "exec(compile(s,p,'exec'),g)"
    )
    with tempfile.TemporaryFile(mode="w+b") as pinned_checker:
        pinned_checker.write(checker_raw)
        pinned_checker.flush()
        for gate_id in GATE_IDS:
            started = datetime.now(timezone.utc).replace(microsecond=0)
            pinned_checker.seek(0)
            try:
                result = subprocess.run(
                    [
                        sys.executable,
                        "-I",
                        "-c",
                        bootstrap,
                        str(checker_path),
                        str(pinned_checker.fileno()),
                        PRE_RECEIPT_PROTOCOL,
                        gate_id,
                        str(ferric),
                        str(candidate_path),
                        str(artifact_root),
                        str(fe2o3),
                    ],
                    check=False,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    timeout=1_800,
                    cwd=ferric,
                    env={"PATH": os.environ.get("PATH", "")},
                    pass_fds=(pinned_checker.fileno(),),
                )
            except (OSError, subprocess.TimeoutExpired) as error:
                fail(f"source-pinned M1 pre-receipt gate {gate_id} failed: {error}")
            if result.returncode != 0:
                fail(
                    f"source-pinned M1 pre-receipt gate {gate_id} failed: "
                    f"{result.stderr.strip()}"
                )
            output = result.stdout
            finished = datetime.now(timezone.utc).replace(microsecond=0)
            if finished <= started:
                finished = started + timedelta(seconds=1)
            expected = (
                f"PASS: {PRE_RECEIPT_PROTOCOL} gate={gate_id} "
                f"candidate_sha256={candidate_sha256}\n"
            )
            if output != expected:
                fail(f"M1 pre-receipt gate output drifted: {gate_id}")
            command = {
                "candidate_sha256": candidate_sha256,
                "checker_sha256": checker_sha256,
                "gate_id": gate_id,
                "protocol": PRE_RECEIPT_PROTOCOL,
                "runtime_python_sha256": python_sha256,
            }
            artifacts, bindings = rosters[gate_id]
            gates.append(
                {
                    "artifact_ids": artifacts,
                    "binding_ids": bindings,
                    "command_sha256": canonical_digest(command),
                    "finished_at_utc": utc_timestamp(finished),
                    "id": gate_id,
                    "output_sha256": digest_bytes(output.encode("ascii")),
                    "result": "pass",
                    "started_at_utc": utc_timestamp(started),
                }
            )
            run_start = started if run_start is None else min(run_start, started)
            run_end = finished if run_end is None else max(run_end, finished)
    if run_start is None or run_end is None:
        fail("M1 pre-receipt gate roster is empty")
    return gates, run_start, run_end


def build_work_completion(
    plan: JsonObject,
    queue: JsonObject,
    candidate: JsonObject,
    gates: list[JsonObject],
) -> JsonObject:
    item_ids = [record.get("id") for record in queue["items"]]
    expected_count = len(candidate["evidence_bindings"]) + len(TCB_IDS) + 1
    if (
        len(item_ids) != expected_count
        or not all(isinstance(identifier, str) for identifier in item_ids)
        or item_ids != sorted(item_ids)
        or len(set(item_ids)) != len(item_ids)
    ):
        fail("M1 work queue cannot transition to an exact completed roster")
    completion = {
        "authority": COMPLETION_AUTHORITY,
        "candidate_index_sha256": digest_bytes(canonical_bytes(candidate)),
        "completed_item_ids": item_ids,
        "counts": {"completed_items": len(item_ids), "missing_items": 0},
        "format": COMPLETION_FORMAT,
        "gate_roster_sha256": canonical_digest(gates),
        "plan_path": "plan.json",
        "plan_sha256": digest_bytes(canonical_bytes(plan)),
        "queue_path": "missing-work.json",
        "queue_sha256": digest_bytes(canonical_bytes(queue)),
        "status": "COMPLETE",
    }
    exact_keys(completion, COMPLETION_KEYS, "M1 completed work transition")
    return completion


def validate_final_artifact_size(candidate: JsonObject, receipt_size: int) -> None:
    total = sum(record["size_bytes"] for record in candidate["artifacts"])
    if total + receipt_size > MAX_TOTAL_ARTIFACT_BYTES:
        fail("final M1 artifact closure exceeds the admitted byte bound")


def qualification_identity(transcript: JsonObject) -> str:
    return canonical_digest(
        {
            "environment_identity_sha256": transcript["environment_identity_sha256"],
            "gate_roster_sha256": transcript["gate_roster_sha256"],
            "index_roster_sha256": transcript["index_roster_sha256"],
            "requirements_sha256": transcript["requirements_sha256"],
            "run_id": transcript["run_id"],
            "source_closure_sha256s": transcript["source_closure_sha256s"],
            "source_roster_sha256": transcript["source_roster_sha256"],
            "target_identity_sha256": transcript["target_identity_sha256"],
            "tcb_roster_sha256": transcript["tcb_roster_sha256"],
            "tool_roster_sha256": transcript["tool_roster_sha256"],
            "validator_roster_sha256": transcript["validator_roster_sha256"],
            "work_queue_completion_sha256": transcript["work_queue_completion_sha256"],
        }
    )


def source_closure_roster(plan: JsonObject) -> list[JsonObject]:
    closure_by_id = {
        record["artifact"]["id"]: record for record in plan["source_closures"]
    }
    return [
        {
            "artifact_id": source["source_closure_artifact_id"],
            "commit": source["commit"],
            "file_count": closure_by_id[source["source_closure_artifact_id"]][
                "file_count"
            ],
            "id": source["id"],
            "sha256": source["source_closure_sha256"],
            "tree": source["tree"],
        }
        for source in plan["sources"]
    ]


def directory_flags() -> int:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    return flags


def open_held_directory(path: Path, description: str) -> tuple[int, tuple[int, ...]]:
    try:
        descriptor = os.open(path, directory_flags())
        metadata = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot hold {description}: {error}")
    if not stat.S_ISDIR(metadata.st_mode):
        os.close(descriptor)
        fail(f"{description} is not a directory")
    return descriptor, directory_identity(metadata)


def verify_held_directory(
    descriptor: int, path: Path, identity: tuple[int, ...], description: str
) -> None:
    try:
        held = os.fstat(descriptor)
        current = path.lstat()
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        directory_identity(held) != identity
        or directory_identity(current) != identity
        or not stat.S_ISDIR(current.st_mode)
    ):
        fail(f"{description} changed during publication")


def open_held_child_directory(
    parent: int, name: str, path: Path, description: str
) -> tuple[int, tuple[int, ...]]:
    descriptor: int | None = None
    try:
        descriptor = os.open(name, directory_flags(), dir_fd=parent)
        held = os.fstat(descriptor)
        current = os.stat(name, dir_fd=parent, follow_symlinks=False)
    except OSError as error:
        if descriptor is not None:
            os.close(descriptor)
        raise OSError(f"cannot hold {description}: {error}") from error
    if not stat.S_ISDIR(held.st_mode) or directory_identity(held) != directory_identity(
        current
    ):
        os.close(descriptor)
        raise OSError(f"{description} path identity drifted: {path}")
    return descriptor, directory_identity(held)


def require_absent(directory: int, name: str, path: Path) -> None:
    try:
        os.stat(name, dir_fd=directory, follow_symlinks=False)
    except FileNotFoundError:
        return
    except OSError as error:
        fail(f"cannot inspect qualification output {path}: {error}")
    fail(f"qualification output already exists: {path}")


def unlink_if_identity(
    directory: int,
    name: str,
    path: Path,
    identity: tuple[int, ...],
) -> str | None:
    try:
        current = os.stat(name, dir_fd=directory, follow_symlinks=False)
        if metadata_identity(current) != identity:
            return f"replacement preserved: {path}"
        os.unlink(name, dir_fd=directory)
    except OSError as error:
        return f"{path}: {error}"
    return None


def publish_new_at(
    directory: int, name: str, path: Path, payload: bytes
) -> PublishedFile:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        descriptor = os.open(name, flags, 0o600, dir_fd=directory)
        created = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot create qualification output {path}: {error}")
    identity = metadata_identity(created)
    try:
        if (
            not stat.S_ISREG(created.st_mode)
            or created.st_uid != os.geteuid()
            or created.st_nlink != 1
            or stat.S_IMODE(created.st_mode) != 0o600
        ):
            raise OSError(
                "created output identity is not owner-private and single-link"
            )
        with os.fdopen(descriptor, "wb", closefd=False) as destination:
            destination.write(payload)
            destination.flush()
            os.fsync(destination.fileno())
        completed = os.fstat(descriptor)
        if (
            not stat.S_ISREG(completed.st_mode)
            or completed.st_uid != os.geteuid()
            or completed.st_nlink != 1
            or stat.S_IMODE(completed.st_mode) != 0o600
        ):
            raise OSError("published output identity changed while it was written")
        identity = metadata_identity(completed)
    except OSError as error:
        try:
            identity = metadata_identity(os.fstat(descriptor))
        except OSError:
            pass
        os.close(descriptor)
        cleanup = unlink_if_identity(directory, name, path, identity)
        suffix = "" if cleanup is None else f"; cleanup failed: {cleanup}"
        fail(f"cannot publish qualification output {path}: {error}{suffix}")
    os.close(descriptor)
    return directory, name, path, identity


def sync_directory(descriptor: int, path: Path) -> None:
    try:
        os.fsync(descriptor)
    except OSError as error:
        fail(f"cannot synchronize directory {path}: {error}")


def rollback(published: list[PublishedFile]) -> list[str]:
    failures: list[str] = []
    directories: set[tuple[int, Path]] = set()
    for directory, name, path, identity in reversed(published):
        failure = unlink_if_identity(directory, name, path, identity)
        if failure is not None:
            failures.append(failure)
        directories.add((directory, path.parent))
    for directory, path in directories:
        try:
            os.fsync(directory)
        except OSError as error:
            failures.append(f"cannot synchronize rollback directory {path}: {error}")
    return failures


def report_cleanup_failures(failures: list[str]) -> None:
    if failures:
        print(
            "FAIL: qualification output rollback failed: " + "; ".join(failures),
            file=sys.stderr,
        )


def remove_created_directory(
    parent: int,
    name: str,
    path: Path,
    identity: tuple[int, ...] | None,
) -> list[str]:
    if identity is None:
        return []
    try:
        current = os.stat(name, dir_fd=parent, follow_symlinks=False)
        if directory_identity(current) != identity:
            return [f"replacement directory preserved: {path}"]
        os.rmdir(name, dir_fd=parent)
        os.fsync(parent)
    except FileNotFoundError:
        return []
    except OSError as error:
        return [f"{path}: {error}"]
    return []


def produce(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    intake_argument: str,
) -> None:
    ferric = exact_directory(ferric_argument, "Ferric repository")
    fe2o3 = exact_directory(fe2o3_argument, "fe2o3 repository")
    plan_dir = exact_directory(plan_argument, "M1 plan directory")
    intake_root = exact_directory(intake_argument, "qualification-run root")
    require_external_root(intake_root, ferric, fe2o3, plan_dir)
    verify_private_directory(plan_dir, "M1 plan directory")
    plan, queue, queue_raw, requirements, requirements_raw = validate_plan(
        ferric, fe2o3, plan_dir
    )
    candidate, tcb = derive_candidate_index(
        plan_dir, plan, queue, requirements, requirements_raw
    )
    intake, environment, measured, candidate_path = load_intake(intake_root, candidate)
    validators = validator_roster(ferric, plan)
    tools = build_tools(ferric, measured, validators)
    gates, run_start, run_end = execute_pre_receipt_gates(
        ferric, fe2o3, candidate_path, plan_dir, candidate, tools
    )
    completion = build_work_completion(plan, queue, candidate, gates)
    completion_raw = canonical_bytes(completion)
    completion_relative = "completed-work.json"
    transcript: JsonObject = {
        "all_required_gates_passed": True,
        "environment": environment,
        "environment_identity_sha256": canonical_digest(environment),
        "finished_at_utc": utc_timestamp(run_end),
        "format": TRANSCRIPT_FORMAT,
        "gate_roster_sha256": canonical_digest(gates),
        "gates": gates,
        "index_roster_sha256": canonical_digest(candidate),
        "milestone": "M1",
        "no_failed_gates": True,
        "no_skipped_gates": True,
        "protocol": QUALIFICATION_PROTOCOL,
        "qualification_id_sha256": "",
        "requirements_sha256": candidate["requirements_sha256"],
        "result": "pass",
        "run_id": intake["run_id"],
        "source_closure_sha256s": {
            record["id"]: record["source_closure_sha256"]
            for record in candidate["sources"]
        },
        "source_roster_sha256": canonical_digest(candidate["sources"]),
        "started_at_utc": utc_timestamp(run_start),
        "target": TARGET_VALUE,
        "target_identity_sha256": canonical_digest(TARGET_VALUE),
        "tcb_identity_sha256s": {
            record["id"]: record["identity_sha256"] for record in tcb
        },
        "tcb_roster_sha256": canonical_digest(tcb),
        "tool_roster_sha256": canonical_digest(tools),
        "tools": tools,
        "validator_roster_sha256": canonical_digest(validators),
        "work_queue_completion_relative_path": completion_relative,
        "work_queue_completion_sha256": digest_bytes(completion_raw),
        "work_queue_completion_size_bytes": len(completion_raw),
    }
    transcript["qualification_id_sha256"] = qualification_identity(transcript)
    transcript_raw = canonical_bytes(transcript)
    receipt_id = "artifact.qualification.m1"
    report_relative = "artifacts/artifact.qualification.m1.qualification-receipt.json"
    transcript_relative = "qualification-transcripts/artifact.qualification.m1.json"
    final_artifact_count = len(candidate["artifacts"]) + 1
    report: JsonObject = {
        "artifact_count": final_artifact_count,
        "artifact_roster_sha256": canonical_digest(candidate["artifacts"]),
        "assurance_count": 17,
        "authority": AUTHORITY,
        "binding_count": len(candidate["evidence_bindings"]),
        "binding_roster_sha256": canonical_digest(candidate["evidence_bindings"]),
        "format": REPORT_FORMAT,
        "gate_ids": list(GATE_IDS),
        "index_roster_sha256": canonical_digest(candidate),
        "milestone": "M1",
        "nonclaim": (
            "This validator authenticates the exact evidence closure and immutable "
            "qualification transcript supplied by the checker. It does not generate "
            "evidence, execute qualification gates, alter repository requirements, or "
            "turn an Open in-repository obligation into a closure claim."
        ),
        "obligation_roster_sha256": canonical_digest(candidate["obligations"]),
        "path_count": 39,
        "path_roster_sha256": canonical_digest(candidate["path_resolutions"]),
        "protocol": VALIDATOR_PROTOCOL,
        "qualification_id_sha256": transcript["qualification_id_sha256"],
        "receipt_artifact": {
            "id": receipt_id,
            "kind": "QualificationReceipt",
            "path": report_relative,
        },
        "requirements_roster_sha256": canonical_digest(requirements),
        "requirements_sha256": candidate["requirements_sha256"],
        "result": "pass",
        "roadmap_count": 33,
        "source_closure_roster": source_closure_roster(plan),
        "source_roster": candidate["sources"],
        "source_roster_sha256": canonical_digest(candidate["sources"]),
        "target": TARGET,
        "tcb_roster": tcb,
        "tcb_roster_sha256": canonical_digest(tcb),
        "transcript_relative_path": transcript_relative,
        "transcript_sha256": digest_bytes(transcript_raw),
        "transcript_size_bytes": len(transcript_raw),
        "validator_count": len(validators),
        "validator_roster": validators,
        "validator_roster_sha256": canonical_digest(validators),
        "work_queue_completion_relative_path": completion_relative,
        "work_queue_completion_sha256": digest_bytes(completion_raw),
        "work_queue_completion_size_bytes": len(completion_raw),
    }
    report_raw = canonical_bytes(report)
    validate_final_artifact_size(candidate, len(report_raw))
    receipt_artifact = {
        "id": receipt_id,
        "kind": "QualificationReceipt",
        "path": report_relative,
        "sha256": digest_bytes(report_raw),
        "size_bytes": len(report_raw),
    }
    final_index = {
        **candidate,
        "artifacts": sorted(
            [*candidate["artifacts"], receipt_artifact],
            key=lambda record: record["id"],
        ),
    }
    index_raw = canonical_bytes(final_index)

    def revalidate_inputs() -> None:
        (
            repeated_plan,
            repeated_queue,
            repeated_queue_raw,
            repeated_requirements,
            repeated_requirements_raw,
        ) = validate_plan(ferric, fe2o3, plan_dir, replay=False)
        repeated_candidate, repeated_tcb = derive_candidate_index(
            plan_dir,
            repeated_plan,
            repeated_queue,
            repeated_requirements,
            repeated_requirements_raw,
        )
        (
            repeated_intake,
            repeated_environment,
            repeated_measured,
            repeated_candidate_path,
        ) = load_intake(intake_root, repeated_candidate)
        repeated_validators = validator_roster(ferric, repeated_plan)
        repeated_tools = build_tools(ferric, repeated_measured, repeated_validators)
        if (
            repeated_plan != plan
            or repeated_queue != queue
            or repeated_queue_raw != queue_raw
            or repeated_requirements != requirements
            or repeated_requirements_raw != requirements_raw
            or repeated_candidate != candidate
            or repeated_tcb != tcb
            or repeated_intake != intake
            or repeated_environment != environment
            or repeated_measured != measured
            or repeated_candidate_path != candidate_path
            or repeated_validators != validators
            or repeated_tools != tools
        ):
            fail("qualification inputs changed during finalization")

    revalidate_inputs()
    completion_path = plan_dir / completion_relative
    transcript_path = plan_dir / transcript_relative
    report_path = plan_dir / report_relative
    index_path = plan_dir / "evidence-index.json"
    plan_fd: int | None = None
    artifacts_fd: int | None = None
    transcript_fd: int | None = None
    transcript_identity: tuple[int, ...] | None = None
    published: list[PublishedFile] = []
    try:
        plan_fd, plan_identity = open_held_directory(plan_dir, "M1 plan directory")
        artifacts_fd, artifacts_identity = open_held_child_directory(
            plan_fd, "artifacts", report_path.parent, "M1 artifact directory"
        )
        require_absent(plan_fd, completion_relative, completion_path)
        require_absent(plan_fd, "evidence-index.json", index_path)
        require_absent(artifacts_fd, report_path.name, report_path)
        require_absent(
            plan_fd,
            "qualification-transcripts",
            transcript_path.parent,
        )
        os.mkdir("qualification-transcripts", mode=0o700, dir_fd=plan_fd)
        transcript_identity = directory_identity(
            os.stat(
                "qualification-transcripts",
                dir_fd=plan_fd,
                follow_symlinks=False,
            )
        )
        transcript_fd, transcript_identity = open_held_child_directory(
            plan_fd,
            "qualification-transcripts",
            transcript_path.parent,
            "qualification transcript directory",
        )
        published.append(
            publish_new_at(
                plan_fd, completion_relative, completion_path, completion_raw
            )
        )
        published.append(
            publish_new_at(
                transcript_fd, transcript_path.name, transcript_path, transcript_raw
            )
        )
        published.append(
            publish_new_at(artifacts_fd, report_path.name, report_path, report_raw)
        )
        published.append(
            publish_new_at(plan_fd, "evidence-index.json", index_path, index_raw)
        )
        for directory, name, path, identity in published:
            current = os.stat(name, dir_fd=directory, follow_symlinks=False)
            if metadata_identity(current) != identity:
                fail(f"qualification output changed during publication: {path}")
        revalidate_inputs()
        verify_held_directory(
            transcript_fd,
            transcript_path.parent,
            transcript_identity,
            "qualification transcript directory",
        )
        verify_held_directory(
            artifacts_fd,
            report_path.parent,
            artifacts_identity,
            "M1 artifact directory",
        )
        verify_held_directory(plan_fd, plan_dir, plan_identity, "M1 plan directory")
        sync_directory(transcript_fd, transcript_path.parent)
        sync_directory(artifacts_fd, report_path.parent)
        sync_directory(plan_fd, plan_dir)
        for directory, name, path, identity in published:
            current = os.stat(name, dir_fd=directory, follow_symlinks=False)
            if metadata_identity(current) != identity:
                fail(f"qualification output changed after synchronization: {path}")
        verify_held_directory(
            transcript_fd,
            transcript_path.parent,
            transcript_identity,
            "qualification transcript directory",
        )
        verify_held_directory(
            artifacts_fd,
            report_path.parent,
            artifacts_identity,
            "M1 artifact directory",
        )
        verify_held_directory(plan_fd, plan_dir, plan_identity, "M1 plan directory")
    except BaseException:
        cleanup_failures = rollback(published)
        if transcript_fd is not None:
            os.close(transcript_fd)
            transcript_fd = None
        if plan_fd is not None:
            cleanup_failures.extend(
                remove_created_directory(
                    plan_fd,
                    "qualification-transcripts",
                    transcript_path.parent,
                    transcript_identity,
                )
            )
        report_cleanup_failures(cleanup_failures)
        raise
    finally:
        if transcript_fd is not None:
            os.close(transcript_fd)
        if artifacts_fd is not None:
            os.close(artifacts_fd)
        if plan_fd is not None:
            os.close(plan_fd)
    print(
        "PASS: assembled M1 qualification receipt intake "
        f"qualification_id={transcript['qualification_id_sha256']} "
        "(nonclaim: run proofs/check-m1-evidence-index.py separately)"
    )


def prepare_candidate(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    output_argument: str,
) -> None:
    ferric = exact_directory(ferric_argument, "Ferric repository")
    fe2o3 = exact_directory(fe2o3_argument, "fe2o3 repository")
    plan_dir = exact_directory(plan_argument, "M1 plan directory")
    output = Path(output_argument).absolute()
    try:
        parent = output.parent.resolve(strict=True)
    except OSError as error:
        fail(f"qualification-run parent is unavailable: {error}")
    if output.parent != parent:
        fail("qualification-run output must be a new canonical path")
    verify_private_directory(parent, "qualification-run parent")
    require_external_root(output, ferric, fe2o3, plan_dir)
    verify_private_directory(plan_dir, "M1 plan directory")
    plan, queue, _, requirements, requirements_raw = validate_plan(
        ferric, fe2o3, plan_dir
    )
    candidate, _ = derive_candidate_index(
        plan_dir, plan, queue, requirements, requirements_raw
    )
    parent_fd: int | None = None
    output_fd: int | None = None
    output_identity: tuple[int, ...] | None = None
    published: list[PublishedFile] = []
    try:
        parent_fd, parent_identity = open_held_directory(
            parent, "qualification-run parent"
        )
        require_absent(parent_fd, output.name, output)
        os.mkdir(output.name, mode=0o700, dir_fd=parent_fd)
        output_identity = directory_identity(
            os.stat(output.name, dir_fd=parent_fd, follow_symlinks=False)
        )
        output_fd, output_identity = open_held_child_directory(
            parent_fd, output.name, output, "qualification-run root"
        )
        candidate_path = output / "candidate-index.json"
        published.append(
            publish_new_at(
                output_fd,
                candidate_path.name,
                candidate_path,
                canonical_bytes(candidate),
            )
        )
        verify_held_directory(
            output_fd, output, output_identity, "qualification-run root"
        )
        verify_held_directory(
            parent_fd, parent, parent_identity, "qualification-run parent"
        )
        sync_directory(output_fd, output)
        sync_directory(parent_fd, parent)
    except BaseException:
        cleanup_failures = rollback(published)
        if output_fd is not None:
            os.close(output_fd)
            output_fd = None
        if parent_fd is not None:
            cleanup_failures.extend(
                remove_created_directory(
                    parent_fd,
                    output.name,
                    output,
                    output_identity,
                )
            )
        report_cleanup_failures(cleanup_failures)
        raise
    finally:
        if output_fd is not None:
            os.close(output_fd)
        if parent_fd is not None:
            os.close(parent_fd)
    print(
        "PASS: exported plan-derived M1 candidate evidence index "
        f"sha256={canonical_digest(candidate)} (nonclaim: no receipt or closure)"
    )


def main() -> None:
    if len(sys.argv) == 6 and sys.argv[1] == "prepare-candidate":
        prepare_candidate(sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5])
        return
    if len(sys.argv) != 5:
        fail(
            f"usage: {sys.argv[0]} [prepare-candidate] FERRIC_REPO FE2O3_REPO "
            "PLAN_DIR QUALIFICATION_RUN_ROOT"
        )
    produce(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4])


if __name__ == "__main__":
    main()
