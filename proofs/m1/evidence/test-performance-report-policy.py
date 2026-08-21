#!/usr/bin/env python3
"""Exercise canonical and hostile M1 performance evidence."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Callable, NoReturn


PROTOCOL = "ferric.m1-validator.performance-report.v1"
Mutation = Callable[[Path, Path, dict[str, Any], dict[str, Any], dict[str, Any]], None]


def fail(message: str) -> NoReturn:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("ascii")


def load_module(path: Path, name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        fail(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def make_sources(module: Any, requirements: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "base_commit": requirements["m1_upstream_base_commit"],
            "commit": digest_bytes(b"fe2o3 performance commit")[:40],
            "id": "source.fe2o3",
            "repository": "fe2o3",
            "source_closure_artifact_id": "artifact.source.fe2o3",
            "source_closure_sha256": digest_bytes(b"fe2o3 performance closure"),
            "tree": digest_bytes(b"fe2o3 performance tree")[:40],
        },
        {
            "base_commit": module.FERRIC_BASE_COMMIT,
            "commit": digest_bytes(b"ferric performance commit")[:40],
            "id": "source.ferric",
            "repository": "ferric",
            "source_closure_artifact_id": "artifact.source.ferric",
            "source_closure_sha256": digest_bytes(b"ferric performance closure"),
            "tree": digest_bytes(b"ferric performance tree")[:40],
        },
    ]


def make_tcb(module: Any) -> list[dict[str, Any]]:
    return [
        {
            "artifact_id": f"artifact.{identifier}",
            "id": identifier,
            "identity_sha256": digest_bytes(identifier.encode("ascii")),
            "kind": module.TCB_KINDS[identifier],
        }
        for identifier in module.TCB_IDS
    ]


def make_environment(module: Any) -> dict[str, Any]:
    environment: dict[str, Any] = {
        "affinity_sha256": digest_bytes(b"affinity"),
        "cache_policy_sha256": digest_bytes(b"cache policy"),
        "clock_policy_sha256": digest_bytes(b"clock policy"),
        "cpu_identity_sha256": digest_bytes(b"cpu identity"),
        "device_count": 1,
        "device_model": "AMD Instinct MI300X",
        "device_uuid": "GPU-01234567-89ab-cdef-0123-456789abcdef",
        "driver_sha256": digest_bytes(b"driver"),
        "firmware_sha256": digest_bytes(b"firmware"),
        "llvm_sha256": digest_bytes(b"llvm"),
        "numa_sha256": digest_bytes(b"numa"),
        "power_policy_sha256": digest_bytes(b"power policy"),
        "rocm_sha256": digest_bytes(b"rocm"),
        "target_arch": "gfx942",
        "target_feature": "xnack-",
        "thermal_policy_sha256": digest_bytes(b"thermal policy"),
        "topology_sha256": digest_bytes(b"topology"),
    }
    environment["environment_sha256"] = module.canonical_digest(environment)
    return environment


def make_baselines(module: Any) -> list[dict[str, Any]]:
    tuning = digest_bytes(b"equal bounded tuning budget")
    return [
        {
            "config_sha256": digest_bytes(f"config:{identifier}".encode("ascii")),
            "id": identifier,
            "identity_sha256": digest_bytes(f"identity:{identifier}".encode("ascii")),
            "kind": module.BASELINE_KINDS[identifier],
            "tuning_budget_sha256": tuning
            if index < 3
            else digest_bytes(f"tuning:{identifier}".encode("ascii")),
        }
        for index, identifier in enumerate(module.BASELINE_IDS)
    ]


def workload(module: Any, index: int) -> dict[str, Any]:
    return {
        "acceptance": ("target-only", "mixed", "high", "low")[index],
        "arrival": module.WORKLOAD_VALUES["arrival"][index],
        "batch": module.WORKLOAD_VALUES["batch"][index],
        "decode_kv_length": module.WORKLOAD_VALUES["decode_kv_length"][index],
        "draft_length": module.WORKLOAD_VALUES["draft_length"][index],
        "isl_osl": module.WORKLOAD_VALUES["isl_osl"][index],
        "prefill_length": module.WORKLOAD_VALUES["prefill_length"][index],
        "prefix_sharing_percent": 0,
    }


def make_rows(
    cell_id: str,
    kind: str,
    engines: tuple[str, ...],
    primary: dict[str, int],
    latency: dict[str, int],
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for phase, count in (("warmup", 10), ("recorded", 30)):
        for ordinal in range(count):
            rotation = ordinal % len(engines)
            serving = kind == "serving-primary" and phase == "recorded"
            rows.append(
                {
                    "clock_khz": 1_700_000,
                    "engine_order": list(engines[rotation:] + engines[:rotation]),
                    "faults": [],
                    "id": f"{cell_id}.{phase}.{ordinal:03d}",
                    "ordinal": ordinal,
                    "phase": phase,
                    "server_start": ordinal // 10 if serving else -1,
                    "status": "passed",
                    "temperature_millicelsius": 65_000,
                    "values": {
                        engine: {
                            "p99_latency_ns": latency[engine],
                            "primary": primary[engine],
                        }
                        for engine in engines
                    },
                    "window": ordinal % 10 if serving else -1,
                }
            )
    return rows


def make_cells(module: Any) -> list[dict[str, Any]]:
    definitions = [
        (
            "core.gemm.001",
            "core-kernel",
            {
                "ferric": 1_000,
                "baseline.vendor": 1_000,
                "baseline.ferric-reference": 1_000,
            },
            {
                "ferric": 1_000,
                "baseline.vendor": 1_000,
                "baseline.ferric-reference": 1_000,
            },
        ),
        (
            "serving.primary.001",
            "serving-primary",
            {
                "ferric": 1_000,
                "baseline.vllm": 900,
                "baseline.sglang": 950,
                "baseline.ferric-reference": 1_000,
            },
            {
                "ferric": 1_000,
                "baseline.vllm": 1_000,
                "baseline.sglang": 1_000,
                "baseline.ferric-reference": 1_000,
            },
        ),
        (
            "speculation.eligible.001",
            "speculation",
            {
                "ferric": 1_120,
                "baseline.ferric-target-only": 1_000,
                "baseline.ferric-reference": 1_100,
            },
            {
                "ferric": 1_030,
                "baseline.ferric-target-only": 1_000,
                "baseline.ferric-reference": 1_000,
            },
        ),
        (
            "speculation.low.001",
            "low-acceptance",
            {
                "ferric": 970,
                "baseline.ferric-target-only": 1_000,
                "baseline.ferric-reference": 1_000,
            },
            {
                "ferric": 1_040,
                "baseline.ferric-target-only": 1_000,
                "baseline.ferric-reference": 1_000,
            },
        ),
    ]
    cells: list[dict[str, Any]] = []
    for index, (cell_id, kind, primary, latency) in enumerate(definitions):
        declaration = workload(module, index)
        engines = module.CELL_ENGINES[kind]
        protocol = {
            "arrival_trace_sha256": digest_bytes(f"arrival:{cell_id}".encode("ascii")),
            "output_limits_sha256": digest_bytes(f"limits:{cell_id}".encode("ascii")),
            "prompt_order_sha256": digest_bytes(f"prompts:{cell_id}".encode("ascii")),
            "sampling_seed_sha256": digest_bytes(f"seed:{cell_id}".encode("ascii")),
        }
        cell = {
            **protocol,
            "core_weight": 1 if kind == "core-kernel" else 0,
            "deterministic_admitted_plan": kind == "low-acceptance",
            "eligible": True,
            "id": cell_id,
            "kind": kind,
            "p99_slo_ns": 1_100,
            "primary_metric": module.PRIMARY_METRICS[kind],
            "public_faster_claim": kind == "serving-primary",
            "rows": make_rows(cell_id, kind, engines, primary, latency),
            "workload": declaration,
            "workload_sha256": "",
        }
        cell["workload_sha256"] = module.canonical_digest(
            {
                **protocol,
                "dimensions": declaration,
                "p99_slo_ns": cell["p99_slo_ns"],
                "primary_metric": cell["primary_metric"],
            }
        )
        cells.append(cell)
    return cells


def refresh_report(
    report_path: Path, context: dict[str, Any], report: dict[str, Any]
) -> None:
    payload = canonical_bytes(report)
    report_path.write_bytes(payload)
    context["artifact"]["sha256"] = digest_bytes(payload)
    context["artifact"]["size_bytes"] = len(payload)


def refresh_measurements(
    measurement_path: Path,
    report_path: Path,
    context: dict[str, Any],
    report: dict[str, Any],
    measurements: dict[str, Any],
) -> None:
    payload = canonical_bytes(measurements)
    measurement_path.write_bytes(payload)
    report["measurement_roster_sha256"] = digest_bytes(payload)
    report["measurement_roster_size_bytes"] = len(payload)
    refresh_report(report_path, context, report)


def make_fixture(
    repo: Path, module: Any, root: Path
) -> tuple[Path, Path, dict[str, Any], dict[str, Any], dict[str, Any]]:
    requirements_path = repo / "proofs/M1_REQUIREMENTS.json"
    requirements = json.loads(requirements_path.read_text(encoding="ascii"))
    obligation = next(
        item for item in requirements["roadmap_requirements"] if item["id"] == "m1.r31"
    )
    path_record = next(
        item for item in requirements["path_obligations"] if item["id"] == "d10-bench"
    )
    sources = make_sources(module, requirements)
    tcb = make_tcb(module)
    artifact_id = "artifact.performance.m1-r31.d10"
    report_relative = f"artifacts/{artifact_id}.performance-report.json"
    measurement_relative = f"measurements/{artifact_id}.measurements.json"
    report_path = root / report_relative
    measurement_path = root / measurement_relative
    report_path.parent.mkdir(parents=True, exist_ok=True)
    measurement_path.parent.mkdir(parents=True, exist_ok=True)
    binding: dict[str, Any] = {
        "artifact_id": artifact_id,
        "evidence_kind": "performance-gate",
        "id": "binding.performance.m1-r31.d10",
        "obligation_class": "Roadmap",
        "obligation_id": "m1.r31",
        "path_id": "d10-bench",
        "profile_id": "qualification",
        "source_identity_id": "source.ferric",
        "statement_sha256": digest_bytes(obligation["title"].encode("utf-8")),
        "tcb_ids": list(module.TCB_IDS),
    }
    binding["binding_sha256"] = module.canonical_digest(binding)
    resolution = {
        "availability": path_record["availability"],
        "id": path_record["id"],
        "path": path_record["path"],
        "repository": path_record["repository"],
        "source_identity_id": "source.ferric",
    }
    context = {
        "artifact": {
            "id": artifact_id,
            "kind": "PerformanceReport",
            "path": report_relative,
            "sha256": digest_bytes(b"pending report"),
            "size_bytes": 1,
        },
        "artifact_absolute_path": str(report_path),
        "binding": binding,
        "format": module.INDEX_FORMAT,
        "path_resolution": resolution,
        "requirements_sha256": digest_file(requirements_path),
        "sources": sources,
        "subject": f"binding:{binding['id']}",
        "tcb": tcb,
    }
    identities = {
        key: digest_bytes(key.encode("ascii")) for key in module.IDENTITY_HASH_KEYS
    }
    identities.update(
        {
            "baseline_protocol_id": "m1.baselines.qualified.001",
            "benchmark_protocol_id": "m1.performance.protocol.001",
            "dispatch_graph_id": "qwen3.m1.dispatch.001",
            "executable_id": "qwen3.m1.executable.001",
            "generated_plan_id": "qwen3.m1.plan.001",
            "schedule_id": "qwen3.m1.schedule.001",
        }
    )
    environment = make_environment(module)
    baselines = make_baselines(module)
    identities["ferric_tuning_budget_sha256"] = baselines[0]["tuning_budget_sha256"]
    matrix = {key: list(values) for key, values in module.WORKLOAD_VALUES.items()}
    cells = make_cells(module)
    identities["workload_roster_sha256"] = module.canonical_digest(
        [cell["workload_sha256"] for cell in cells]
    )
    measurements = {
        "authority": module.AUTHORITY,
        "baseline_roster": copy.deepcopy(baselines),
        "cells": cells,
        "environment_sha256": environment["environment_sha256"],
        "format": module.MEASUREMENT_FORMAT,
        "qualification_identities": copy.deepcopy(identities),
        "target": module.TARGET,
        "workload_matrix": copy.deepcopy(matrix),
    }
    summary = module.summarize_suite(measurements)
    report = {
        "authority": module.AUTHORITY,
        "baseline_roster": copy.deepcopy(baselines),
        "binding_sha256": binding["binding_sha256"],
        "environment": environment,
        "evidence_kind": "performance-gate",
        "format": module.REPORT_FORMAT,
        "measurement_roster_relative_path": measurement_relative,
        "measurement_roster_sha256": digest_bytes(b"pending measurements"),
        "measurement_roster_size_bytes": 1,
        "milestone": "M1",
        "nonclaim": module.NONCLAIM,
        "obligation_class": binding["obligation_class"],
        "obligation_id": binding["obligation_id"],
        "obligation_state": "Open",
        "path_id": binding["path_id"],
        "path_resolution_sha256": module.canonical_digest(resolution),
        "performance_policy_path": module.PERFORMANCE_POLICY_PATH,
        "performance_policy_sha256": digest_file(repo / module.PERFORMANCE_POLICY_PATH),
        "profile_id": binding["profile_id"],
        "qualification_identities": copy.deepcopy(identities),
        "requirements_sha256": context["requirements_sha256"],
        "source_roster_sha256": module.canonical_digest(sources),
        "statement_sha256": binding["statement_sha256"],
        "summary": summary,
        "target": module.TARGET,
        "tcb_identity_sha256s": {item["id"]: item["identity_sha256"] for item in tcb},
        "tcb_roster_sha256": module.canonical_digest(tcb),
        "threshold_semantics": module.THRESHOLD_SEMANTICS,
        "thresholds": copy.deepcopy(module.THRESHOLDS),
        "workload_matrix": copy.deepcopy(matrix),
    }
    refresh_measurements(measurement_path, report_path, context, report, measurements)
    return report_path, measurement_path, context, report, measurements


def invoke(
    validator: Path,
    context: dict[str, Any],
    *,
    protocol: str = PROTOCOL,
    raw_context: bytes | None = None,
) -> subprocess.CompletedProcess[bytes]:
    if raw_context is None:
        raw_context = (
            json.dumps(
                context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
            )
            + "\n"
        ).encode("ascii")
    return subprocess.run(
        [sys.executable, "-I", str(validator), protocol],
        check=False,
        input=raw_context,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=15,
    )


def canonical_case(repo: Path, module: Any, validator: Path, root: Path) -> None:
    _, _, context, _, _ = make_fixture(repo, module, root)
    result = invoke(validator, context)
    payload = json.dumps(
        context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("ascii")
    expected = (
        f"PASS: {PROTOCOL} artifact_sha256={context['artifact']['sha256']} "
        f"context_sha256={digest_bytes(payload)}\n"
    ).encode("ascii")
    if result.returncode != 0 or result.stdout != expected:
        fail(
            f"canonical performance report rejected: exit={result.returncode}, output={result.stdout!r}"
        )


def hostile_cases(repo: Path, module: Any, validator: Path, root: Path) -> int:
    cases: list[tuple[str, Mutation]] = []

    def report_field(key: str, value: Any) -> Mutation:
        def mutate(
            report_path: Path,
            _measurement_path: Path,
            context: dict[str, Any],
            report: dict[str, Any],
            _measurements: dict[str, Any],
        ) -> None:
            report[key] = copy.deepcopy(value)
            refresh_report(report_path, context, report)

        return mutate

    cases.extend(
        [
            ("format", report_field("format", "FERRIC-M1-PERFORMANCE-REPORT-V2")),
            ("authority", report_field("authority", "qualification-authority")),
            ("nonclaim", report_field("nonclaim", "Performance proves correctness.")),
            ("kind", report_field("evidence_kind", "hardware-test")),
            ("milestone", report_field("milestone", "M2")),
            ("target", report_field("target", "gfx950:xnack-")),
            ("status", report_field("obligation_state", "Closed")),
            (
                "binding-replay",
                report_field("binding_sha256", digest_bytes(b"binding")),
            ),
            ("path-replay", report_field("path_id", "serving-bench")),
            ("profile-replay", report_field("profile_id", "kernel")),
            ("policy-path", report_field("performance_policy_path", "docs/ROADMAP.md")),
            (
                "policy-bytes",
                report_field("performance_policy_sha256", digest_bytes(b"policy")),
            ),
            (
                "source-replay",
                report_field("source_roster_sha256", digest_bytes(b"source")),
            ),
            ("tcb-replay", report_field("tcb_roster_sha256", digest_bytes(b"tcb"))),
            (
                "threshold",
                lambda p, _m, c, r, _x: (
                    r["thresholds"].__setitem__("serving_lcb_min_ratio_ppm", 900_000),
                    refresh_report(p, c, r),
                ),
            ),
            (
                "threshold-semantics",
                report_field("threshold_semantics", "floating point means"),
            ),
            (
                "summary-arithmetic",
                lambda p, _m, c, r, _x: (
                    r["summary"]["cell_summaries"][0].__setitem__(
                        "primary_ratio_ppm", 2_000_000
                    ),
                    refresh_report(p, c, r),
                ),
            ),
            (
                "raw-path",
                report_field(
                    "measurement_roster_relative_path", "measurements/replayed.json"
                ),
            ),
        ]
    )

    def measurement_edit(edit: Callable[[dict[str, Any]], None]) -> Mutation:
        def mutate(
            report_path: Path,
            measurement_path: Path,
            context: dict[str, Any],
            report: dict[str, Any],
            measurements: dict[str, Any],
        ) -> None:
            edit(measurements)
            refresh_measurements(
                measurement_path, report_path, context, report, measurements
            )

        return mutate

    cases.extend(
        [
            (
                "raw-format",
                measurement_edit(
                    lambda x: x.__setitem__(
                        "format", "FERRIC-M1-PERFORMANCE-MEASUREMENTS-V2"
                    )
                ),
            ),
            (
                "raw-authority",
                measurement_edit(
                    lambda x: x.__setitem__("authority", "self-certified")
                ),
            ),
            (
                "raw-target",
                measurement_edit(lambda x: x.__setitem__("target", "gfx942:xnack+")),
            ),
            (
                "raw-environment",
                measurement_edit(
                    lambda x: x.__setitem__(
                        "environment_sha256", digest_bytes(b"environment")
                    )
                ),
            ),
            (
                "raw-plan",
                measurement_edit(
                    lambda x: x["qualification_identities"].__setitem__(
                        "generated_plan_sha256", digest_bytes(b"plan")
                    )
                ),
            ),
            (
                "workload-matrix",
                measurement_edit(lambda x: x["workload_matrix"]["batch"].pop()),
            ),
            ("required-cell-omission", measurement_edit(lambda x: x["cells"].pop())),
            (
                "duplicate-cell",
                measurement_edit(
                    lambda x: x["cells"].__setitem__(1, copy.deepcopy(x["cells"][0]))
                ),
            ),
            (
                "workload-substitution",
                measurement_edit(
                    lambda x: x["cells"][0]["workload"].__setitem__("batch", 2)
                ),
            ),
            (
                "workload-hash",
                measurement_edit(
                    lambda x: x["cells"][0].__setitem__(
                        "workload_sha256", digest_bytes(b"workload")
                    )
                ),
            ),
            (
                "prompt-order-substitution",
                measurement_edit(
                    lambda x: x["cells"][0].__setitem__(
                        "prompt_order_sha256", digest_bytes(b"other prompts")
                    )
                ),
            ),
            (
                "metric-substitution",
                measurement_edit(
                    lambda x: x["cells"][1].__setitem__(
                        "primary_metric", "requests-per-second"
                    )
                ),
            ),
            (
                "p99-slo-substitution",
                measurement_edit(
                    lambda x: x["cells"][1].__setitem__("p99_slo_ns", 999)
                ),
            ),
            (
                "ineligible",
                measurement_edit(
                    lambda x: x["cells"][1].__setitem__("eligible", False)
                ),
            ),
            (
                "low-plan",
                measurement_edit(
                    lambda x: x["cells"][3].__setitem__(
                        "deterministic_admitted_plan", False
                    )
                ),
            ),
            ("dropped-sample", measurement_edit(lambda x: x["cells"][0]["rows"].pop())),
            (
                "failed-sample",
                measurement_edit(
                    lambda x: x["cells"][0]["rows"][10].__setitem__("status", "failed")
                ),
            ),
            (
                "faulted-sample",
                measurement_edit(
                    lambda x: x["cells"][0]["rows"][10]["faults"].append("ecc")
                ),
            ),
            (
                "sample-order",
                measurement_edit(
                    lambda x: x["cells"][0]["rows"].__setitem__(
                        10, x["cells"][0]["rows"].pop(11)
                    )
                ),
            ),
            (
                "boolean-ordinal",
                measurement_edit(
                    lambda x: x["cells"][0]["rows"][1].__setitem__("ordinal", True)
                ),
            ),
            (
                "engine-order",
                measurement_edit(
                    lambda x: x["cells"][0]["rows"][10]["engine_order"].reverse()
                ),
            ),
            (
                "missing-engine",
                measurement_edit(
                    lambda x: x["cells"][0]["rows"][10]["values"].pop("baseline.vendor")
                ),
            ),
            (
                "zero-metric",
                measurement_edit(
                    lambda x: x["cells"][0]["rows"][10]["values"]["ferric"].__setitem__(
                        "primary", 0
                    )
                ),
            ),
            (
                "variance",
                measurement_edit(
                    lambda x: x["cells"][0]["rows"][10]["values"]["ferric"].__setitem__(
                        "primary", 2_000
                    )
                ),
            ),
            (
                "thermal",
                measurement_edit(
                    lambda x: x["cells"][1]["rows"][10].__setitem__(
                        "temperature_millicelsius", 80_000
                    )
                ),
            ),
            (
                "clock",
                measurement_edit(
                    lambda x: x["cells"][1]["rows"][10].__setitem__(
                        "clock_khz", 1_000_000
                    )
                ),
            ),
            (
                "serving-window",
                measurement_edit(
                    lambda x: x["cells"][1]["rows"][10].__setitem__("window", 9)
                ),
            ),
            (
                "core-floor",
                measurement_edit(
                    lambda x: [
                        row["values"]["ferric"].__setitem__("primary", 790)
                        for row in x["cells"][0]["rows"]
                    ]
                ),
            ),
            (
                "core-geomean",
                measurement_edit(
                    lambda x: [
                        row["values"]["ferric"].__setitem__("primary", 900)
                        for row in x["cells"][0]["rows"]
                    ]
                ),
            ),
            (
                "serving-lcb",
                measurement_edit(
                    lambda x: [
                        row["values"]["ferric"].__setitem__("primary", 900)
                        for row in x["cells"][1]["rows"]
                    ]
                ),
            ),
            (
                "speculation-throughput",
                measurement_edit(
                    lambda x: [
                        row["values"]["ferric"].__setitem__("primary", 1_090)
                        for row in x["cells"][2]["rows"]
                    ]
                ),
            ),
            (
                "speculation-latency",
                measurement_edit(
                    lambda x: [
                        row["values"]["ferric"].__setitem__("p99_latency_ns", 1_060)
                        for row in x["cells"][2]["rows"]
                    ]
                ),
            ),
            (
                "low-acceptance-regression",
                measurement_edit(
                    lambda x: [
                        row["values"]["ferric"].__setitem__("primary", 940)
                        for row in x["cells"][3]["rows"]
                    ]
                ),
            ),
            (
                "public-faster-lcb",
                measurement_edit(
                    lambda x: [
                        row["values"]["ferric"].__setitem__("primary", 995)
                        for row in x["cells"][1]["rows"]
                    ]
                ),
            ),
            (
                "baseline-budget",
                lambda p, m, c, r, x: (
                    r["baseline_roster"][0].__setitem__(
                        "tuning_budget_sha256", digest_bytes(b"more tuning")
                    ),
                    x["baseline_roster"][0].__setitem__(
                        "tuning_budget_sha256", digest_bytes(b"more tuning")
                    ),
                    refresh_measurements(m, p, c, r, x),
                ),
            ),
            (
                "ferric-tuning-budget",
                lambda p, m, c, r, x: (
                    r["qualification_identities"].__setitem__(
                        "ferric_tuning_budget_sha256", digest_bytes(b"extra tuning")
                    ),
                    x["qualification_identities"].__setitem__(
                        "ferric_tuning_budget_sha256", digest_bytes(b"extra tuning")
                    ),
                    refresh_measurements(m, p, c, r, x),
                ),
            ),
        ]
    )

    for index, (name, mutation) in enumerate(cases):
        report_path, measurement_path, context, report, measurements = make_fixture(
            repo, module, root / f"case-{index:03d}-{name}"
        )
        mutation(report_path, measurement_path, context, report, measurements)
        result = invoke(validator, context)
        if result.returncode == 0:
            fail(f"hostile performance fixture accepted: {name}")

    extra = 0
    report_path, measurement_path, context, report, measurements = make_fixture(
        repo, module, root / "nan"
    )
    raw = canonical_bytes(measurements).replace(
        b'"primary": 1000', b'"primary": NaN', 1
    )
    measurement_path.write_bytes(raw)
    report["measurement_roster_sha256"] = digest_bytes(raw)
    report["measurement_roster_size_bytes"] = len(raw)
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("NaN measurement was accepted")
    extra += 1

    report_path, measurement_path, context, report, measurements = make_fixture(
        repo, module, root / "infinity"
    )
    raw = canonical_bytes(measurements).replace(
        b'"primary": 1000', b'"primary": Infinity', 1
    )
    measurement_path.write_bytes(raw)
    report["measurement_roster_sha256"] = digest_bytes(raw)
    report["measurement_roster_size_bytes"] = len(raw)
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("infinite measurement was accepted")
    extra += 1

    report_path, measurement_path, context, report, _ = make_fixture(
        repo, module, root / "raw-noncanonical"
    )
    value = json.loads(measurement_path.read_text(encoding="ascii"))
    raw = (json.dumps(value, ensure_ascii=True, sort_keys=True) + "\n").encode("ascii")
    measurement_path.write_bytes(raw)
    report["measurement_roster_sha256"] = digest_bytes(raw)
    report["measurement_roster_size_bytes"] = len(raw)
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("noncanonical measurement roster was accepted")
    extra += 1

    report_path, _, context, report, _ = make_fixture(
        repo, module, root / "report-duplicate"
    )
    raw = canonical_bytes(report).replace(
        b'{\n  "authority":', b'{\n  "format": "duplicate",\n  "authority":', 1
    )
    report_path.write_bytes(raw)
    context["artifact"]["sha256"] = digest_bytes(raw)
    context["artifact"]["size_bytes"] = len(raw)
    if invoke(validator, context).returncode == 0:
        fail("duplicate-key report was accepted")
    extra += 1

    report_path, _, context, report, _ = make_fixture(
        repo, module, root / "report-extra"
    )
    report["qualified"] = True
    refresh_report(report_path, context, report)
    if invoke(validator, context).returncode == 0:
        fail("extra report authority field was accepted")
    extra += 1

    report_path, _, context, _, _ = make_fixture(repo, module, root / "report-symlink")
    target = report_path.with_name("target.json")
    report_path.rename(target)
    report_path.symlink_to(target.name)
    if invoke(validator, context).returncode == 0:
        fail("symlink performance report was accepted")
    extra += 1

    _, measurement_path, context, _, _ = make_fixture(
        repo, module, root / "measurement-symlink"
    )
    target = measurement_path.with_name("target.json")
    measurement_path.rename(target)
    measurement_path.symlink_to(target.name)
    if invoke(validator, context).returncode == 0:
        fail("symlink measurement roster was accepted")
    extra += 1

    _, _, context, _, _ = make_fixture(repo, module, root / "context")
    compact = json.dumps(
        context, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    if (
        invoke(
            validator,
            context,
            raw_context=(
                json.dumps(context, ensure_ascii=True, sort_keys=True) + "\n"
            ).encode("ascii"),
        ).returncode
        == 0
    ):
        fail("noncanonical performance context was accepted")
    if (
        invoke(
            validator,
            context,
            raw_context=(compact + "\n")
            .encode("ascii")
            .replace(b'{"artifact":', b'{"format":"duplicate","artifact":', 1),
        ).returncode
        == 0
    ):
        fail("duplicate-key performance context was accepted")
    if invoke(validator, context, protocol=PROTOCOL + ".drift").returncode == 0:
        fail("wrong performance protocol was accepted")
    replay = copy.deepcopy(context)
    replay["binding"]["obligation_id"] = "m1.r33"
    if invoke(validator, replay).returncode == 0:
        fail("cross-obligation replay was accepted")
    extra += 4

    report_path, _, _, _, _ = make_fixture(repo, module, root / "toctou")
    original = module.file_identity
    calls = 0

    def changed(metadata: Any) -> tuple[int, int, int, int, int, int]:
        nonlocal calls
        calls += 1
        identity = original(metadata)
        return identity if calls == 1 else (*identity[:-1], identity[-1] + 1)

    module.file_identity = changed
    try:
        with contextlib.redirect_stderr(io.StringIO()):
            try:
                module.read_bounded(
                    report_path, module.MAX_REPORT_BYTES, "racing report"
                )
            except SystemExit:
                pass
            else:
                fail("simulated report replacement during read was accepted")
    finally:
        module.file_identity = original
    extra += 1
    return len(cases) + extra


def audit_checker_pin(repo: Path, validator: Path) -> None:
    checker = load_module(
        repo / "proofs/check-m1-evidence-index.py", "ferric_m1_evidence_checker"
    )
    expected = (
        "proofs/m1/evidence/validate-performance-report.py",
        PROTOCOL,
        digest_file(validator),
    )
    if checker.TRUSTED_VALIDATORS.get("performance-gate") != expected:
        fail("checker-owned performance path, protocol, or source pin drifted")


def audit_open_requirements(repo: Path) -> None:
    requirements = json.loads(
        (repo / "proofs/M1_REQUIREMENTS.json").read_text(encoding="ascii")
    )
    if any(
        record["obligation_state"] != "Open"
        for key in ("roadmap_requirements", "assurance_properties", "path_obligations")
        for record in requirements[key]
    ):
        fail("M1 status was changed by performance validation")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: test-performance-report-policy.py FERRIC_REPO")
    repo = Path(sys.argv[1]).resolve(strict=True)
    validator = repo / "proofs/m1/evidence/validate-performance-report.py"
    module = load_module(validator, "ferric_m1_performance_validator")
    audit_checker_pin(repo, validator)
    audit_open_requirements(repo)
    with tempfile.TemporaryDirectory(prefix="ferric-m1-performance.") as raw:
        root = Path(raw)
        canonical_case(repo, module, validator, root / "canonical")
        hostile_count = hostile_cases(repo, module, validator, root / "hostile")
    print(
        "PASS: M1 performance validator accepted 1 canonical report and rejected "
        f"{hostile_count} hostile fixtures"
    )


if __name__ == "__main__":
    main()
