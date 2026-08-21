#!/usr/bin/env python3
"""Validate one canonical, identity-bound M1 performance report."""

from __future__ import annotations

from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, BinaryIO, NoReturn


PROTOCOL = "ferric.m1-validator.performance-report.v1"
INDEX_FORMAT = "ferric.m1-evidence-index.v1"
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
FERRIC_BASE_COMMIT = "c5a86fd56c1c817664593df25c04bbed30e84971"
PERFORMANCE_POLICY_PATH = "docs/PERFORMANCE.md"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
GIT_ID = re.compile(r"[0-9a-f]{40}\Z")
SAFE_ID = re.compile(r"[a-z0-9][a-z0-9.-]*\Z")
SAFE_NAME = re.compile(r"[A-Za-z0-9_.:+/@-]+\Z")
SAFE_SEGMENT = re.compile(r"[A-Za-z0-9_.-]+\Z")
MAX_CONTEXT_BYTES = 1_000_000
MAX_REPORT_BYTES = 512_000
MAX_MEASUREMENT_BYTES = 8_000_000
MAX_CELLS = 128
MAX_ROWS_PER_CELL = 256
BOOTSTRAP_ROUNDS = 2048
BOOTSTRAP_SEED = 0xF3_2026_0821

TCB_IDS = ("tcb.compiler", "tcb.hardware", "tcb.runtime")
TCB_KINDS = {
    "tcb.compiler": "Compiler",
    "tcb.hardware": "Hardware",
    "tcb.runtime": "Runtime",
}
SOURCE_IDS = ("source.fe2o3", "source.ferric")
SOURCE_REPOSITORIES = {"source.fe2o3": "fe2o3", "source.ferric": "ferric"}
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

