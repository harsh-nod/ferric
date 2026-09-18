#!/usr/bin/env python3
"""Plot a fully revalidated full-forward pair, with separate arithmetic cohorts.

This tool never runs a GPU. Both raw captures must pass the unchanged pinned
checker before any public report or chart output directory is created.
"""

import argparse
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import stat
import types

import target_decode_plots_v1 as drawing


COHORTS = {
    "scalar": {
        "checker": "target_full_forward_scalar_v2.py",
        "sha256": "045c76bffc17d1ea3f958d599d5e56fa1bafafe90146bcc108c404078a55d639",
        "schema": "FerricTargetFullForwardScalarObservationV2",
        "variants": ("serial-control", "full-forward-scalar-v3"),
        "projection": "baseline", "kernel_profile": "v3-wave", "logits": "BF16",
        "title": "Qwen3-8B: scalar/BF16 full-forward submissions",
        "controller_cohort": "rank-wrapper-fixed-v6", "attention": "baseline",
    },
    "mfma-v7": {
        "checker": "target_full_forward_mfma_v7_v2.py",
        "sha256": "2da842ca2403e8ffb3bb9f8e9f73e586a4a0317bdb239fefb94adb5d9ee8b93b",
        "schema": "FerricTargetFullForwardMfmaV7ObservationV2",
        "variants": ("serial-mfma-v7-control", "full-forward-mfma-v7"),
        "projection": "mfma", "kernel_profile": "v3-mfma", "logits": "FP32",
        "title": "Qwen3-8B: MFMA/FP32-v7 full-forward submissions",
        "controller_cohort": "rank-wrapper-fixed-v6", "attention": "baseline",
    },
    "mfma-v7-wave": {
        "checker": "target_full_forward_mfma_wave_v1.py",
        "sha256": "921d34280af358146882201c0faed96ead24a7162e2b8f2fdd1e962ced0d77bb",
        "schema": "FerricTargetFullForwardMfmaWaveObservationV1",
        "variants": ("serial-mfma-v7-wave-control", "full-forward-mfma-v7-wave"),
        "projection": "mfma", "kernel_profile": "v3-mfma", "logits": "FP32",
        "title": "Qwen3-8B: MFMA/v7/wave-attention full-forward submissions",
        "controller_cohort": "mfma-v7-wave-attention-controller-v7", "attention": "wave",
    },
}
LABELS = ("Serial submissions", "Full-forward submission")


def load_checker(family):
    cohort = COHORTS[family]
    path = Path(__file__).with_name(cohort["checker"])
    descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 131072:
            raise ValueError("checker source extent")
        raw = os.read(descriptor, 131073)
        after = os.fstat(descriptor)
        if len(raw) != before.st_size or any(getattr(before, key) != getattr(after, key) for key in
                                             ("st_dev", "st_ino", "st_mode", "st_size", "st_mtime_ns", "st_ctime_ns")):
            raise ValueError("checker source changed")
    finally:
        os.close(descriptor)
    if hashlib.sha256(raw).hexdigest() != cohort["sha256"]:
        raise ValueError("frozen full-forward checker digest")
    module = types.ModuleType("_frozen_full_forward_plot_" + family.replace("-", "_"))
    module.__file__ = str(path)
    exec(compile(raw, str(path), "exec"), module.__dict__)
    return module


def revalidate(checker, family, directory, variant, reference):
    read = checker.core.read_bounded
    report = checker.compare(read(directory / "capture.ndjson", 1048576),
                             read(directory / "exit-status.txt", 16),
                             read(directory / "workload.json", 65536), reference,
                             read(directory / "predeclared-expectation.json", 65536),
                             read(directory / "stderr.log", 65536))
    report["comparator_sha256"] = COHORTS[family]["sha256"]
    checker.core.same(report["variant"], variant, "exact plotted variant")
    raw = read(directory / "report-public.json", 1048576)
    checker.core.same(checker.core.json_value(raw), report, "regenerated public report")
    checker.core.require(read(directory / "postcheck-status.txt", 256) ==
                         b"controller=0 idle=0 topology=0 inputs=0 plan=0 source=0\n",
                         "successful harness postchecks")
    return report, raw


