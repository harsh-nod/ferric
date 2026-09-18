#!/usr/bin/env python3
"""Reproduce the fixed, numerically validated BF16 argmax observation pair."""

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics

import target_batch_decode_v1 as core
import target_decode_plots_v1 as charts

VARIANTS = ("control", "cooperative")
LABELS = {"control": "Scalar argmax control", "cooperative": "Cooperative Wave64 argmax"}
REPORT_SHA = {
    "control": "82804d31d3bbbe2ad43dc1e4555acf45ecce48047693d82a90a2c53978985648",
    "cooperative": "758393bbce91222ea3ee94b1055cee51ed52a90db401d23a8979c26d2d155c88",
}
IMAGES = {
    "control": {
        "artifact_hsaco_id": "583a889a4ac5f03c7b004cdd52c1d062649853326e9f4b3d251e11de706573b5",
        "artifact_manifest_id": "636e1ca50839455767027616086b632168404de928560880fdb316feb6c54d35",
        "artifact_handoff_id": "3d6714946adb7e459717d3e03868a4a66ff86e47113c762f7f2d1e9edda81e33",
    },
    "cooperative": {
        "artifact_hsaco_id": "7641c055026f4f21ad45625b77f8ba66183ab82de9aad8041eb80deed5bc66aa",
        "artifact_manifest_id": "143da25bf7bd83cadceca8961ce58b4e1535fe965b1cc28bbfd5c2904c098af8",
        "artifact_handoff_id": "6ee3360f150300d984c8ff5c1640f5db58ea047c8a1edd055896ade8d2387ffa",
    },
}
CONFIGURATION = {
    "batch_tokens": 1, "prefill_chunk": 1, "context_tokens": 64, "physical_pages": 4,
    "collective": "device-tp1-v3", "kernel_profile": "v3-wave", "prefix_cache": False,
    "output_head_pruning": False, "host_timing_enabled": False, "runtime_ordered_batches": False,
    "performance_profile": {
        "attention": "baseline", "projection": "baseline", "dispatch_sequences": False,
        "queue_rollover": False, "runtime_cache_admission": True,
        "runtime_operational": True, "runtime_profiling": False,
    },
}
TOP_FIELDS = {
    "authority", "benchmark_qualified", "cleanup", "comparator_sha256", "concurrent_requests",
    "configuration", "dispatches", "generated_tokens", "generated_utf8_bytes", "identities",
    "input_sha256", "measured_requests", "model", "nonclaims", "passed", "precision",
    "processed_kv_tokens", "profile", "reference_sha256", "reference_tokens_and_bytes_match",
    "revision", "schema", "tensor_parallel", "timing", "warmup_requests",
}