CONTEXT_KEYS = {
    "artifact",
    "artifact_absolute_path",
    "binding",
    "format",
    "path_resolution",
    "requirements_sha256",
    "sources",
    "subject",
    "tcb",
}
ARTIFACT_KEYS = {"id", "kind", "path", "sha256", "size_bytes"}
BINDING_KEYS = {
    "artifact_id",
    "binding_sha256",
    "evidence_kind",
    "id",
    "obligation_class",
    "obligation_id",
    "path_id",
    "profile_id",
    "source_identity_id",
    "statement_sha256",
    "tcb_ids",
}
PATH_KEYS = {"availability", "id", "path", "repository", "source_identity_id"}
SOURCE_KEYS = {
    "base_commit",
    "commit",
    "id",
    "repository",
    "source_closure_artifact_id",
    "source_closure_sha256",
    "tree",
}
TCB_KEYS = {"artifact_id", "id", "identity_sha256", "kind"}
REPORT_KEYS = {
    "authority",
    "baseline_roster",
    "binding_sha256",
    "environment",
    "evidence_kind",
    "format",
    "milestone",
    "measurement_roster_relative_path",
    "measurement_roster_sha256",
    "measurement_roster_size_bytes",
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
WORKLOAD_KEYS = set(WORKLOAD_VALUES)
SUMMARY_KEYS = {
    "cell_summaries",
    "core_weighted_geomean_passed",
    "faster_claim_cell_ids",
    "qualified_cell_ids",
}
CELL_SUMMARY_KEYS = {
    "baseline_primary_medians",
    "cell_id",
    "clock_drift_ppm",
    "ferric_p99_median_ns",
    "ferric_primary_median",
    "latency_regression_ppm",
    "paired_bootstrap_lcb_ppm",
    "primary_ratio_ppm",
    "recorded_samples",
    "selected_baseline_id",
    "thermal_drift_ppm",
    "variance_ppm",
    "warmups",
}


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def exact_keys(value: Any, expected: set[str], description: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        fail(f"{description} fields drifted")
    return value


def reject_duplicate_key(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_nonfinite(value: str) -> NoReturn:
    fail(f"non-finite JSON number is forbidden: {value}")


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_digest(value: Any) -> str:
    return digest_bytes(
        json.dumps(
            value, ensure_ascii=True, separators=(",", ":"), sort_keys=True
        ).encode("ascii")
    )


def require_sha256(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or SHA256.fullmatch(value) is None
        or len(set(value)) == 1
    ):
        fail(f"invalid {description}")
    return value


def require_git_id(value: Any, description: str) -> str:
    if (
        not isinstance(value, str)
        or GIT_ID.fullmatch(value) is None
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


def safe_relative(value: Any, description: str) -> PurePosixPath:
    if not isinstance(value, str) or not value or len(value) > 4096:
        fail(f"invalid {description}")
    path = PurePosixPath(value)
    if (
        path.is_absolute()
        or path.as_posix() != value
        or any(
            part in ("", ".", "..") or SAFE_SEGMENT.fullmatch(part) is None
            for part in path.parts
        )
    ):
        fail(f"unsafe {description}")
    return path


def file_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def read_bounded(path: Path, limit: int, description: str) -> bytes:
    try:
        before = path.lstat()
        if stat.S_ISLNK(before.st_mode) or not stat.S_ISREG(before.st_mode):
            fail(f"{description} must be a regular non-symlink file")
        if before.st_size <= 0 or before.st_size > limit:
            fail(f"{description} size is outside its bound")
        flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags)
        try:
            with os.fdopen(descriptor, "rb", closefd=False) as source:
                opened = os.fstat(descriptor)
                if file_identity(before) != file_identity(opened):
                    fail(f"{description} changed before open")
                payload = read_exact(source, limit, description)
                after = os.fstat(descriptor)
                if file_identity(opened) != file_identity(after):
                    fail(f"{description} changed during read")
        finally:
            os.close(descriptor)
        final = path.lstat()
        if file_identity(before) != file_identity(final):
            fail(f"{description} was replaced during read")
    except OSError as error:
        fail(f"cannot read {description}: {error}")
    return payload


def read_exact(source: BinaryIO, limit: int, description: str) -> bytes:
    payload = source.read(limit + 1)
    if len(payload) > limit:
        fail(f"{description} exceeds its size bound")
    return payload


def reject_symlink_components(
    root: Path, relative: PurePosixPath, description: str
) -> Path:
    current = root
    try:
        root_meta = root.lstat()
    except OSError as error:
        fail(f"cannot inspect evidence root: {error}")
    if stat.S_ISLNK(root_meta.st_mode) or not stat.S_ISDIR(root_meta.st_mode):
        fail("evidence root must be a regular directory")
    for index, part in enumerate(relative.parts):
        current = current / part
        try:
            metadata = current.lstat()
        except OSError as error:
            fail(f"cannot inspect {description}: {error}")
        if stat.S_ISLNK(metadata.st_mode):
            fail(f"{description} traverses a symlink")
        if index < len(relative.parts) - 1 and not stat.S_ISDIR(metadata.st_mode):
            fail(f"{description} parent is not a directory")
    return current


def parse_canonical(payload: bytes, description: str) -> dict[str, Any]:
    try:
        value = json.loads(
            payload,
            object_pairs_hook=reject_duplicate_key,
            parse_constant=reject_nonfinite,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"cannot parse {description}: {error}")
    if not isinstance(value, dict):
        fail(f"{description} must be a JSON object")
    expected = (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")
    if payload != expected:
        fail(f"{description} is not canonical JSON")
    return value


def load_context() -> tuple[dict[str, Any], bytes]:
    payload = sys.stdin.buffer.read(MAX_CONTEXT_BYTES + 1)
    if not payload or len(payload) > MAX_CONTEXT_BYTES:
        fail("performance validator context is empty or too large")
    try:
        value = json.loads(
            payload,
            object_pairs_hook=reject_duplicate_key,
            parse_constant=reject_nonfinite,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"performance validator context is invalid JSON: {error}")
    if not isinstance(value, dict):
        fail("performance validator context must be an object")
    expected = (
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    ).encode("ascii")
    if payload != expected:
        fail("performance validator context is not canonical")
    exact_keys(value, CONTEXT_KEYS, "performance validator context")
    return value, payload[:-1]


def evidence_root(report_path: Path, relative: PurePosixPath) -> Path:
    root = report_path
    for _ in relative.parts:
        root = root.parent
    expected = reject_symlink_components(root, relative, "performance report")
    if expected != report_path:
        fail("performance report absolute and relative paths disagree")
    return root


def validate_sources(value: Any, requirements: dict[str, Any]) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(SOURCE_IDS):
        fail("performance source roster is incomplete")
    for record, expected_id in zip(value, SOURCE_IDS, strict=True):
        exact_keys(record, SOURCE_KEYS, f"source context {expected_id}")
        if (
            record["id"] != expected_id
            or record["repository"] != SOURCE_REPOSITORIES[expected_id]
        ):
            fail("performance source order, identity, or repository drifted")
        require_git_id(record["base_commit"], f"{expected_id} base commit")
        require_git_id(record["commit"], f"{expected_id} commit")
        require_git_id(record["tree"], f"{expected_id} tree")
        require_id(
            record["source_closure_artifact_id"], f"{expected_id} closure artifact"
        )
        require_sha256(record["source_closure_sha256"], f"{expected_id} source closure")
    if value[0]["base_commit"] != requirements["m1_upstream_base_commit"]:
        fail("performance fe2o3 base identity drifted")
    if value[1]["base_commit"] != FERRIC_BASE_COMMIT:
        fail("performance Ferric base identity drifted")
    return value


def validate_tcb(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(TCB_IDS):
        fail("performance TCB roster is incomplete")
    for record, expected_id in zip(value, TCB_IDS, strict=True):
        exact_keys(record, TCB_KEYS, f"TCB context {expected_id}")
        if record["id"] != expected_id or record["kind"] != TCB_KINDS[expected_id]:
            fail("performance TCB order, identity, or kind drifted")
        require_id(record["artifact_id"], f"{expected_id} artifact")
        require_sha256(record["identity_sha256"], f"{expected_id} identity")
    return value


def requirements_spec(
    requirements: dict[str, Any], obligation_class: str, obligation_id: str
) -> tuple[dict[str, Any], str]:
    if any(
        record["obligation_state"] != "Open"
        for key in ("roadmap_requirements", "assurance_properties", "path_obligations")
        for record in requirements[key]
    ):
        fail("performance validation requires every M1 state to remain Open")
    key, name_key, statement_key = (
        ("roadmap_requirements", "id", "title")
        if obligation_class == "Roadmap"
        else ("assurance_properties", "name", "boundary")
        if obligation_class == "Assurance"
        else fail("performance obligation class drifted")
    )
    matches = [
        record for record in requirements[key] if record[name_key] == obligation_id
    ]
    if len(matches) != 1:
        fail("performance binding names an unknown obligation")
    return matches[0], matches[0][statement_key]


def validate_identities(value: Any) -> dict[str, str]:
    record = exact_keys(value, IDENTITY_KEYS, "qualification identities")
    for key in sorted(IDENTITY_HASH_KEYS):
        require_sha256(record[key], key)
    for key in sorted(IDENTITY_NAME_KEYS):
        require_name(record[key], key)
    return record


def validate_environment(value: Any) -> dict[str, Any]:
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
        fail("performance device identity or target drifted")
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
        fail("performance environment identity mismatch")
    return record


def validate_baselines(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list) or len(value) != len(BASELINE_IDS):
        fail("performance baseline roster is incomplete")
    for record, expected_id in zip(value, BASELINE_IDS, strict=True):
        exact_keys(record, BASELINE_KEYS, f"baseline {expected_id}")
        if record["id"] != expected_id or record["kind"] != BASELINE_KINDS[expected_id]:
            fail("performance baseline identity, kind, or order drifted")
        for key in ("config_sha256", "identity_sha256", "tuning_budget_sha256"):
            require_sha256(record[key], f"{expected_id} {key}")
    tuning = {record["tuning_budget_sha256"] for record in value[:3]}
    if len(tuning) != 1:
        fail("Ferric/vendor/vLLM/SGLang tuning budgets are not equal")
    return value


def validate_workload_matrix(value: Any) -> dict[str, list[Any]]:
    record = exact_keys(value, WORKLOAD_KEYS, "M1 workload matrix")
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
    return Fraction(max(values) - min(values), 1) * 1_000_000 <= (
        median(values) * threshold_ppm
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


def validate_rows(cell: dict[str, Any]) -> tuple[list[dict[str, Any]], int, int]:
    rows = cell["rows"]
    if not isinstance(rows, list) or len(rows) > MAX_ROWS_PER_CELL:
        fail(f"invalid row roster for {cell['id']}")
    engines = CELL_ENGINES[cell["kind"]]
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
    expected_rows = warmups + recorded
    if rows != expected_rows:
        fail(f"warmup/recorded sample order drifted for {cell['id']}")
    for phase, roster in (("warmup", warmups), ("recorded", recorded)):
        for ordinal, row in enumerate(roster):
            exact_keys(row, ROW_KEYS, f"{cell['id']} {phase} row")
            expected_id = f"{cell['id']}.{phase}.{ordinal:03d}"
            expected_order = list(
                engines[ordinal % len(engines) :] + engines[: ordinal % len(engines)]
            )
            if (
                row["id"] != expected_id
                or row["phase"] != phase
                or not isinstance(row["ordinal"], int)
                or isinstance(row["ordinal"], bool)
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
                    not isinstance(row["server_start"], int)
                    or isinstance(row["server_start"], bool)
                    or not isinstance(row["window"], int)
                    or isinstance(row["window"], bool)
                    or row["server_start"] != expected_start
                    or row["window"] != expected_window
                ):
                    fail("serving start/window roster is incomplete or reordered")
            elif (
                not isinstance(row["server_start"], int)
                or isinstance(row["server_start"], bool)
                or not isinstance(row["window"], int)
                or isinstance(row["window"], bool)
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


def summarize_cell(cell: dict[str, Any]) -> dict[str, Any]:
    exact_keys(cell, CELL_KEYS, "performance cell")
    cell_id = require_id(cell["id"], "performance cell id")
    kind = cell["kind"]
    if kind not in CELL_ENGINES:
        fail(f"unknown performance cell kind: {kind}")
    if cell["primary_metric"] != PRIMARY_METRICS[kind]:
        fail(f"primary performance metric substituted: {cell_id}")
    p99_slo_ns = require_positive_int(cell["p99_slo_ns"], f"{cell_id} p99 SLO")
    workload = exact_keys(cell["workload"], WORKLOAD_KEYS, f"{cell_id} workload")
    for key, choices in WORKLOAD_VALUES.items():
        if type(workload[key]) is not type(choices[0]) or workload[key] not in choices:
            fail(f"{cell_id} workload is outside the declared M1 matrix: {key}")
    workload_protocol = {
        key: require_sha256(cell[key], f"{cell_id} {key}")
        for key in (
            "arrival_trace_sha256",
            "output_limits_sha256",
            "prompt_order_sha256",
            "sampling_seed_sha256",
        )
    }
    workload_identity = {
        **workload_protocol,
        "dimensions": workload,
        "p99_slo_ns": cell["p99_slo_ns"],
        "primary_metric": cell["primary_metric"],
    }
    if require_sha256(
        cell["workload_sha256"], f"{cell_id} workload identity"
    ) != canonical_digest(workload_identity):
        fail(f"{cell_id} workload identity mismatch")
    if (
        not isinstance(cell["eligible"], bool)
        or not isinstance(cell["deterministic_admitted_plan"], bool)
        or not isinstance(cell["public_faster_claim"], bool)
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
    if not isinstance(weight, int) or isinstance(weight, bool):
        fail(f"invalid core weight: {cell_id}")
    if (
        kind == "core-kernel"
        and require_positive_int(weight, f"{cell_id} weight") > 100
    ) or (kind != "core-kernel" and weight != 0):
        fail(f"invalid core weight: {cell_id}")

    recorded, warmup_count, sample_count = validate_rows(cell)
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
    temperature_values = [row["temperature_millicelsius"] for row in recorded]
    clock_values = [row["clock_khz"] for row in recorded]
    thermal = drift_ppm(temperature_values)
    clock = drift_ppm(clock_values)
    variance_limit = (
        THRESHOLDS["kernel_variance_max_ppm"]
        if kind == "core-kernel"
        else THRESHOLDS["serving_variance_max_ppm"]
    )
    if (
        any(not drift_within(values, variance_limit) for values in primary.values())
        or not drift_within(
            temperature_values, THRESHOLDS["thermal_clock_drift_max_ppm"]
        )
        or not drift_within(clock_values, THRESHOLDS["thermal_clock_drift_max_ppm"])
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
    return {
        "baseline_primary_medians": {
            engine: int(value)
            if value.denominator == 1
            else f"{value.numerator}/{value.denominator}"
            for engine, value in medians.items()
            if engine != "ferric"
        },
        "cell_id": cell_id,
        "clock_drift_ppm": clock,
        "ferric_p99_median_ns": int(latency_medians["ferric"])
        if latency_medians["ferric"].denominator == 1
        else f"{latency_medians['ferric'].numerator}/{latency_medians['ferric'].denominator}",
        "ferric_primary_median": int(medians["ferric"])
        if medians["ferric"].denominator == 1
        else f"{medians['ferric'].numerator}/{medians['ferric'].denominator}",
        "latency_regression_ppm": latency_regression,
        "paired_bootstrap_lcb_ppm": lcb,
        "primary_ratio_ppm": primary_ratio,
        "recorded_samples": sample_count,
        "selected_baseline_id": selected,
        "thermal_drift_ppm": thermal,
        "variance_ppm": variance,
        "warmups": warmup_count,
    }


def summarize_suite(measurements: dict[str, Any]) -> dict[str, Any]:
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

    core_cells = [cell for cell in cells if cell["kind"] == "core-kernel"]
    weighted_ferric = 1
    weighted_baseline = 1
    total_weight = 0
    for cell in core_cells:
        recorded, _, _ = validate_rows(cell)
        ferric = median([row["values"]["ferric"]["primary"] for row in recorded])
        baseline = median(
            [row["values"]["baseline.vendor"]["primary"] for row in recorded]
        )
        weight = cell["core_weight"]
        weighted_ferric *= ferric.numerator**weight * baseline.denominator**weight
        weighted_baseline *= baseline.numerator**weight * ferric.denominator**weight
        total_weight += weight
    geomean_passed = (
        weighted_ferric * 1_000_000**total_weight
        >= weighted_baseline
        * THRESHOLDS["core_weighted_geomean_min_ratio_ppm"] ** total_weight
    )
    if not geomean_passed:
        fail("core weighted geometric mean is below 0.95")
    return {
        "cell_summaries": summaries,
        "core_weighted_geomean_passed": True,
        "faster_claim_cell_ids": [
            cell["id"] for cell in cells if cell["public_faster_claim"]
        ],
        "qualified_cell_ids": ids,
    }


def validate(context: dict[str, Any]) -> None:
    if context["format"] != INDEX_FORMAT:
        fail("performance context index format drifted")
    repo = Path(__file__).resolve(strict=True).parents[3]
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements_raw = read_bounded(
        requirements_path, MAX_REPORT_BYTES, "M1 requirements manifest"
    )
    requirements = parse_canonical(requirements_raw, "M1 requirements manifest")
    requirements_sha256 = require_sha256(
        context["requirements_sha256"], "requirements SHA-256"
    )
    if requirements_sha256 != digest_bytes(requirements_raw):
        fail("performance context requirements identity drifted")

    artifact = exact_keys(
        context["artifact"], ARTIFACT_KEYS, "performance artifact context"
    )
    artifact_id = require_id(artifact["id"], "performance artifact id")
    if artifact["kind"] != "PerformanceReport":
        fail("performance report artifact kind drifted")
    report_relative = safe_relative(artifact["path"], "performance report path")
    if report_relative.as_posix() != f"artifacts/{artifact_id}.performance-report.json":
        fail("performance report path is not canonical for its artifact id")
    report_sha256 = require_sha256(artifact["sha256"], "performance report SHA-256")
    report_size = require_positive_int(
        artifact["size_bytes"], "performance report size"
    )
    if not isinstance(context["artifact_absolute_path"], str):
        fail("performance report absolute path is invalid")
    report_path = Path(context["artifact_absolute_path"])
    root = evidence_root(report_path, report_relative)
    report_raw = read_bounded(report_path, MAX_REPORT_BYTES, "performance report")
    if len(report_raw) != report_size or digest_bytes(report_raw) != report_sha256:
        fail("performance report bytes do not match the evidence context")
    report = exact_keys(
        parse_canonical(report_raw, "performance report"),
        REPORT_KEYS,
        "performance report",
    )

    binding = exact_keys(
        context["binding"], BINDING_KEYS, "performance binding context"
    )
    for key in ("artifact_id", "id"):
        require_id(binding[key], f"binding {key}")
    for key in ("obligation_id", "path_id", "profile_id", "source_identity_id"):
        require_name(binding[key], f"binding {key}")
    if (
        context["subject"] != f"binding:{binding['id']}"
        or binding["artifact_id"] != artifact_id
        or binding["evidence_kind"] != "performance-gate"
        or binding["source_identity_id"] not in SOURCE_IDS
        or binding["tcb_ids"] != list(TCB_IDS)
    ):
        fail("performance binding context drifted")
    binding_payload = {
        key: value for key, value in binding.items() if key != "binding_sha256"
    }
    if require_sha256(binding["binding_sha256"], "binding SHA-256") != canonical_digest(
        binding_payload
    ):
        fail("performance binding identity mismatch")
    spec, statement = requirements_spec(
        requirements, binding["obligation_class"], binding["obligation_id"]
    )
    profiles = {
        record["id"]: record["kinds"] for record in requirements["evidence_profiles"]
    }
    if (
        binding["profile_id"] not in spec["evidence_profiles"]
        or "performance-gate" not in profiles.get(binding["profile_id"], [])
        or binding["path_id"] not in spec["path_obligations"]
        or require_sha256(binding["statement_sha256"], "statement SHA-256")
        != digest_bytes(statement.encode("utf-8"))
    ):
        fail("performance obligation, profile, path, or statement drifted")
    resolution = exact_keys(
        context["path_resolution"], PATH_KEYS, "performance path resolution"
    )
    paths = {record["id"]: record for record in requirements["path_obligations"]}
    expected_path = paths.get(binding["path_id"])
    if (
        expected_path is None
        or resolution["id"] != binding["path_id"]
        or resolution["availability"] != expected_path["availability"]
        or resolution["path"] != expected_path["path"]
        or resolution["repository"] != expected_path["repository"]
        or resolution["source_identity_id"] != binding["source_identity_id"]
        or binding["source_identity_id"] != f"source.{expected_path['repository']}"
    ):
        fail("performance path resolution drifted")
    sources = validate_sources(context["sources"], requirements)
    tcb = validate_tcb(context["tcb"])

    policy_path = repo / PERFORMANCE_POLICY_PATH
    policy_raw = read_bounded(policy_path, MAX_REPORT_BYTES, "performance policy")
    identities = validate_identities(report["qualification_identities"])
    environment = validate_environment(report["environment"])
    baselines = validate_baselines(report["baseline_roster"])
    matrix = validate_workload_matrix(report["workload_matrix"])
    if (
        report["thresholds"] != THRESHOLDS
        or report["threshold_semantics"] != THRESHOLD_SEMANTICS
    ):
        fail("performance threshold declaration drifted")
    if (
        identities["ferric_tuning_budget_sha256"]
        != baselines[0]["tuning_budget_sha256"]
    ):
        fail("Ferric and baseline tuning budgets are not equal")

    measurement_relative = safe_relative(
        report["measurement_roster_relative_path"], "measurement roster path"
    )
    if (
        measurement_relative.as_posix()
        != f"measurements/{artifact_id}.measurements.json"
    ):
        fail("measurement roster path is not canonical for its artifact id")
    measurement_path = reject_symlink_components(
        root, measurement_relative, "measurement roster"
    )
    measurement_raw = read_bounded(
        measurement_path, MAX_MEASUREMENT_BYTES, "measurement roster"
    )
    if require_positive_int(
        report["measurement_roster_size_bytes"], "measurement roster size"
    ) != len(measurement_raw) or require_sha256(
        report["measurement_roster_sha256"], "measurement roster SHA-256"
    ) != digest_bytes(measurement_raw):
        fail("measurement roster bytes do not match the report identity")
    measurements = exact_keys(
        parse_canonical(measurement_raw, "measurement roster"),
        MEASUREMENT_KEYS,
        "measurement roster",
    )
    if (
        measurements["format"] != MEASUREMENT_FORMAT
        or measurements["authority"] != AUTHORITY
        or measurements["target"] != TARGET
        or measurements["environment_sha256"] != environment["environment_sha256"]
        or measurements["qualification_identities"] != identities
        or measurements["baseline_roster"] != baselines
        or measurements["workload_matrix"] != matrix
    ):
        fail("measurement roster identity or environment substitution detected")
    if identities["workload_roster_sha256"] != canonical_digest(
        [cell.get("workload_sha256") for cell in measurements["cells"]]
        if isinstance(measurements["cells"], list)
        else None
    ):
        fail("workload-roster identity mismatch")
    expected_summary = summarize_suite(measurements)
    exact_keys(report["summary"], SUMMARY_KEYS, "performance report summary")
    if any(
        not isinstance(item, dict) or set(item) != CELL_SUMMARY_KEYS
        for item in report["summary"]["cell_summaries"]
    ):
        fail("performance cell-summary fields drifted")
    if report["summary"] != expected_summary:
        fail("performance report arithmetic or threshold summary drifted")

    expected_tcb = {record["id"]: record["identity_sha256"] for record in tcb}
    if (
        report["format"] != REPORT_FORMAT
        or report["authority"] != AUTHORITY
        or report["nonclaim"] != NONCLAIM
        or report["evidence_kind"] != "performance-gate"
        or report["milestone"] != "M1"
        or report["target"] != TARGET
        or report["requirements_sha256"] != requirements_sha256
        or report["binding_sha256"] != binding["binding_sha256"]
        or report["obligation_class"] != binding["obligation_class"]
        or report["obligation_id"] != binding["obligation_id"]
        or report["obligation_state"] != "Open"
        or report["statement_sha256"] != binding["statement_sha256"]
        or report["path_id"] != binding["path_id"]
        or report["profile_id"] != binding["profile_id"]
        or report["path_resolution_sha256"] != canonical_digest(resolution)
        or report["source_roster_sha256"] != canonical_digest(sources)
        or report["tcb_identity_sha256s"] != expected_tcb
        or report["tcb_roster_sha256"] != canonical_digest(tcb)
        or report["performance_policy_path"] != PERFORMANCE_POLICY_PATH
        or report["performance_policy_sha256"] != digest_bytes(policy_raw)
    ):
        fail("performance report content, source, binding, policy, or TCB drifted")


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] != PROTOCOL:
        fail("performance validator protocol mismatch")
    context, payload = load_context()
    validate(context)
    print(
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(payload)}"
    )


if __name__ == "__main__":
    main()