def rows_and_contrast(checker, family, reports):
    cohort = COHORTS[family]
    core = checker.core
    core.require(type(reports) is list and len(reports) == 2, "exact complete plot pair")
    rows = []
    for index, (report, variant, label) in enumerate(zip(reports, cohort["variants"], LABELS)):
        full_forward = index == 1
        for key, expected in {
            "schema": cohort["schema"], "variant": variant,
            "controller_cohort": cohort["controller_cohort"], "comparator_sha256": cohort["sha256"],
            "authority": "none", "model": "Qwen/Qwen3-8B", "revision": core.REVISION,
            "reference_sha256": core.REFERENCE_SHA256, "passed": True,
            "reference_tokens_and_bytes_match": True, "tensor_parallel": 1, "concurrent_requests": 1,
            "warmup_requests": 0, "measured_requests": 1, "profile": "single-unwarmed-request",
            "processed_kv_tokens": 36, "dispatches": 22176, "generated_tokens": core.TOKENS,
            "generated_utf8_bytes": list("".join(core.PIECES).encode()),
            "weight_precision": "BF16", "activation_precision": "BF16", "logits_precision": cohort["logits"],
            "benchmark_qualified": False, "performance_qualified": False, "runtime_profiling": False,
            "gpu_timestamps": False, "overlap_measured": False,
            "completion_polls_measured": False, "persistent_kernel": False,
            "common_comparator_sha256": checker.CORE_SHA256,
            "identities": {"controller_sha256": checker.CONTROLLER_SHA256,
                           "worker_sha256": checker.WORKER_SHA256, **checker.ARTIFACT},
            "execution_counts": checker.execution_counts({"runtime_full_forward": full_forward}),
        }.items():
            core.same(report[key], expected, "exact full-forward plot " + key)
        config = {
            "batch_tokens": 1, "prefill_chunk": 1, "context_tokens": 64, "physical_pages": 4,
            "prefix_cache": False, "output_head_pruning": False, "runtime_ordered_batches": False,
            "kernel_profile": cohort["kernel_profile"], "collective": "device-tp1-v3", "host_timing_enabled": False,
            "target_only": True, "speculation": False,
            "head_precision": "BF16" if family == "scalar" else "fp32-v7",
            "runtime_full_forward": full_forward,
            "full_forward_profile": checker.PROFILE if full_forward else None,
            "performance_profile": {"runtime_cache_admission": True, "runtime_operational": True,
                                    "runtime_profiling": False, "dispatch_sequences": False,
                                    "queue_rollover": False, "projection": cohort["projection"], "attention": cohort["attention"]},
        }
        core.same(report["configuration"], config, "exact full-forward plot configuration")
        if cohort["logits"] == "FP32":
            core.same(report["head_artifact"], checker.HEAD_ARTIFACT, "unchanged FP32-v7 head image")
            core.same(report["head_precision"], "fp32-v7", "explicit FP32-v7 head")
            core.same(report["fp32_head_workspace_bytes"], 9_723_904, "unchanged FP32 workspace")
        else:
            core.require("head_artifact" not in report, "scalar has no extra image")
        timing = report["timing"]
        intervals = timing["decode_intervals_ns"]
        core.same(timing["decode_interval_count"], 31, "31 actual intervals")
        core.require(type(intervals) is list and len(intervals) == 31, "actual interval roster")
        for value in [*intervals, timing["admission_ttft_ns"], timing["generation_ns"]]:
            core.integer(value, 1, 2**64 - 1, "exact u64 interval")
        rate = 31 * 1e9 / sum(intervals)
        core.same(rate, timing["post_first_tokens_per_second"], "recomputed rate")
        core.same(sum(intervals) / 31, timing["mean_tpot_ns"], "recomputed TPOT")
        core.same(timing["generation_ns"], timing["admission_ttft_ns"] + sum(intervals), "request timing boundary")
        core.seconds(timing["setup_seconds"], "setup observation")
        rows.append({"family": "full-forward-v2-" + family, "variant": variant, "label": label,
                     "identities": report["identities"], "model_revision": report["revision"],
                     "reference_sha256": report["reference_sha256"],
                     "decode_intervals_seconds": [value / 1e9 for value in intervals],
                     "post_first_tokens_per_second": rate})
    drawing.matched(rows)
    ratio = rows[1]["post_first_tokens_per_second"] / rows[0]["post_first_tokens_per_second"]
    contrast = {
        "schema": "FerricFullForwardObservedContrastV2", "family": family,
        "controller_cohort": cohort["controller_cohort"], "observed_rate_ratio": ratio,
        "observed_tpot_change_percent": (1 / ratio - 1) * 100,
        "measured_requests_per_variant": 1, "warmup_requests": 0,
        "weight_precision": "BF16", "activation_precision": "BF16", "logits_precision": cohort["logits"],
        "packets_per_request_each": 22176, "source_completion_frontiers": {"serial": 22176, "full_forward": 36},
        "frontier_count_scope": "source-derived schedule; not measured polls or GPU event count",
        "cpu_isolation_verified": False, "performance_qualified": False,
        "causal_or_stable_speedup_claim": False, "gpu_overlap_measured": False, "persistent_gpu_kernel": False,
    }
    core.require(all(math.isfinite(value) for value in (
        ratio, contrast["observed_tpot_change_percent"],
        max(max(row["decode_intervals_seconds"]) for row in rows) * 1120,
        max(row["post_first_tokens_per_second"] for row in rows) * 720)), "finite chart extents")
    return rows, contrast


