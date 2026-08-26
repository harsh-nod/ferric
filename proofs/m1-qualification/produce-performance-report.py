#!/usr/bin/env python3
"""Import one externally measured M1 performance suite and publish its report."""

from __future__ import annotations

from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys
import tempfile
from typing import Any, BinaryIO, Callable, NoReturn


PLAN_FORMAT = "FERRIC-M1-EVIDENCE-PLAN-V1"
WORK_FORMAT = "FERRIC-M1-EVIDENCE-WORK-QUEUE-V1"
PLAN_AUTHORITY = "planning-only-no-evidence"
INTAKE_FORMAT = "FERRIC-M1-PERFORMANCE-INTAKE-V1"
REPORT_FORMAT = "FERRIC-M1-PERFORMANCE-REPORT-V1"
MEASUREMENT_FORMAT = "FERRIC-M1-PERFORMANCE-MEASUREMENTS-V1"
TARGET = "gfx942:xnack-"
AUTHORITY = "checked-performance-only"
NONCLAIM = (
    "This report authenticates checked performance measurements only. It does "
    "not establish semantic correctness, theorem truth, machine refinement, "
    "artifact loading, queue publication, kernel launch, hardware correctness, "
    "or M1 qualification and closes no obligation."
)
PERFORMANCE_POLICY_PATH = "docs/PERFORMANCE.md"
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
PERFORMANCE_ROSTER_SHA256 = (
    "534b95746e961c13f470aca4be53fa4d35f54fa5c8efe6a79792a8c28fe7e645"
)
PERFORMANCE_TOPOLOGY_SHA256 = (
    "20cefdf2a19b6a22dd3750047dec6ad26101df4b7c1ab152fab024a314d6fc8f"
)
PERFORMANCE_ARTIFACT_TOPOLOGY_SHA256 = (
    "0901f56b657064ba46bacf72435e8756975257bda5a7485eb6db46d2e62f3812"
)
PRODUCER_PATH = "proofs/m1-qualification/produce-performance-report.py"
PRODUCER_ROLE = "ferric-m1-performance-intake-reporter"
TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
TCB_REPORT_FORMAT = "FERRIC-M1-TCB-REPORT-V1"
TCB_REPORT_AUTHORITY = "trusted-boundary-declaration-only"
TCB_REPORT_NONCLAIM = (
    "This report authenticates the declared M1 trusted boundary only. It does "
    "not establish component presence, version provenance, compiler or runtime "
    "correctness, hardware behavior, theorem truth, machine refinement, load, "
    "launch, performance, or qualification authority and closes no obligation."
)
TCB_REPORT_KEYS = {
    "authority",
    "component_roster",
    "evidence_kind",
    "format",
    "milestone",
    "nonclaim",
    "obligation_roster",
    "obligation_state",
    "path_roster",
    "profile_roster",
    "requirements_sha256",
    "source_roster",
    "subject_tcb_id",
    "subject_tcb_kind",
    "target",
    "tcb_structure_roster",
    "validator_roster",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
BASELINE_IDS = (
    "baseline.vendor",
    "baseline.vllm",
    "baseline.sglang",
    "baseline.ferric-target-only",
    "baseline.ferric-reference",
)
BASELINE_KINDS = {
    "baseline.vendor": "VendorKernel",
    "baseline.vllm": "vLLM",
    "baseline.sglang": "SGLang",
    "baseline.ferric-target-only": "FerricTargetOnly",
    "baseline.ferric-reference": "FerricReference",
}
CELL_ENGINES = {
    "core-kernel": ("ferric", "baseline.vendor", "baseline.ferric-reference"),
    "serving-primary": (
        "ferric",
        "baseline.vllm",
        "baseline.sglang",
        "baseline.ferric-reference",
    ),
    "speculation": (
        "ferric",
        "baseline.ferric-target-only",
        "baseline.ferric-reference",
    ),
    "low-acceptance": (
        "ferric",
        "baseline.ferric-target-only",
        "baseline.ferric-reference",
    ),
}
REQUIRED_CELL_KINDS = tuple(CELL_ENGINES)
PRIMARY_METRICS = {
    "core-kernel": "throughput-flops-per-second",
    "serving-primary": "total-tokens-per-second",
    "speculation": "total-tokens-per-second",
    "low-acceptance": "total-tokens-per-second",
}
WORKLOAD_VALUES = {
    "batch": (1, 4, 16, 32),
    "prefill_length": (128, 512, 2048, 8192),
    "decode_kv_length": (128, 1024, 4096, 8192),
    "isl_osl": ("128x128", "1024x256", "4096x256", "512x2048"),
    "arrival": ("closed-loop", "poisson", "burst", "overload-sweep"),
    "prefix_sharing_percent": (0,),
    "draft_length": (1, 2, 4, 8),
    "acceptance": ("target-only", "low", "mixed", "high"),
}
BOOTSTRAP_ROUNDS = 2048
BOOTSTRAP_SEED = 0xF3_2026_0821
THRESHOLDS = {
    "bootstrap_confidence_percent": 95,
    "bootstrap_rounds": BOOTSTRAP_ROUNDS,
    "core_shape_min_ratio_ppm": 800_000,
    "core_weighted_geomean_min_ratio_ppm": 950_000,
    "kernel_variance_max_ppm": 20_000,
    "low_acceptance_min_ratio_ppm": 950_000,
    "metric_regression_max_ppm": 50_000,
    "public_faster_lcb_strictly_above_ppm": 1_050_000,
    "recorded_samples_min": 30,
    "serving_lcb_min_ratio_ppm": 950_000,
    "serving_starts": 3,
    "serving_variance_max_ppm": 50_000,
    "serving_windows_per_start": 10,
    "speculation_latency_regression_max_ppm": 50_000,
    "speculation_min_ratio_ppm": 1_100_000,
    "thermal_clock_drift_max_ppm": 30_000,
    "warmups_min": 10,
}
THRESHOLD_SEMANTICS = (
    "integer-v1: medians are exact rational medians; ratios are floor(numerator/"
    "denominator*1e6); paired-bootstrap-95-lcb is the sorted floor percentile "
    "at floor(0.025*N) from 2048 deterministic LCG resamples; variance and "
    "thermal/clock drift are (max-min)/median in ppm; higher primary metrics "
    "are better; p99 latency is lower; the fastest serving baseline has the "
    "largest median primary metric; equality passes except the public-faster "
    "claim, whose LCB must be strictly greater than 1.05."
)

INTAKE_KEYS = {"environment", "format", "measurements"}
IDENTITY_HASH_KEYS = {
    "baseline_protocol_sha256",
    "benchmark_protocol_sha256",
    "config_sha256",
    "dispatch_graph_sha256",
    "ferric_artifact_sha256",
    "ferric_tuning_budget_sha256",
    "generated_plan_sha256",
    "model_sha256",
    "schedule_catalog_sha256",
    "tokenizer_sha256",
    "weights_sha256",
    "workload_roster_sha256",
}
IDENTITY_NAME_KEYS = {
    "baseline_protocol_id",
    "benchmark_protocol_id",
    "dispatch_graph_id",
    "executable_id",
    "generated_plan_id",
    "schedule_id",
}
IDENTITY_KEYS = IDENTITY_HASH_KEYS | IDENTITY_NAME_KEYS
ENVIRONMENT_KEYS = {
    "affinity_sha256",
    "cache_policy_sha256",
    "clock_policy_sha256",
    "cpu_identity_sha256",
    "device_count",
    "device_model",
    "device_uuid",
    "driver_sha256",
    "environment_sha256",
    "firmware_sha256",
    "llvm_sha256",
    "numa_sha256",
    "power_policy_sha256",
    "rocm_sha256",
    "target_arch",
    "target_feature",
    "thermal_policy_sha256",
    "topology_sha256",
}
BASELINE_KEYS = {
    "config_sha256",
    "id",
    "identity_sha256",
    "kind",
    "tuning_budget_sha256",
}
MEASUREMENT_KEYS = {
    "authority",
    "baseline_roster",
    "cells",
    "environment_sha256",
    "format",
    "qualification_identities",
    "target",
    "workload_matrix",
}
CELL_KEYS = {
    "arrival_trace_sha256",
    "core_weight",
    "deterministic_admitted_plan",
    "eligible",
    "id",
    "kind",
    "output_limits_sha256",
    "p99_slo_ns",
    "primary_metric",
    "prompt_order_sha256",
    "public_faster_claim",
    "rows",
    "sampling_seed_sha256",
    "workload",
    "workload_sha256",
}
ROW_KEYS = {
    "clock_khz",
    "engine_order",
    "faults",
    "id",
    "ordinal",
    "phase",
    "server_start",
    "status",
    "temperature_millicelsius",
    "values",
    "window",
}
VALUE_KEYS = {"p99_latency_ns", "primary"}
SUMMARY_KEYS = {
    "cell_summaries",
    "core_weighted_geomean_passed",
    "faster_claim_cell_ids",
    "qualified_cell_ids",
}
REPORT_KEYS = {
    "authority",
    "baseline_roster",
    "binding_sha256",
    "environment",
    "evidence_kind",
    "format",
    "measurement_roster_relative_path",
    "measurement_roster_sha256",
    "measurement_roster_size_bytes",
    "milestone",
    "nonclaim",
    "obligation_class",
    "obligation_id",
    "obligation_state",
    "path_id",
    "path_resolution_sha256",
    "performance_policy_path",
    "performance_policy_sha256",
    "profile_id",
    "qualification_identities",
    "requirements_sha256",
    "source_roster_sha256",
    "statement_sha256",
    "summary",
    "target",
    "tcb_identity_sha256s",
    "tcb_roster_sha256",
    "threshold_semantics",
    "thresholds",
    "workload_matrix",
}
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.:+/@-]+\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
MAX_JSON_BYTES = 16_000_000
MAX_FILE_BYTES = 64_000_000
MAX_INTAKE_BYTES = 9_000_000
MAX_REPORT_BYTES = 512_000
MAX_MEASUREMENT_BYTES = 8_000_000
MAX_CELLS = 128
MAX_ROWS_PER_CELL = 256