def validate_report(data, variant):
    core.require(variant in VARIANTS, "argmax variant")
    core.fields(data, TOP_FIELDS, "public argmax report")
    for key, value in {
        "schema": "FerricTargetBatchDecodeObservationV1", "authority": "none",
        "benchmark_qualified": False, "passed": True, "reference_tokens_and_bytes_match": True,
        "model": "Qwen/Qwen3-8B", "revision": core.REVISION, "precision": "BF16",
        "tensor_parallel": 1, "concurrent_requests": 1, "warmup_requests": 0, "measured_requests": 1,
        "profile": "single-unwarmed-request", "processed_kv_tokens": 36, "dispatches": 22176,
        "reference_sha256": core.REFERENCE_SHA256, "generated_tokens": core.TOKENS,
        "generated_utf8_bytes": list("".join(core.PIECES).encode()), "configuration": CONFIGURATION,
        "comparator_sha256": "90cd589fe9b98660f6efb3400775cd269af236688706f37eaedee2d12e92b975",
        "identities": {
            "controller_sha256": "a28848eadc9aa7a34f21e7909b881c1aa8cdbf7d9bc8f788645504ced77357c2",
            "worker_sha256": "b4cb30788d4a32d9cae240823c26a80e9103f5698f91d95b16d2bda7e78270f9",
            **IMAGES[variant],
        },
        "cleanup": {
            "all_emitted_page_counts_checked": True, "completed_request_retirement_record": True,
            "final_free_page_count_observed": False, "worker_close_record": True,
        },
    }.items():
        core.same(data[key], value, "argmax " + key)
    core.fields(data["input_sha256"], {"capture", "expectation", "reference", "status", "workload"}, "input hashes")
    for value in data["input_sha256"].values():
        core.digest(value, "input hash")
    core.same(data["input_sha256"]["reference"], core.REFERENCE_SHA256, "input reference")
    core.require(type(data["nonclaims"]) is list and all(type(value) is str for value in data["nonclaims"]),
                 "public nonclaims")
    timing = data["timing"]
    core.fields(timing, {
        "admission_ttft_ns", "batch_host_durations_ns", "clock", "decode_interval_count",
        "decode_intervals_ns", "generation_ns", "mean_tpot_ns", "median_tpot_ns",
        "post_first_tokens_per_second", "setup_seconds", "whole_controller_seconds",
    }, "timing")
    core.same(timing["clock"], "host std::time::Instant; output completion receipts", "host clock")
    core.same(timing["decode_interval_count"], 31, "31 post-first intervals")
    intervals = timing["decode_intervals_ns"]
    batches = timing["batch_host_durations_ns"]
    core.require(type(intervals) is list and len(intervals) == 31, "interval roster")
    core.require(type(batches) is list and len(batches) == 36, "batch roster")
    for value in [*intervals, *batches, timing["admission_ttft_ns"], timing["generation_ns"]]:
        core.integer(value, 1, 2**64 - 1, "positive exact nanoseconds")
    total = sum(intervals)
    core.same(timing["generation_ns"], timing["admission_ttft_ns"] + total, "generation accounting")
    core.same(timing["median_tpot_ns"], statistics.median(intervals), "observed median")
    for key, expected in {
        "mean_tpot_ns": total / 31, "post_first_tokens_per_second": 31e9 / total,
    }.items():
        actual = core.seconds(timing[key], key)
        core.require(math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-12), "recomputed " + key)
    setup = core.seconds(timing["setup_seconds"], "setup")
    whole = core.seconds(timing["whole_controller_seconds"], "whole controller")
    core.require(whole >= setup + timing["generation_ns"] / 1e9, "whole duration accounting")
    return {
        "variant": variant, "label": LABELS[variant], "intervals_ns": intervals,
        "ttft_seconds": timing["admission_ttft_ns"] / 1e9,
        "mean_tpot_seconds": total / 31e9, "post_first_tokens_per_second": 31e9 / total,
        "setup_seconds": setup,
    }


def read_report(path, variant):
    raw = core.read_bounded(path, 1024 * 1024)
    core.same(hashlib.sha256(raw).hexdigest(), REPORT_SHA[variant], "fixed public report bytes")
    return raw, validate_report(core.json_value(raw), variant)


def plot_rates(path, rows):
    svg = charts.plots.canvas("BF16 argmax: observed Qwen3-8B post-first decode rates", 194)
    charts.plots.label(svg, 24, 52, "One unwarmed request per image; no stable speedup or isolated GPU-time claim.")
    maximum = max(row["post_first_tokens_per_second"] for row in rows) * 1.12
    for index, row in enumerate(rows):
        y = 76 + 40 * index
        value = row["post_first_tokens_per_second"]
        charts.plots.label(svg, 24, y + 18, row["label"])
        width = 660 * value / maximum
        charts.plots.node(svg, "rect", x=290, y=y, width=width, height=25, fill=charts.PALETTE[index])
        charts.plots.label(svg, 298 + width, y + 18, f"{value:.6f}")
    charts.plots.label(svg, 24, 178, "31 intervals per request; tokens/s excludes first-token latency and setup.")
    charts.plots.save(path, svg)