def generate(output, checker, family, reports, raw_reports):
    rows, contrast = rows_and_contrast(checker, family, reports)
    checker.core.require(type(raw_reports) is list and len(raw_reports) == 2, "exact source reports")
    for report, raw in zip(reports, raw_reports):
        checker.core.same(checker.core.json_value(raw), report, "only exact public reports may be written")
    encoded = json.dumps(contrast, indent=2, sort_keys=True, allow_nan=False) + "\n"
    output.mkdir(parents=True, exist_ok=False)
    title = COHORTS[family]["title"]
    drawing.interval_plot(output / "intervals.svg", rows, title)
    drawing.plots.rates(output / "rates.svg", [(row["label"], row["post_first_tokens_per_second"]) for row in rows], title)
    for name, raw in zip(("control-report.json", "full-forward-report.json"), raw_reports):
        with (output / name).open("xb") as stream:
            stream.write(raw)
    with (output / "intervals.csv").open("x", newline="", encoding="utf-8") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerow(["variant", "output_token_ordinal", "decode_interval_ms"])
        for row in rows:
            writer.writerows((row["variant"], index, f"{value * 1000:.9f}")
                             for index, value in enumerate(row["decode_intervals_seconds"], 2))
    lines = ["| Submission | TTFT (s) | Mean TPOT (s) | Post-first tokens/s | Observed rate / control | Setup (s) |",
             "| --- | ---: | ---: | ---: | ---: | ---: |"]
    for report, row in zip(reports, rows):
        timing = report["timing"]
        lines.append(f"| {row['label']} | {timing['admission_ttft_ns'] / 1e9:.6f} | {timing['mean_tpot_ns'] / 1e9:.6f} | "
                     f"{row['post_first_tokens_per_second']:.6f} | {row['post_first_tokens_per_second'] / rows[0]['post_first_tokens_per_second']:.3f}x | "
                     f"{timing['setup_seconds']:.6f} |")
    (output / "table.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (output / "observed-contrast.json").write_text(encoded, encoding="utf-8")
    sums = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}" for path in sorted(output.iterdir())]
    (output / "SHA256SUMS").write_text("\n".join(sums) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--family", choices=tuple(COHORTS), required=True)
    for name in ("control-capture", "full-capture", "reference", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    checker = load_checker(args.family)
    reference = checker.core.read_bounded(args.reference, 131072)
    pairs = [revalidate(checker, args.family, directory, variant, reference) for directory, variant in
             zip((args.control_capture, args.full_capture), COHORTS[args.family]["variants"])]
    generate(args.output, checker, args.family, [pair[0] for pair in pairs], [pair[1] for pair in pairs])
    print("Both captures independently revalidated; 62 actual intervals plotted; no GPU work.")


if __name__ == "__main__":
    main()