JsonObject = dict[str, Any]
HeldFile = tuple[str, BinaryIO, os.stat_result, bytes, str]
HeldDirectoryComponent = tuple[int, str, int, os.stat_result, str]
AbsoluteDirectoryCustody = tuple[int, list[HeldDirectoryComponent], str]
PublishedFile = tuple[int, str, int, bytes, str, os.stat_result]


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


def require_name(value: Any, description: str) -> str:
    if not isinstance(value, str) or SAFE_NAME.fullmatch(value) is None:
        fail(f"invalid {description}")
    return value


def require_positive_int(value: Any, description: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        fail(f"invalid {description}")
    return value


def file_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def directory_binding(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
    )


def verify_private_directory(metadata: os.stat_result, description: str) -> None:
    if (
        not stat.S_ISDIR(metadata.st_mode)
        or stat.S_IMODE(metadata.st_mode) != 0o700
        or metadata.st_uid != os.geteuid()
    ):
        fail(f"{description} must be an exact owner-private 0700 directory")


def directory_open_flags() -> int:
    required = ("O_NOFOLLOW", "O_DIRECTORY", "O_CLOEXEC")
    if any(not hasattr(os, name) for name in required):
        fail("descriptor-relative custody requires O_NOFOLLOW/O_DIRECTORY/O_CLOEXEC")
    return os.O_RDONLY | os.O_NOFOLLOW | os.O_DIRECTORY | os.O_CLOEXEC


def authenticate_absolute_directory(
    path: Path, description: str, *, private: bool = False
) -> AbsoluteDirectoryCustody:
    absolute = lexical_absolute(path)
    if not absolute.is_absolute() or absolute.parts[0] != "/":
        fail(f"{description} must be absolute")
    root_fd = os.open("/", directory_open_flags())
    chain: list[HeldDirectoryComponent] = []
    parent_fd = root_fd
    try:
        for ordinal, component in enumerate(absolute.parts[1:], 1):
            if component in {"", ".", ".."} or "/" in component:
                fail(f"unsafe {description} component")
            descriptor = -1
            try:
                before = os.stat(component, dir_fd=parent_fd, follow_symlinks=False)
                descriptor = os.open(
                    component, directory_open_flags(), dir_fd=parent_fd
                )
                opened = os.fstat(descriptor)
            except OSError as error:
                if descriptor >= 0:
                    os.close(descriptor)
                fail(f"{description} component {ordinal} is unavailable: {error}")
            if (
                stat.S_ISLNK(before.st_mode)
                or not stat.S_ISDIR(before.st_mode)
                or directory_binding(before) != directory_binding(opened)
            ):
                os.close(descriptor)
                fail(f"{description} component {ordinal} is not a stable directory")
            chain.append(
                (
                    parent_fd,
                    component,
                    descriptor,
                    opened,
                    f"{description} component {ordinal}",
                )
            )
            parent_fd = descriptor
        if private:
            verify_private_directory(os.fstat(parent_fd), description)
        return root_fd, chain, description
    except BaseException:
        for held in reversed(chain):
            os.close(held[2])
        os.close(root_fd)
        raise


def directory_custody_fd(custody: AbsoluteDirectoryCustody) -> int:
    root_fd, chain, _ = custody
    return chain[-1][2] if chain else root_fd


def revalidate_absolute_directory(
    custody: AbsoluteDirectoryCustody, *, private: bool = False
) -> None:
    root_fd, chain, description = custody
    if not stat.S_ISDIR(os.fstat(root_fd).st_mode):
        fail(f"filesystem root changed for {description}")
    for ordinal, (parent_fd, name, descriptor, authenticated, item) in enumerate(chain):
        try:
            named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
            opened = os.fstat(descriptor)
        except OSError as error:
            fail(f"cannot revalidate {item}: {error}")
        if (
            stat.S_ISLNK(named.st_mode)
            or directory_binding(authenticated) != directory_binding(opened)
            or directory_binding(opened) != directory_binding(named)
        ):
            fail(f"{item} was replaced after it was opened")
        if private and ordinal == len(chain) - 1:
            verify_private_directory(opened, description)


def close_absolute_directory(custody: AbsoluteDirectoryCustody) -> None:
    root_fd, chain, _ = custody
    for held in reversed(chain):
        os.close(held[2])
    os.close(root_fd)


def open_regular_at(
    directory_fd: int, name: str, description: str
) -> tuple[BinaryIO, os.stat_result]:
    if not name or name in {".", ".."} or "/" in name or "\0" in name:
        fail(f"{description} name is unsafe")
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        before = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        descriptor = os.open(name, flags, dir_fd=directory_fd)
        source = os.fdopen(descriptor, "rb")
        opened = os.fstat(source.fileno())
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(before.st_mode)
        or not stat.S_ISREG(before.st_mode)
        or file_identity(before) != file_identity(opened)
    ):
        source.close()
        fail(f"{description} must be a stable regular nonsymlink file")
    return source, opened


def read_held_file_at(
    directory_fd: int,
    name: str,
    limit: int,
    description: str,
    *,
    single_link: bool = False,
) -> tuple[JsonObject, HeldFile]:
    source, before = open_regular_at(directory_fd, name, description)
    try:
        if (
            before.st_size <= 0
            or before.st_size > limit
            or stat.S_IMODE(before.st_mode) != 0o600
            or before.st_uid != os.geteuid()
            or (single_link and before.st_nlink != 1)
        ):
            fail(f"{description} is not an admitted owner-private file")
        raw = source.read(limit + 1)
        after = os.fstat(source.fileno())
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if (
            len(raw) != before.st_size
            or file_identity(before) != file_identity(after)
            or file_identity(after) != file_identity(named)
        ):
            fail(f"{description} changed while it was read")
        value = parse_canonical(raw, description)
        return value, (name, source, after, raw, description)
    except BaseException:
        source.close()
        raise


def read_held_bytes_at(
    directory_fd: int,
    name: str,
    limit: int,
    description: str,
    *,
    owner_private: bool = True,
) -> HeldFile:
    source, before = open_regular_at(directory_fd, name, description)
    try:
        if (
            before.st_size <= 0
            or before.st_size > limit
            or (
                owner_private
                and (
                    stat.S_IMODE(before.st_mode) != 0o600
                    or before.st_uid != os.geteuid()
                )
            )
        ):
            fail(f"{description} is not an admitted owner-private file")
        raw = source.read(limit + 1)
        after = os.fstat(source.fileno())
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if (
            len(raw) != before.st_size
            or file_identity(before) != file_identity(after)
            or file_identity(after) != file_identity(named)
        ):
            fail(f"{description} changed while it was read")
        return name, source, after, raw, description
    except BaseException:
        source.close()
        raise


def revalidate_held_file(directory_fd: int, held: HeldFile) -> None:
    name, source, authenticated, expected, description = held
    try:
        before = os.fstat(source.fileno())
        source.seek(0)
        raw = source.read(len(expected) + 1)
        after = os.fstat(source.fileno())
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        stat.S_ISLNK(named.st_mode)
        or raw != expected
        or file_identity(authenticated) != file_identity(before)
        or file_identity(before) != file_identity(after)
        or file_identity(after) != file_identity(named)
    ):
        fail(f"{description} changed after authentication")


def reject_duplicate_key(pairs: list[tuple[str, Any]]) -> JsonObject:
    result: JsonObject = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_nonfinite(value: str) -> NoReturn:
    fail(f"non-finite JSON number is forbidden: {value}")


def parse_canonical(raw: bytes, description: str) -> JsonObject:
    try:
        value = json.loads(
            raw,
            object_pairs_hook=reject_duplicate_key,
            parse_constant=reject_nonfinite,
        )
    except (UnicodeError, json.JSONDecodeError) as error:
        fail(f"invalid {description}: {error}")
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        fail(f"{description} is not a canonical JSON object")
    return value


def validate_identities(value: Any) -> JsonObject:
    record = exact_keys(value, IDENTITY_KEYS, "performance qualification identities")
    for key in sorted(IDENTITY_HASH_KEYS):
        require_sha256(record[key], key)
    for key in sorted(IDENTITY_NAME_KEYS):
        require_name(record[key], key)
    return record


def validate_environment(value: Any) -> JsonObject:
    record = exact_keys(value, ENVIRONMENT_KEYS, "performance environment")
    if (
        record["target_arch"] != "gfx942"
        or record["target_feature"] != "xnack-"
        or record["device_count"] != 1
        or isinstance(record["device_count"], bool)
        or record["device_model"] != "AMD Instinct MI300X"
        or not isinstance(record["device_uuid"], str)
        or not record["device_uuid"].startswith("GPU-")
        or SAFE_NAME.fullmatch(record["device_uuid"]) is None
    ):
        fail("performance intake device identity or target drifted")
    for key in sorted(
        ENVIRONMENT_KEYS
        - {
            "device_count",
            "device_model",
            "device_uuid",
            "target_arch",
            "target_feature",
            "environment_sha256",
        }
    ):
        require_sha256(record[key], f"environment {key}")
    payload = {key: item for key, item in record.items() if key != "environment_sha256"}
    if require_sha256(
        record["environment_sha256"], "environment identity"
    ) != canonical_digest(payload):
        fail("performance intake environment identity mismatch")
    return record