def plot_intervals(path, rows):
    svg = charts.plots.canvas("BF16 argmax: all 62 observed post-first host intervals", 475)
    charts.plots.label(svg, 24, 52, "One unwarmed, nonisolated request per image; these are not kernel durations or GPU overlap.")
    left, right, top, bottom = 88, 1060, 110, 405
    maximum = max(max(row["intervals_ns"]) for row in rows) / 1e6 * 1.12
    for tick in range(6):
        y = bottom - (bottom - top) * tick / 5
        charts.plots.node(svg, "line", x1=left, x2=right, y1=y, y2=y, stroke="#dddddd")
        charts.plots.label(svg, 18, y + 4, f"{maximum * tick / 5:.1f}")
    charts.plots.label(svg, 18, 94, "ms")
    for ordinal in (2, 8, 16, 24, 32):
        charts.plots.label(svg, left + (right - left) * (ordinal - 2) / 30 - 5, bottom + 24, ordinal)
    for index, row in enumerate(rows):
        color = charts.PALETTE[index]
        charts.plots.label(svg, 24 + index * 370, 81, row["label"])
        charts.plots.node(svg, "line", x1=300 + index * 370, x2=324 + index * 370,
                          y1=76, y2=76, stroke=color, stroke_width=3)
        points = [(left + (right - left) * i / 30, bottom - value / 1e6 / maximum * (bottom - top))
                  for i, value in enumerate(row["intervals_ns"])]
        charts.plots.node(svg, "polyline", points=" ".join(f"{x:.3f},{y:.3f}" for x, y in points),
                          fill="none", stroke=color, stroke_width=2)
        for ordinal, ((x, y), value) in enumerate(zip(points, row["intervals_ns"]), 2):
            circle = charts.plots.node(svg, "circle", cx=f"{x:.3f}", cy=f"{y:.3f}", r=2.5, fill=color)
            charts.plots.node(circle, "title").text = f"{row['label']}: output {ordinal}, {value} ns"
    charts.plots.label(svg, 320, 458, "Output token ordinal; token 1 belongs to TTFT")
    charts.plots.save(path, svg)


def generate(control, cooperative, output):
    # Validate everything before publishing even an empty output directory.
    inputs = [read_report(path, variant) for path, variant in zip((control, cooperative), VARIANTS)]
    rows = [row for _, row in inputs]
    rate_ratio = rows[1]["post_first_tokens_per_second"] / rows[0]["post_first_tokens_per_second"]
    tpot_change = (rows[1]["mean_tpot_seconds"] / rows[0]["mean_tpot_seconds"] - 1) * 100
    output.mkdir(parents=True, exist_ok=False)
    for (raw, _), variant in zip(inputs, VARIANTS):
        (output / f"{variant}-report-public.json").write_bytes(raw)
    plot_rates(output / "rates.svg", rows)
    plot_intervals(output / "intervals.svg", rows)
    with (output / "intervals.csv").open("x", newline="", encoding="utf-8") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerow(["variant", "output_token_ordinal", "host_interval_ns"])
        for row in rows:
            writer.writerows((row["variant"], ordinal, value) for ordinal, value in enumerate(row["intervals_ns"], 2))
    table = ["| Configuration | TTFT (s) | Mean TPOT (s) | Post-first tokens/s | Setup (s) |",
             "| --- | ---: | ---: | ---: | ---: |"]
    for row in rows:
        table.append(f"| {row['label']} | {row['ttft_seconds']:.6f} | {row['mean_tpot_seconds']:.6f} | "
                     f"{row['post_first_tokens_per_second']:.6f} | {row['setup_seconds']:.6f} |")
    (output / "table.md").write_text("\n".join(table) + "\n", encoding="utf-8")
    summary = {
        "schema": "FerricBf16ArgmaxPairV1", "authority": "none", "report_sha256": REPORT_SHA,
        "observations": [{key: value for key, value in row.items() if key != "intervals_ns"} for row in rows],
        "observed_rate_ratio": rate_ratio, "observed_mean_tpot_change_percent": tpot_change,
        "requests_per_variant": 1, "warmup_requests": 0, "intervals_per_variant": 31,
        "benchmark_qualified": False, "causal_or_statistical_speedup_claim": False,
        "gpu_timestamps": False, "gpu_overlap_measured": False, "standalone_argmax_timing": False,
        "nonclaim": "Descriptive single-request host observations; no stable gain or isolated argmax attribution.",
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    hashes = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}" for path in sorted(output.iterdir())]
    (output / "SHA256SUMS").write_text("\n".join(hashes) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--control", type=Path, required=True)
    parser.add_argument("--cooperative", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    generate(args.control, args.cooperative, args.output)


if __name__ == "__main__":
    main()