def validate_baselines(value: Any) -> list[JsonObject]:
    if not isinstance(value, list) or len(value) != len(BASELINE_IDS):
        fail("performance intake baseline roster is incomplete")
    for record, expected_id in zip(value, BASELINE_IDS, strict=True):
        exact_keys(record, BASELINE_KEYS, f"baseline {expected_id}")
        if record["id"] != expected_id or record["kind"] != BASELINE_KINDS[expected_id]:
            fail("performance intake baseline identity, kind, or order drifted")
        for key in ("config_sha256", "identity_sha256", "tuning_budget_sha256"):
            require_sha256(record[key], f"{expected_id} {key}")
    if len({record["tuning_budget_sha256"] for record in value[:3]}) != 1:
        fail("Ferric/vendor/vLLM/SGLang tuning budgets are not equal")
    return value


def validate_workload_matrix(value: Any) -> JsonObject:
    record = exact_keys(value, set(WORKLOAD_VALUES), "M1 workload matrix")
    for key, expected in WORKLOAD_VALUES.items():
        if record[key] != list(expected):
            fail(f"M1 workload matrix drifted: {key}")
    return record


def median(values: list[int]) -> Fraction:
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return Fraction(ordered[middle], 1)
    return Fraction(ordered[middle - 1] + ordered[middle], 2)


def ppm(numerator: Fraction, denominator: Fraction) -> int:
    if numerator <= 0 or denominator <= 0:
        fail("performance ratio has a non-positive operand")
    return (numerator * 1_000_000 // denominator).numerator


def drift_ppm(values: list[int]) -> int:
    return (
        ppm(Fraction(max(values) - min(values), 1), median(values))
        if max(values) != min(values)
        else 0
    )


def drift_within(values: list[int], threshold_ppm: int) -> bool:
    return (
        Fraction(max(values) - min(values), 1) * 1_000_000
        <= median(values) * threshold_ppm
    )


def lcg(state: int) -> int:
    return (state * 6364136223846793005 + 1442695040888963407) & ((1 << 64) - 1)


def bootstrap_lcb(ferric: list[int], baseline: list[int], cell_id: str) -> int:
    state = BOOTSTRAP_SEED ^ int.from_bytes(
        hashlib.sha256(cell_id.encode("ascii")).digest()[:8], "big"
    )
    results: list[int] = []
    count = len(ferric)
    for _ in range(BOOTSTRAP_ROUNDS):
        ferric_draw: list[int] = []
        baseline_draw: list[int] = []
        for _ in range(count):
            state = lcg(state)
            index = state % count
            ferric_draw.append(ferric[index])
            baseline_draw.append(baseline[index])
        results.append(ppm(median(ferric_draw), median(baseline_draw)))
    results.sort()
    return results[(BOOTSTRAP_ROUNDS * 25) // 1000]


def ratio_ge(left: Fraction, right: Fraction, threshold_ppm: int) -> bool:
    return left * 1_000_000 >= right * threshold_ppm


def validate_rows(cell: JsonObject) -> tuple[list[JsonObject], int, int]:
    rows = cell["rows"]
    if not isinstance(rows, list) or len(rows) > MAX_ROWS_PER_CELL:
        fail(f"invalid row roster for {cell['id']}")
    warmups = [
        row for row in rows if isinstance(row, dict) and row.get("phase") == "warmup"
    ]
    recorded = [
        row for row in rows if isinstance(row, dict) and row.get("phase") == "recorded"
    ]
    if (
        len(warmups) < THRESHOLDS["warmups_min"]
        or len(recorded) < THRESHOLDS["recorded_samples_min"]
    ):
        fail(f"insufficient warmup or recorded samples for {cell['id']}")
    if rows != warmups + recorded:
        fail(f"warmup/recorded sample order drifted for {cell['id']}")
    engines = CELL_ENGINES[cell["kind"]]
    for phase, roster in (("warmup", warmups), ("recorded", recorded)):
        for ordinal, row in enumerate(roster):
            exact_keys(row, ROW_KEYS, f"{cell['id']} {phase} row")
            expected_id = f"{cell['id']}.{phase}.{ordinal:03d}"
            rotation = ordinal % len(engines)
            expected_order = list(engines[rotation:] + engines[:rotation])
            if (
                row["id"] != expected_id
                or row["phase"] != phase
                or type(row["ordinal"]) is not int
                or row["ordinal"] != ordinal
                or row["engine_order"] != expected_order
                or row["status"] != "passed"
                or row["faults"] != []
            ):
                fail(f"dropped, failed, reordered, or faulted sample: {expected_id}")
            require_positive_int(
                row["temperature_millicelsius"], f"{expected_id} temperature"
            )
            require_positive_int(row["clock_khz"], f"{expected_id} clock")
            values = exact_keys(row["values"], set(engines), f"{expected_id} values")
            for engine in engines:
                metrics = exact_keys(
                    values[engine], VALUE_KEYS, f"{expected_id} {engine}"
                )
                require_positive_int(
                    metrics["primary"], f"{expected_id} {engine} primary"
                )
                require_positive_int(
                    metrics["p99_latency_ns"], f"{expected_id} {engine} p99"
                )
            if cell["kind"] == "serving-primary" and phase == "recorded":
                expected_start = ordinal // THRESHOLDS["serving_windows_per_start"]
                expected_window = ordinal % THRESHOLDS["serving_windows_per_start"]
                if (
                    type(row["server_start"]) is not int
                    or type(row["window"]) is not int
                    or row["server_start"] != expected_start
                    or row["window"] != expected_window
                ):
                    fail("serving start/window roster is incomplete or reordered")
            elif (
                type(row["server_start"]) is not int
                or type(row["window"]) is not int
                or row["server_start"] != -1
                or row["window"] != -1
            ):
                fail(f"unexpected server start/window annotation for {expected_id}")
    if (
        cell["kind"] == "serving-primary"
        and len(recorded)
        != THRESHOLDS["serving_starts"] * THRESHOLDS["serving_windows_per_start"]
    ):
        fail("serving qualification requires exactly three starts and ten windows")
    return recorded, len(warmups), len(recorded)


def summarize_cell(cell: JsonObject) -> JsonObject:
    exact_keys(cell, CELL_KEYS, "performance cell")
    cell_id = require_id(cell["id"], "performance cell id")
    kind = cell["kind"]
    if kind not in CELL_ENGINES:
        fail(f"unknown performance cell kind: {kind}")
    if cell["primary_metric"] != PRIMARY_METRICS[kind]:
        fail(f"primary performance metric substituted: {cell_id}")
    p99_slo_ns = require_positive_int(cell["p99_slo_ns"], f"{cell_id} p99 SLO")
    workload = exact_keys(cell["workload"], set(WORKLOAD_VALUES), f"{cell_id} workload")
    for key, choices in WORKLOAD_VALUES.items():
        if type(workload[key]) is not type(choices[0]) or workload[key] not in choices:
            fail(f"{cell_id} workload is outside the declared M1 matrix: {key}")
    protocol = {
        key: require_sha256(cell[key], f"{cell_id} {key}")
        for key in (
            "arrival_trace_sha256",
            "output_limits_sha256",
            "prompt_order_sha256",
            "sampling_seed_sha256",
        )
    }
    workload_identity = {
        **protocol,
        "dimensions": workload,
        "p99_slo_ns": cell["p99_slo_ns"],
        "primary_metric": cell["primary_metric"],
    }
    if require_sha256(
        cell["workload_sha256"], f"{cell_id} workload identity"
    ) != canonical_digest(workload_identity):
        fail(f"{cell_id} workload identity mismatch")
    if (
        type(cell["eligible"]) is not bool
        or type(cell["deterministic_admitted_plan"]) is not bool
        or type(cell["public_faster_claim"]) is not bool
    ):
        fail(f"{cell_id} boolean declaration drifted")
    if not cell["eligible"]:
        fail(f"ineligible performance cell retained: {cell_id}")
    if kind == "low-acceptance" and (
        workload["acceptance"] != "low" or not cell["deterministic_admitted_plan"]
    ):
        fail("low-acceptance cell lacks an admitted deterministic plan")
    if kind != "low-acceptance" and cell["deterministic_admitted_plan"]:
        fail(f"unexpected deterministic-plan declaration: {cell_id}")
    weight = cell["core_weight"]
    if (
        type(weight) is not int
        or (kind == "core-kernel" and not 1 <= weight <= 100)
        or (kind != "core-kernel" and weight != 0)
    ):
        fail(f"invalid core weight: {cell_id}")

    recorded, warmups, sample_count = validate_rows(cell)
    engines = CELL_ENGINES[kind]
    primary = {
        engine: [row["values"][engine]["primary"] for row in recorded]
        for engine in engines
    }
    latency = {
        engine: [row["values"][engine]["p99_latency_ns"] for row in recorded]
        for engine in engines
    }
    medians = {engine: median(values) for engine, values in primary.items()}
    latency_medians = {engine: median(values) for engine, values in latency.items()}
    if any(value > p99_slo_ns for value in latency_medians.values()):
        fail(f"engine exceeded the equal p99 SLO: {cell_id}")
    if kind == "serving-primary":
        candidates = ("baseline.vllm", "baseline.sglang")
        selected = max(
            candidates, key=lambda engine: (medians[engine], -candidates.index(engine))
        )
    elif kind == "core-kernel":
        selected = "baseline.vendor"
    else:
        selected = "baseline.ferric-target-only"
    primary_ratio = ppm(medians["ferric"], medians[selected])
    lcb = bootstrap_lcb(primary["ferric"], primary[selected], cell_id)
    reference = "baseline.ferric-reference"
    latency_comparison = (
        "baseline.ferric-target-only" if kind == "speculation" else reference
    )
    latency_regression = (
        ppm(latency_medians["ferric"], latency_medians[latency_comparison]) - 1_000_000
    )
    variance = max(drift_ppm(values) for values in primary.values())
    temperatures = [row["temperature_millicelsius"] for row in recorded]
    clocks = [row["clock_khz"] for row in recorded]
    thermal = drift_ppm(temperatures)
    clock = drift_ppm(clocks)
    variance_limit = (
        THRESHOLDS["kernel_variance_max_ppm"]
        if kind == "core-kernel"
        else THRESHOLDS["serving_variance_max_ppm"]
    )
    if (
        any(not drift_within(values, variance_limit) for values in primary.values())
        or not drift_within(temperatures, THRESHOLDS["thermal_clock_drift_max_ppm"])
        or not drift_within(clocks, THRESHOLDS["thermal_clock_drift_max_ppm"])
    ):
        fail(f"variance, thermal, or clock drift gate failed: {cell_id}")
    if not ratio_ge(
        medians["ferric"],
        medians[reference],
        1_000_000 - THRESHOLDS["metric_regression_max_ppm"],
    ):
        fail(f"primary metric regressed more than five percent: {cell_id}")
    if latency_medians["ferric"] * 1_000_000 > latency_medians[reference] * (
        1_000_000 + THRESHOLDS["metric_regression_max_ppm"]
    ):
        fail(f"p99 latency regressed more than five percent: {cell_id}")
    if kind == "core-kernel" and primary_ratio < THRESHOLDS["core_shape_min_ratio_ppm"]:
        fail(f"core shape is below the 80 percent floor: {cell_id}")
    if kind == "serving-primary" and lcb < THRESHOLDS["serving_lcb_min_ratio_ppm"]:
        fail(f"serving paired-bootstrap LCB is below 0.95: {cell_id}")
    if kind == "speculation":
        if primary_ratio < THRESHOLDS["speculation_min_ratio_ppm"]:
            fail(f"speculation throughput improvement is below ten percent: {cell_id}")
        if latency_medians["ferric"] * 1_000_000 > latency_medians[
            "baseline.ferric-target-only"
        ] * (1_000_000 + THRESHOLDS["speculation_latency_regression_max_ppm"]):
            fail(f"speculation p99 latency regression exceeds five percent: {cell_id}")
    if (
        kind == "low-acceptance"
        and primary_ratio < THRESHOLDS["low_acceptance_min_ratio_ppm"]
    ):
        fail(f"low-acceptance regression exceeds five percent: {cell_id}")
    if (
        cell["public_faster_claim"]
        and lcb <= THRESHOLDS["public_faster_lcb_strictly_above_ppm"]
    ):
        fail(f"public faster claim lacks an LCB strictly above 1.05: {cell_id}")

    def fraction_value(value: Fraction) -> int | str:
        return (
            int(value)
            if value.denominator == 1
            else f"{value.numerator}/{value.denominator}"
        )

    return {
        "baseline_primary_medians": {
            engine: fraction_value(value)
            for engine, value in medians.items()
            if engine != "ferric"
        },
        "cell_id": cell_id,
        "clock_drift_ppm": clock,
        "ferric_p99_median_ns": fraction_value(latency_medians["ferric"]),
        "ferric_primary_median": fraction_value(medians["ferric"]),
        "latency_regression_ppm": latency_regression,
        "paired_bootstrap_lcb_ppm": lcb,
        "primary_ratio_ppm": primary_ratio,
        "recorded_samples": sample_count,
        "selected_baseline_id": selected,
        "thermal_drift_ppm": thermal,
        "variance_ppm": variance,
        "warmups": warmups,
    }


def summarize_suite(measurements: JsonObject) -> JsonObject:
    cells = measurements["cells"]
    if not isinstance(cells, list) or not 1 <= len(cells) <= MAX_CELLS:
        fail("performance cell roster is empty or exceeds its bound")
    summaries = [summarize_cell(cell) for cell in cells]
    ids = [summary["cell_id"] for summary in summaries]
    if len(ids) != len(set(ids)):
        fail("performance cell roster contains duplicate identities")
    kinds = [cell["kind"] for cell in cells]
    if any(kind not in kinds for kind in REQUIRED_CELL_KINDS):
        fail("performance suite omits a required release-gate class")
    workload_ids = [cell["workload_sha256"] for cell in cells]
    if len(workload_ids) != len(set(workload_ids)):
        fail("performance suite reuses a workload identity")
    weighted_ferric = 1
    weighted_baseline = 1
    total_weight = 0
    for cell in (item for item in cells if item["kind"] == "core-kernel"):
        recorded, _, _ = validate_rows(cell)
        ferric = median([row["values"]["ferric"]["primary"] for row in recorded])
        baseline = median(
            [row["values"]["baseline.vendor"]["primary"] for row in recorded]
        )
        weight = cell["core_weight"]
        weighted_ferric *= ferric.numerator**weight * baseline.denominator**weight
        weighted_baseline *= baseline.numerator**weight * ferric.denominator**weight
        total_weight += weight
    if (
        weighted_ferric * 1_000_000**total_weight
        < weighted_baseline
        * THRESHOLDS["core_weighted_geomean_min_ratio_ppm"] ** total_weight
    ):
        fail("core weighted geometric mean is below 0.95")
    return {
        "cell_summaries": summaries,
        "core_weighted_geomean_passed": True,
        "faster_claim_cell_ids": [
            cell["id"] for cell in cells if cell["public_faster_claim"]
        ],
        "qualified_cell_ids": ids,
    }


def validate_intake(value: Any) -> tuple[JsonObject, JsonObject, JsonObject]:
    intake = exact_keys(value, INTAKE_KEYS, "performance intake")
    if intake["format"] != INTAKE_FORMAT:
        fail("performance intake format drifted")
    environment = validate_environment(intake["environment"])
    measurements = exact_keys(
        intake["measurements"], MEASUREMENT_KEYS, "performance measurements"
    )
    if (
        measurements["format"] != MEASUREMENT_FORMAT
        or measurements["authority"] != AUTHORITY
        or measurements["target"] != TARGET
    ):
        fail("performance measurement format, authority, or target drifted")
    identities = validate_identities(measurements["qualification_identities"])
    baselines = validate_baselines(measurements["baseline_roster"])
    validate_workload_matrix(measurements["workload_matrix"])
    if measurements["environment_sha256"] != environment["environment_sha256"]:
        fail("performance measurement environment substitution detected")
    if (
        identities["ferric_tuning_budget_sha256"]
        != baselines[0]["tuning_budget_sha256"]
    ):
        fail("Ferric and baseline tuning budgets are not equal")
    cells = measurements["cells"]
    if identities["workload_roster_sha256"] != canonical_digest(
        [cell.get("workload_sha256") for cell in cells]
        if isinstance(cells, list)
        else None
    ):
        fail("performance workload-roster identity mismatch")
    summary = summarize_suite(measurements)
    exact_keys(summary, SUMMARY_KEYS, "performance summary")
    return environment, measurements, summary


def run(arguments: list[str], description: str, *, cwd: Path) -> str:
    try:
        result = subprocess.run(
            arguments,
            cwd=cwd,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=300,
            env={"PATH": os.environ.get("PATH", "")},
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        fail(f"cannot {description}: {error}")
    if result.returncode != 0:
        fail(f"cannot {description}: {result.stdout.strip()}")
    return result.stdout


def git(repo: Path, arguments: list[str], description: str) -> str:
    return run(["git", "-C", str(repo), *arguments], description, cwd=repo).strip()


def repository_identity(repo: Path, description: str) -> tuple[str, str]:
    if (
        git(
            repo,
            ["rev-parse", "--is-inside-work-tree"],
            f"inspect {description} repository",
        )
        != "true"
    ):
        fail(f"{description} is not a Git worktree")
    commit = git(repo, ["rev-parse", "HEAD^{commit}"], f"resolve {description} commit")
    tree = git(repo, ["rev-parse", "HEAD^{tree}"], f"resolve {description} tree")
    if GIT_ID.fullmatch(commit) is None or GIT_ID.fullmatch(tree) is None:
        fail(f"{description} Git identity is malformed")
    if git(
        repo,
        ["status", "--porcelain=v1", "--untracked-files=all"],
        f"inspect {description} status",
    ):
        fail(f"{description} repository must be clean")
    return commit, tree


def read_bounded_path(path: Path, limit: int, description: str) -> bytes:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        before = path.lstat()
        descriptor = os.open(path, flags)
        with os.fdopen(descriptor, "rb") as source:
            opened = os.fstat(source.fileno())
            if before.st_size <= 0 or before.st_size > limit:
                fail(f"{description} size is outside the admitted bound")
            raw = source.read(limit + 1)
            after = os.fstat(source.fileno())
    except OSError as error:
        fail(f"{description} is unavailable: {error}")
    if (
        stat.S_ISLNK(before.st_mode)
        or not stat.S_ISREG(before.st_mode)
        or file_identity(before) != file_identity(opened)
        or file_identity(opened) != file_identity(after)
        or len(raw) != before.st_size
    ):
        fail(f"{description} changed while it was read")
    return raw


def safe_relative(value: Any, description: str) -> PurePosixPath:
    if not isinstance(value, str) or not value or len(value) > 4096:
        fail(f"invalid {description}")
    relative = PurePosixPath(value)
    if (
        relative.is_absolute()
        or relative.as_posix() != value
        or any(part in {"", ".", ".."} for part in relative.parts)
    ):
        fail(f"unsafe {description}")
    return relative


def ensure_private_child_directory(
    parent_fd: int, name: str, description: str
) -> tuple[int, bool]:
    created = False
    try:
        os.mkdir(name, 0o700, dir_fd=parent_fd)
        created = True
    except FileExistsError:
        pass
    except OSError as error:
        fail(f"cannot create {description}: {error}")
    descriptor = -1
    created_identity: os.stat_result | None = None
    try:
        try:
            before = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
            if created:
                created_identity = before
            descriptor = os.open(name, directory_open_flags(), dir_fd=parent_fd)
            opened = os.fstat(descriptor)
        except OSError as error:
            fail(f"cannot open {description}: {error}")
        if stat.S_ISLNK(before.st_mode) or directory_binding(
            before
        ) != directory_binding(opened):
            fail(f"{description} is not a stable directory")
        verify_private_directory(opened, description)
        if created:
            try:
                os.fsync(parent_fd)
            except OSError as error:
                fail(f"cannot sync newly created {description}: {error}")
        return descriptor, created
    except BaseException:
        cleanup_failure = None
        if created and descriptor >= 0:
            cleanup_failure = rollback_exact_directory(
                parent_fd, name, descriptor, description
            )
        elif created and created_identity is not None:
            try:
                named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
                if directory_binding(named) != directory_binding(created_identity):
                    cleanup_failure = f"cannot remove replaced failed {description}"
                else:
                    os.rmdir(name, dir_fd=parent_fd)
                    os.fsync(parent_fd)
            except FileNotFoundError:
                pass
            except OSError as error:
                cleanup_failure = f"cannot remove failed {description}: {error}"
        elif created:
            cleanup_failure = f"cannot identify failed {description} for exact rollback"
        if descriptor >= 0:
            os.close(descriptor)
        if cleanup_failure is not None:
            fail(f"{description} creation rollback failed: {cleanup_failure}")
        raise


def open_private_child_directory(parent_fd: int, name: str, description: str) -> int:
    try:
        before = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        descriptor = os.open(name, directory_open_flags(), dir_fd=parent_fd)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot open {description}: {error}")
    if stat.S_ISLNK(before.st_mode) or directory_binding(before) != directory_binding(
        opened
    ):
        os.close(descriptor)
        fail(f"{description} is not a stable directory")
    verify_private_directory(opened, description)
    return descriptor


def revalidate_child_directory(
    parent_fd: int, name: str, descriptor: int, description: str
) -> None:
    try:
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        opened = os.fstat(descriptor)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if stat.S_ISLNK(named.st_mode) or directory_binding(named) != directory_binding(
        opened
    ):
        fail(f"{description} was replaced after it was opened")
    verify_private_directory(opened, description)


def replay_plan(
    ferric: Path,
    fe2o3: Path,
    plan_raw: bytes,
    queue_raw: bytes,
) -> None:
    planner = ferric / "proofs/m1-qualification/planner.py"
    with tempfile.TemporaryDirectory(prefix="ferric-m1-performance-plan.") as raw:
        candidate = Path(raw) / "candidate"
        run(
            [
                sys.executable,
                "-I",
                str(planner),
                str(ferric),
                str(fe2o3),
                str(candidate),
            ],
            "rederive the exact M1 evidence plan",
            cwd=ferric,
        )
        if (
            read_bounded_path(
                candidate / "plan.json", MAX_JSON_BYTES, "rederived M1 plan"
            )
            != plan_raw
            or read_bounded_path(
                candidate / "missing-work.json", MAX_JSON_BYTES, "rederived M1 queue"
            )
            != queue_raw
        ):
            fail("M1 plan or work queue is not the exact current planner output")


def expected_performance_producer(binding_id: str) -> JsonObject:
    return {
        "availability": "available",
        "command": [
            "python3",
            "-I",
            PRODUCER_PATH,
            "FERRIC_REPO",
            "FE2O3_REPO",
            "PLAN_DIR",
            "PERFORMANCE_INTAKE",
            binding_id,
        ],
        "role": PRODUCER_ROLE,
    }


def validate_plan_and_select(
    ferric: Path,
    fe2o3: Path,
    plan_fd: int,
    binding_id: str,
) -> tuple[
    JsonObject,
    JsonObject,
    JsonObject,
    JsonObject,
    list[JsonObject],
    list[JsonObject],
    list[JsonObject],
    bytes,
    bytes,
    list[HeldFile],
]:
    plan, plan_held = read_held_file_at(
        plan_fd, "plan.json", MAX_JSON_BYTES, "M1 evidence plan"
    )
    queue, queue_held = read_held_file_at(
        plan_fd, "missing-work.json", MAX_JSON_BYTES, "M1 evidence work queue"
    )
    plan_raw = plan_held[3]
    queue_raw = queue_held[3]
    held = [plan_held, queue_held]
    try:
        if plan.get("format") != PLAN_FORMAT or plan.get("authority") != PLAN_AUTHORITY:
            fail("M1 performance producer requires the canonical planning-only plan")
        if (
            queue.get("format") != WORK_FORMAT
            or queue.get("authority") != PLAN_AUTHORITY
            or queue.get("status") != "INCOMPLETE"
        ):
            fail("M1 performance producer requires the canonical incomplete queue")
        items = queue.get("items")
        counts = queue.get("counts")
        if (
            not isinstance(items, list)
            or len(items) != 358
            or counts
            != {
                "available_producer_items": 358,
                "missing_items": 358,
                "missing_producer_items": 0,
            }
        ):
            fail("M1 performance producer work-queue counts drifted")
        if queue.get("plan_path") != "plan.json" or queue.get(
            "plan_sha256"
        ) != digest_bytes(plan_raw):
            fail("M1 performance producer queue does not bind its plan")
        for forbidden in ("evidence-index.json", "receipt.json"):
            try:
                os.stat(forbidden, dir_fd=plan_fd, follow_symlinks=False)
            except FileNotFoundError:
                pass
            else:
                fail("M1 performance production refuses a closure output")

        replay_plan(ferric, fe2o3, plan_raw, queue_raw)
        slots = [
            slot
            for slot in plan.get("binding_slots", [])
            if slot.get("binding", {}).get("evidence_kind") == "performance-gate"
        ]
        ids = [slot["binding"]["id"] for slot in slots]
        if (
            len(slots) != 36
            or ids != sorted(ids)
            or digest_bytes(("\n".join(ids) + "\n").encode("ascii"))
            != PERFORMANCE_ROSTER_SHA256
        ):
            fail("M1 performance binding roster drifted")
        queue_by_id = {item.get("id"): item for item in items}
        topology: list[str] = []
        artifact_topology: list[str] = []
        for slot in slots:
            binding = slot["binding"]
            artifact_id = binding["artifact_id"]
            artifact = {
                "id": artifact_id,
                "kind": "PerformanceReport",
                "path": f"artifacts/{artifact_id}.performance-report.json",
            }
            producer = expected_performance_producer(binding["id"])
            work_id = binding["id"].replace("binding.", "work.", 1)
            work = {
                "expected_artifact": artifact,
                "id": work_id,
                "producer": producer,
                "state": "missing",
                "subject": f"binding:{binding['id']}",
            }
            if (
                binding["obligation_class"] not in {"Assurance", "Roadmap"}
                or binding["profile_id"] not in {"kernel", "qualification", "runtime"}
                or binding["source_identity_id"] not in SOURCE_IDS
                or binding["tcb_ids"] != list(TCB_IDS)
                or slot.get("expected_artifact") != artifact
                or slot.get("producer") != producer
                or slot.get("state") != "missing"
                or slot.get("foundation_selectors") != []
                or queue_by_id.get(work_id) != work
            ):
                fail(f"M1 performance producer contract drifted: {binding['id']}")
            row = [
                binding["id"],
                binding["obligation_class"],
                binding["obligation_id"],
                binding["profile_id"],
                binding["path_id"],
                binding["source_identity_id"],
            ]
            topology.append("|".join(row) + "\n")
            artifact_topology.append(
                "|".join([*row, artifact_id, artifact["path"]]) + "\n"
            )
        if (
            digest_bytes("".join(topology).encode("ascii"))
            != PERFORMANCE_TOPOLOGY_SHA256
            or digest_bytes("".join(artifact_topology).encode("ascii"))
            != PERFORMANCE_ARTIFACT_TOPOLOGY_SHA256
        ):
            fail("M1 performance allocation topology drifted")
        matches = [slot for slot in slots if slot["binding"]["id"] == binding_id]
        if len(matches) != 1:
            fail(f"unknown M1 performance binding: {binding_id}")
        slot = matches[0]
        binding = slot["binding"]
        resolutions = [
            row
            for row in plan.get("path_resolutions", [])
            if row.get("id") == binding["path_id"]
        ]
        if len(resolutions) != 1:
            fail("selected M1 performance path resolution is missing")
        resolution = resolutions[0]
        if (
            resolution.get("source_identity_id") != binding["source_identity_id"]
            or binding["source_identity_id"] != f"source.{resolution.get('repository')}"
        ):
            fail("selected M1 performance path resolution drifted")
        sources = plan.get("sources")
        if not isinstance(sources, list) or [
            source.get("id") for source in sources
        ] != list(SOURCE_IDS):
            fail("M1 performance source roster drifted")
        identities = {
            source["repository"]: (source["commit"], source["tree"])
            for source in sources
        }
        if identities != {
            "ferric": repository_identity(ferric, "Ferric"),
            "fe2o3": repository_identity(fe2o3, "fe2o3"),
        }:
            fail("M1 performance source identities drifted")
        requirements = plan.get("requirements")
        requirements_raw = read_bounded_path(
            ferric / "proofs/M1_REQUIREMENTS.json",
            MAX_JSON_BYTES,
            "M1 requirements",
        )
        if (
            not isinstance(requirements, dict)
            or requirements.get("path") != "proofs/M1_REQUIREMENTS.json"
            or requirements.get("sha256") != digest_bytes(requirements_raw)
        ):
            fail("M1 performance requirements identity drifted")
        requirements_document = parse_canonical(requirements_raw, "M1 requirements")
        closures = plan.get("source_closures")
        if (
            not isinstance(closures, list)
            or len(closures) != 2
            or [row.get("artifact", {}).get("id") for row in closures]
            != ["artifact.source.fe2o3", "artifact.source.ferric"]
        ):
            fail("M1 performance source-closure roster drifted")
        plan_validators = plan.get("trusted_validators")
        if not isinstance(plan_validators, list) or not plan_validators:
            fail("M1 performance trusted-validator roster drifted")
        validators: list[JsonObject] = []
        for row in plan_validators:
            if not isinstance(row, dict) or set(row) != {
                "evidence_kind",
                "path",
                "protocol",
                "source_sha256",
            }:
                fail("M1 performance trusted-validator record drifted")
            validators.append({"availability": "ExistingFoundation", **row})
        return (
            slot,
            resolution,
            requirements,
            requirements_document,
            sources,
            closures,
            validators,
            plan_raw,
            queue_raw,
            held,
        )
    except BaseException:
        for _, source, _, _, _ in held:
            source.close()
        raise


def authenticate_tcb_reports(
    artifact_fd: int,
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
) -> tuple[list[JsonObject], list[HeldFile]]:
    roster: list[JsonObject] = []
    held: list[HeldFile] = []
    try:
        for identifier in TCB_IDS:
            name = f"artifact.{identifier}.tcb-report.json"
            _, custody = read_held_file_at(
                artifact_fd, name, MAX_REPORT_BYTES, f"M1 TCB report {identifier}"
            )
            expected = canonical_bytes(
                exact_keys(
                    tcb_report_for(
                        ferric, requirements, sources, validators, identifier
                    ),
                    TCB_REPORT_KEYS,
                    f"expected M1 TCB report {identifier}",
                )
            )
            if custody[3] != expected:
                custody[1].close()
                fail(
                    "M1 TCB report is not the exact authenticated projection: "
                    f"{identifier}"
                )
            held.append(custody)
            roster.append(
                {
                    "artifact_id": f"artifact.{identifier}",
                    "id": identifier,
                    "identity_sha256": digest_bytes(custody[3]),
                    "kind": TCB_KINDS[identifier],
                }
            )
    except BaseException:
        for _, source, _, _, _ in held:
            source.close()
        raise
    if len({row["identity_sha256"] for row in roster}) != 3:
        for _, source, _, _, _ in held:
            source.close()
        fail("M1 TCB report identities are not unique")
    return roster, held


def revalidate_tcb_reports(
    artifact_fd: int,
    held: list[HeldFile],
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
) -> None:
    for custody, identifier in zip(held, TCB_IDS, strict=True):
        revalidate_held_file(artifact_fd, custody)
        expected = canonical_bytes(
            exact_keys(
                tcb_report_for(ferric, requirements, sources, validators, identifier),
                TCB_REPORT_KEYS,
                f"expected M1 TCB report {identifier}",
            )
        )
        if custody[3] != expected:
            fail(f"M1 TCB report changed semantically: {identifier}")


def authenticate_source_closures(
    plan_fd: int, closures: list[JsonObject]
) -> tuple[int, list[HeldFile]]:
    directory_fd = open_private_child_directory(
        plan_fd, "source-closures", "M1 source-closure directory"
    )
    held: list[HeldFile] = []
    try:
        for record in closures:
            artifact = record.get("artifact")
            if not isinstance(artifact, dict):
                fail("M1 source-closure artifact declaration is malformed")
            relative = safe_relative(
                artifact.get("path"), "M1 source-closure artifact path"
            )
            if len(relative.parts) != 2 or relative.parts[0] != "source-closures":
                fail("M1 source-closure artifact path drifted")
            custody = read_held_bytes_at(
                directory_fd,
                relative.parts[1],
                MAX_FILE_BYTES,
                f"M1 source closure {artifact.get('id')}",
            )
            if (
                artifact.get("kind") != "SourceClosure"
                or artifact.get("sha256") != digest_bytes(custody[3])
                or artifact.get("size_bytes") != len(custody[3])
                or not custody[3].endswith(b"\n")
                or custody[3].endswith(b"\n\n")
                or record.get("file_count") != len(custody[3].splitlines())
            ):
                custody[1].close()
                fail("M1 source-closure bytes or declaration drifted")
            held.append(custody)
    except BaseException:
        for _, source, _, _, _ in held:
            source.close()
        os.close(directory_fd)
        raise
    return directory_fd, held


def projected_obligations(requirements: JsonObject) -> list[JsonObject]:
    rows = [
        {
            "class": "Roadmap",
            "id": record["id"],
            "path_ids": record["path_obligations"],
            "profile_ids": record["evidence_profiles"],
            "statement_sha256": digest_bytes(record["title"].encode("utf-8")),
            "status": record["obligation_state"],
        }
        for record in requirements["roadmap_requirements"]
    ]
    rows.extend(
        {
            "class": "Assurance",
            "id": record["name"],
            "path_ids": record["path_obligations"],
            "profile_ids": record["evidence_profiles"],
            "statement_sha256": digest_bytes(record["boundary"].encode("utf-8")),
            "status": record["obligation_state"],
        }
        for record in requirements["assurance_properties"]
    )
    if len(rows) != 50 or len({(row["class"], row["id"]) for row in rows}) != 50:
        fail("projected M1 obligation roster drifted")
    return rows


def projected_paths(requirements: JsonObject) -> list[JsonObject]:
    rows = [
        {
            "availability": record["availability"],
            "id": record["id"],
            "path": record["path"],
            "repository": record["repository"],
            "source_identity_id": f"source.{record['repository']}",
            "status": record["obligation_state"],
        }
        for record in requirements["path_obligations"]
    ]
    if len(rows) != 39 or len({row["id"] for row in rows}) != 39:
        fail("projected M1 path roster drifted")
    for row in rows:
        safe_relative(row["path"], f"path obligation {row['id']}")
        if row["repository"] not in {"fe2o3", "ferric"}:
            fail(f"M1 path repository drifted: {row['id']}")
    return rows


def projected_profiles(requirements: JsonObject) -> list[JsonObject]:
    return [
        {"evidence_kinds": record["kinds"], "id": record["id"]}
        for record in requirements["evidence_profiles"]
    ]


def component(
    identifier: str,
    kind: str,
    version: str,
    status: str,
    authority: str,
    identity_payload: Any,
) -> JsonObject:
    return {
        "authority": authority,
        "id": identifier,
        "identity_sha256": canonical_digest(identity_payload),
        "kind": kind,
        "status": status,
        "version": version,
    }


def component_roster(ferric: Path, sources: list[JsonObject]) -> list[JsonObject]:
    by_id = {record["id"]: record for record in sources}
    rust_toolchain = digest_bytes(
        read_bounded_path(
            ferric / "rust-toolchain.toml", MAX_FILE_BYTES, "Rust toolchain pin"
        )
    )
    try:
        verus_version = (
            read_bounded_path(
                ferric / "proofs/verus/VERUS_VERSION", 4096, "Verus version pin"
            )
            .decode("ascii")
            .removesuffix("\n")
        )
    except UnicodeError as error:
        fail(f"cannot read Verus version pin: {error}")
    if not verus_version or "\n" in verus_version:
        fail("Verus version pin is not one canonical line")
    verus_closure = digest_bytes(
        read_bounded_path(
            ferric / "proofs/verus/VERUS_CLOSURE_MANIFEST",
            MAX_FILE_BYTES,
            "Verus closure manifest",
        )
    )
    rows = [
        component(
            "compiler.amdgpu-linker",
            "Compiler",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["compiler.amdgpu-linker", "qualification-bound-external", TARGET],
        ),
        component(
            "compiler.llvm-amdgpu",
            "Compiler",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["compiler.llvm-amdgpu", "qualification-bound-external", TARGET],
        ),
        component(
            "compiler.rust",
            "Compiler",
            "1.97.1",
            "Pinned",
            "source-configuration-only",
            ["compiler.rust", "1.97.1", rust_toolchain],
        ),
        component(
            "compiler.verus",
            "Compiler",
            verus_version,
            "Pinned",
            "proof-tool-source-closure",
            ["compiler.verus", verus_version, verus_closure],
        ),
        component(
            "hardware.gfx942",
            "Hardware",
            TARGET,
            "Contracted",
            "single-device-target-only",
            ["hardware.gfx942", TARGET, "one-physical-device"],
        ),
        component(
            "runtime.amdgpu-firmware",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.amdgpu-firmware", "qualification-bound-external", TARGET],
        ),
        component(
            "runtime.amdgpu-kernel-driver",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.amdgpu-kernel-driver", "qualification-bound-external", TARGET],
        ),
        component(
            "runtime.fe2o3",
            "Runtime",
            by_id["source.fe2o3"]["commit"],
            "SourceBound",
            "exact-source-identity",
            by_id["source.fe2o3"],
        ),
        component(
            "runtime.ferric",
            "Runtime",
            by_id["source.ferric"]["commit"],
            "SourceBound",
            "exact-source-identity",
            by_id["source.ferric"],
        ),
        component(
            "runtime.hsa",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "external-identity-required",
            ["runtime.hsa", "qualification-bound-external", TARGET],
        ),
        component(
            "runtime.posix-host",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "os-filesystem-process-supervision",
            ["runtime.posix-host", "qualification-bound-external"],
        ),
        component(
            "runtime.python",
            "Runtime",
            "qualification-bound-external",
            "Contracted",
            "validator-interpreter-and-stdlib",
            ["runtime.python", "qualification-bound-external"],
        ),
    ]
    if [row["id"] for row in rows] != sorted(row["id"] for row in rows):
        fail("internal TCB component roster is not canonical")
    if len({row["identity_sha256"] for row in rows}) != len(rows):
        fail("internal TCB component identities are not unique")
    return rows


def tcb_report_for(
    ferric: Path,
    requirements: JsonObject,
    sources: list[JsonObject],
    validators: list[JsonObject],
    subject: str,
) -> JsonObject:
    structure = [
        {
            "artifact_id": f"artifact.{identifier}",
            "id": identifier,
            "kind": TCB_KINDS[identifier],
        }
        for identifier in TCB_IDS
    ]
    return {
        "authority": TCB_REPORT_AUTHORITY,
        "component_roster": component_roster(ferric, sources),
        "evidence_kind": "tcb-report",
        "format": TCB_REPORT_FORMAT,
        "milestone": "M1",
        "nonclaim": TCB_REPORT_NONCLAIM,
        "obligation_roster": projected_obligations(requirements),
        "obligation_state": "Open",
        "path_roster": projected_paths(requirements),
        "profile_roster": projected_profiles(requirements),
        "requirements_sha256": digest_bytes(
            read_bounded_path(
                ferric / "proofs/M1_REQUIREMENTS.json",
                MAX_JSON_BYTES,
                "M1 requirements",
            )
        ),
        "source_roster": sources,
        "subject_tcb_id": subject,
        "subject_tcb_kind": TCB_KINDS[subject],
        "target": TARGET,
        "tcb_structure_roster": structure,
        "validator_roster": validators,
    }


def performance_documents(
    requirements: JsonObject,
    sources: list[JsonObject],
    tcb: list[JsonObject],
    slot: JsonObject,
    resolution: JsonObject,
    intake: JsonObject,
    policy_raw: bytes,
) -> tuple[bytes, bytes]:
    environment, measurements, summary = validate_intake(intake)
    binding = slot["binding"]
    artifact_id = binding["artifact_id"]
    measurement_bytes = canonical_bytes(measurements)
    if len(measurement_bytes) > MAX_MEASUREMENT_BYTES:
        fail("canonical performance measurements exceed their admitted bound")
    if not policy_raw or len(policy_raw) > MAX_REPORT_BYTES:
        fail("held performance policy size is outside the admitted bound")
    report = {
        "authority": AUTHORITY,
        "baseline_roster": measurements["baseline_roster"],
        "binding_sha256": binding["binding_sha256"],
        "environment": environment,
        "evidence_kind": "performance-gate",
        "format": REPORT_FORMAT,
        "measurement_roster_relative_path": f"measurements/{artifact_id}.measurements.json",
        "measurement_roster_sha256": digest_bytes(measurement_bytes),
        "measurement_roster_size_bytes": len(measurement_bytes),
        "milestone": "M1",
        "nonclaim": NONCLAIM,
        "obligation_class": binding["obligation_class"],
        "obligation_id": binding["obligation_id"],
        "obligation_state": "Open",
        "path_id": binding["path_id"],
        "path_resolution_sha256": canonical_digest(resolution),
        "performance_policy_path": PERFORMANCE_POLICY_PATH,
        "performance_policy_sha256": digest_bytes(policy_raw),
        "profile_id": binding["profile_id"],
        "qualification_identities": measurements["qualification_identities"],
        "requirements_sha256": requirements["sha256"],
        "source_roster_sha256": canonical_digest(sources),
        "statement_sha256": binding["statement_sha256"],
        "summary": summary,
        "target": TARGET,
        "tcb_identity_sha256s": {row["id"]: row["identity_sha256"] for row in tcb},
        "tcb_roster_sha256": canonical_digest(tcb),
        "threshold_semantics": THRESHOLD_SEMANTICS,
        "thresholds": THRESHOLDS,
        "workload_matrix": measurements["workload_matrix"],
    }
    exact_keys(report, REPORT_KEYS, "M1 performance report")
    report_bytes = canonical_bytes(report)
    if len(report_bytes) > MAX_REPORT_BYTES:
        fail("canonical M1 performance report exceeds its admitted bound")
    return measurement_bytes, report_bytes


def published_binding(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_uid,
        metadata.st_gid,
    )


def rollback_exact_file(
    directory_fd: int, name: str, descriptor: int, description: str
) -> str | None:
    try:
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except FileNotFoundError:
        return None
    except OSError as error:
        return f"cannot inspect failed {description} publication: {error}"
    try:
        held = os.fstat(descriptor)
    except OSError as error:
        return f"cannot identify failed {description} publication: {error}"
    if (
        stat.S_ISLNK(named.st_mode)
        or not stat.S_ISREG(named.st_mode)
        or published_binding(named) != published_binding(held)
    ):
        return f"cannot remove replaced failed {description} publication"
    try:
        os.unlink(name, dir_fd=directory_fd)
        os.fsync(directory_fd)
    except OSError as error:
        return f"cannot remove failed {description} publication: {error}"
    return None


def rollback_publications(published: list[PublishedFile]) -> list[str]:
    failures: list[str] = []
    for directory_fd, name, descriptor, _, description, _ in reversed(published):
        failure = rollback_exact_file(directory_fd, name, descriptor, description)
        if failure is not None:
            failures.append(failure)
    return failures


def rollback_exact_directory(
    parent_fd: int, name: str, descriptor: int, description: str
) -> str | None:
    try:
        named = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
    except FileNotFoundError:
        return None
    except OSError as error:
        return f"cannot inspect failed {description}: {error}"
    try:
        held = os.fstat(descriptor)
    except OSError as error:
        return f"cannot identify failed {description}: {error}"
    if (
        stat.S_ISLNK(named.st_mode)
        or not stat.S_ISDIR(named.st_mode)
        or directory_binding(named) != directory_binding(held)
    ):
        return f"cannot remove replaced failed {description}"
    try:
        os.rmdir(name, dir_fd=parent_fd)
        os.fsync(parent_fd)
    except OSError as error:
        return f"cannot remove failed {description}: {error}"
    return None


def create_new_file_at(
    directory_fd: int, name: str, value: bytes, description: str
) -> int:
    flags = (
        os.O_RDWR
        | os.O_CREAT
        | os.O_EXCL
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    try:
        descriptor = os.open(name, flags, 0o600, dir_fd=directory_fd)
    except OSError as error:
        fail(f"cannot create {description} without replacement: {error}")
    try:
        created = os.fstat(descriptor)
        if (
            not stat.S_ISREG(created.st_mode)
            or stat.S_IMODE(created.st_mode) != 0o600
            or created.st_uid != os.geteuid()
            or created.st_size != 0
        ):
            fail(f"new {description} is not an exact owner-private regular file")
        remaining = memoryview(value)
        while remaining:
            written = os.write(descriptor, remaining)
            if written <= 0:
                fail(f"cannot completely write {description}")
            remaining = remaining[written:]
        os.fsync(descriptor)
        after_write = os.fstat(descriptor)
        os.lseek(descriptor, 0, os.SEEK_SET)
        raw = os.read(descriptor, len(value) + 1)
        after_read = os.fstat(descriptor)
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
        if (
            after_write.st_size != len(value)
            or raw != value
            or file_identity(after_write) != file_identity(after_read)
            or stat.S_ISLNK(named.st_mode)
            or published_binding(named) != published_binding(after_read)
        ):
            fail(f"published {description} bytes or binding changed")
    except BaseException:
        failure = rollback_exact_file(directory_fd, name, descriptor, description)
        os.close(descriptor)
        if failure is not None:
            fail(f"{description} publication rollback failed: {failure}")
        raise
    return descriptor


def verify_published_file(
    directory_fd: int,
    name: str,
    descriptor: int,
    expected: bytes,
    authenticated: os.stat_result,
    description: str,
) -> None:
    try:
        before = os.fstat(descriptor)
        os.lseek(descriptor, 0, os.SEEK_SET)
        raw = os.read(descriptor, len(expected) + 1)
        after = os.fstat(descriptor)
        named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    except OSError as error:
        fail(f"cannot revalidate {description}: {error}")
    if (
        stat.S_ISLNK(named.st_mode)
        or raw != expected
        or file_identity(authenticated) != file_identity(before)
        or file_identity(before) != file_identity(after)
        or file_identity(after) != file_identity(named)
    ):
        fail(f"published {description} bytes or binding changed after sync")


def publish_performance(
    plan_custody: AbsoluteDirectoryCustody,
    plan_fd: int,
    artifact_fd: int,
    measurement_fd: int,
    artifact_id: str,
    measurement_bytes: bytes,
    report_bytes: bytes,
    custody_check: Callable[[], None],
) -> None:
    measurement_name = f"{artifact_id}.measurements.json"
    report_name = f"{artifact_id}.performance-report.json"
    published: list[PublishedFile] = []
    try:
        revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
        )
        revalidate_child_directory(
            plan_fd, "measurements", measurement_fd, "M1 measurement directory"
        )
        revalidate_absolute_directory(plan_custody, private=True)
        for directory_fd, name in (
            (measurement_fd, measurement_name),
            (artifact_fd, report_name),
        ):
            try:
                os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            except FileNotFoundError:
                pass
            else:
                fail("performance publication refuses a preexisting output")
        custody_check()
        descriptor = create_new_file_at(
            measurement_fd,
            measurement_name,
            measurement_bytes,
            "M1 performance measurement roster",
        )
        published.append(
            (
                measurement_fd,
                measurement_name,
                descriptor,
                measurement_bytes,
                "M1 performance measurement roster",
                os.fstat(descriptor),
            )
        )
        custody_check()
        descriptor = create_new_file_at(
            artifact_fd,
            report_name,
            report_bytes,
            "M1 performance report",
        )
        published.append(
            (
                artifact_fd,
                report_name,
                descriptor,
                report_bytes,
                "M1 performance report",
                os.fstat(descriptor),
            )
        )
        os.fsync(measurement_fd)
        os.fsync(artifact_fd)
        os.fsync(plan_fd)
        revalidate_child_directory(
            plan_fd, "measurements", measurement_fd, "M1 measurement directory"
        )
        revalidate_child_directory(
            plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
        )
        revalidate_absolute_directory(plan_custody, private=True)
        custody_check()
        for (
            directory_fd,
            name,
            descriptor,
            expected,
            description,
            identity,
        ) in published:
            verify_published_file(
                directory_fd, name, descriptor, expected, identity, description
            )
    except OSError as error:
        failures = rollback_publications(published)
        if failures:
            fail(
                f"cannot durably publish M1 performance report: {error}; rollback failures: {' | '.join(failures)}"
            )
        fail(f"cannot durably publish M1 performance report: {error}")
    except BaseException:
        failures = rollback_publications(published)
        if failures:
            fail(
                "M1 performance publication rollback failures: " + " | ".join(failures)
            )
        raise
    finally:
        for _, _, descriptor, _, _, _ in published:
            try:
                os.close(descriptor)
            except OSError:
                pass


def within(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def lexical_absolute(path: str | os.PathLike[str]) -> Path:
    return Path(os.path.abspath(os.fspath(path)))


def directory_descends_from(candidate_fd: int, root_fd: int) -> bool:
    root = os.fstat(root_fd)
    root_identity = (root.st_dev, root.st_ino)
    current = os.open(".", directory_open_flags(), dir_fd=candidate_fd)
    try:
        for _ in range(4096):
            metadata = os.fstat(current)
            identity = (metadata.st_dev, metadata.st_ino)
            if identity == root_identity:
                return True
            parent = os.open("..", directory_open_flags(), dir_fd=current)
            parent_metadata = os.fstat(parent)
            if (parent_metadata.st_dev, parent_metadata.st_ino) == identity:
                os.close(parent)
                return False
            os.close(current)
            current = parent
    finally:
        os.close(current)
    fail("performance intake ancestry exceeds its admitted bound")


def produce(
    ferric_argument: str,
    fe2o3_argument: str,
    plan_argument: str,
    intake_argument: str,
    binding_id: str,
) -> None:
    ferric = lexical_absolute(ferric_argument)
    fe2o3 = lexical_absolute(fe2o3_argument)
    plan_root = lexical_absolute(plan_argument)
    intake_path = lexical_absolute(intake_argument)
    if intake_path.name in {"", ".", ".."}:
        fail("performance intake path is invalid")
    if any(within(intake_path, root) for root in (ferric, fe2o3, plan_root)):
        fail("performance intake must be external to repositories and plan output")

    ferric_custody: AbsoluteDirectoryCustody | None = None
    fe2o3_custody: AbsoluteDirectoryCustody | None = None
    plan_custody: AbsoluteDirectoryCustody | None = None
    intake_parent_custody: AbsoluteDirectoryCustody | None = None
    policy_directory_custody: AbsoluteDirectoryCustody | None = None
    plan_fd = -1
    intake_parent_fd = -1
    policy_directory_fd = -1
    artifact_fd = -1
    source_closure_fd = -1
    measurement_fd = -1
    measurement_dir_created = False
    measurement_directory_cleanup_failure: str | None = None
    publication_complete = False
    plan_files: list[HeldFile] = []
    source_closure_files: list[HeldFile] = []
    tcb_files: list[HeldFile] = []
    intake_held: HeldFile | None = None
    policy_held: HeldFile | None = None
    measurement_bytes = b""
    report_bytes = b""
    try:
        ferric_custody = authenticate_absolute_directory(
            ferric, "Ferric source repository"
        )
        fe2o3_custody = authenticate_absolute_directory(
            fe2o3, "fe2o3 source repository"
        )
        plan_custody = authenticate_absolute_directory(
            plan_root, "M1 evidence plan directory", private=True
        )
        intake_parent_custody = authenticate_absolute_directory(
            intake_path.parent, "performance intake parent directory", private=True
        )
        policy_path = ferric / PERFORMANCE_POLICY_PATH
        policy_directory_custody = authenticate_absolute_directory(
            policy_path.parent, "performance policy directory"
        )
        plan_fd = directory_custody_fd(plan_custody)
        intake_parent_fd = directory_custody_fd(intake_parent_custody)
        policy_directory_fd = directory_custody_fd(policy_directory_custody)
        if any(
            directory_descends_from(intake_parent_fd, directory_custody_fd(custody))
            for custody in (ferric_custody, fe2o3_custody, plan_custody)
        ):
            fail("performance intake aliases a repository or plan directory")
        intake, intake_held = read_held_file_at(
            intake_parent_fd,
            intake_path.name,
            MAX_INTAKE_BYTES,
            "external M1 performance intake",
            single_link=True,
        )
        policy_held = read_held_bytes_at(
            policy_directory_fd,
            policy_path.name,
            MAX_REPORT_BYTES,
            "performance policy",
            owner_private=False,
        )
        validate_intake(intake)
        (
            slot,
            resolution,
            requirements,
            requirements_document,
            sources,
            closures,
            validators,
            plan_raw,
            queue_raw,
            plan_files,
        ) = validate_plan_and_select(ferric, fe2o3, plan_fd, binding_id)
        source_closure_fd, source_closure_files = authenticate_source_closures(
            plan_fd, closures
        )
        artifact_fd = open_private_child_directory(
            plan_fd, "artifacts", "M1 artifact directory"
        )
        tcb, tcb_files = authenticate_tcb_reports(
            artifact_fd,
            ferric,
            requirements_document,
            sources,
            validators,
        )
        measurement_bytes, report_bytes = performance_documents(
            requirements,
            sources,
            tcb,
            slot,
            resolution,
            intake,
            policy_held[3],
        )
        repeated = validate_plan_and_select(ferric, fe2o3, plan_fd, binding_id)
        try:
            if repeated[7] != plan_raw or repeated[8] != queue_raw:
                fail("M1 plan or work queue changed during performance production")
            repeated_documents = performance_documents(
                repeated[2],
                repeated[4],
                tcb,
                repeated[0],
                repeated[1],
                intake,
                policy_held[3],
            )
            if repeated_documents != (measurement_bytes, report_bytes):
                fail("M1 performance inputs changed during production")
        finally:
            for _, source, _, _, _ in repeated[9]:
                source.close()

        expected_repositories = {
            source["repository"]: (source["commit"], source["tree"])
            for source in sources
        }

        def revalidate_inputs() -> None:
            for held in plan_files:
                revalidate_held_file(plan_fd, held)
            for held in source_closure_files:
                revalidate_held_file(source_closure_fd, held)
            revalidate_tcb_reports(
                artifact_fd,
                tcb_files,
                ferric,
                requirements_document,
                sources,
                validators,
            )
            if intake_held is None:
                fail("performance intake custody is missing")
            revalidate_held_file(intake_parent_fd, intake_held)
            if policy_held is None:
                fail("performance policy custody is missing")
            revalidate_held_file(policy_directory_fd, policy_held)
            revalidate_absolute_directory(ferric_custody)
            revalidate_absolute_directory(fe2o3_custody)
            revalidate_absolute_directory(plan_custody, private=True)
            revalidate_absolute_directory(intake_parent_custody, private=True)
            revalidate_absolute_directory(policy_directory_custody)
            if expected_repositories != {
                "ferric": repository_identity(ferric, "Ferric"),
                "fe2o3": repository_identity(fe2o3, "fe2o3"),
            }:
                fail("source repository changed during performance production")
            for forbidden in ("evidence-index.json", "receipt.json"):
                try:
                    os.stat(forbidden, dir_fd=plan_fd, follow_symlinks=False)
                except FileNotFoundError:
                    pass
                else:
                    fail("performance producer created a forbidden closure output")
            revalidate_child_directory(
                plan_fd, "artifacts", artifact_fd, "M1 artifact directory"
            )
            revalidate_child_directory(
                plan_fd,
                "source-closures",
                source_closure_fd,
                "M1 source-closure directory",
            )

        revalidate_inputs()
        measurement_fd, measurement_dir_created = ensure_private_child_directory(
            plan_fd, "measurements", "M1 measurement directory"
        )
        publish_performance(
            plan_custody,
            plan_fd,
            artifact_fd,
            measurement_fd,
            slot["binding"]["artifact_id"],
            measurement_bytes,
            report_bytes,
            revalidate_inputs,
        )
        publication_complete = True
    finally:
        if intake_held is not None:
            intake_held[1].close()
        if policy_held is not None:
            policy_held[1].close()
        for _, source, _, _, _ in plan_files:
            source.close()
        for _, source, _, _, _ in source_closure_files:
            source.close()
        for _, source, _, _, _ in tcb_files:
            source.close()
        if (
            not publication_complete
            and measurement_dir_created
            and plan_fd >= 0
            and measurement_fd >= 0
        ):
            measurement_directory_cleanup_failure = rollback_exact_directory(
                plan_fd,
                "measurements",
                measurement_fd,
                "M1 measurement directory",
            )
        if measurement_fd >= 0:
            os.close(measurement_fd)
        if artifact_fd >= 0:
            os.close(artifact_fd)
        if source_closure_fd >= 0:
            os.close(source_closure_fd)
        if intake_parent_custody is not None:
            close_absolute_directory(intake_parent_custody)
        if policy_directory_custody is not None:
            close_absolute_directory(policy_directory_custody)
        if plan_custody is not None:
            close_absolute_directory(plan_custody)
        if fe2o3_custody is not None:
            close_absolute_directory(fe2o3_custody)
        if ferric_custody is not None:
            close_absolute_directory(ferric_custody)
        if measurement_directory_cleanup_failure is not None:
            fail(measurement_directory_cleanup_failure)
    print(
        f"PASS: produced M1 performance report binding={binding_id} "
        f"report_sha256={digest_bytes(report_bytes)}"
    )


def main() -> None:
    if len(sys.argv) != 6:
        fail(
            f"usage: {sys.argv[0]} FERRIC_REPO FE2O3_REPO PLAN_DIR "
            "PERFORMANCE_INTAKE binding.NNNNN"
        )
    produce(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5])


if __name__ == "__main__":
    main()
