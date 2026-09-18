#!/usr/bin/env python3
"""Plot validated one-request Target8B observations without mixing runtimes."""

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import sys

import target_decode_profile_v1 as validation

# Reuse the existing bounded SVG writer, typography and rate-chart conventions.
sys.path.insert(0, str(Path(__file__).with_name("qwen-decode-report")))
import plots  # noqa: E402

LEGACY = {"baseline": (False, False), "cache": (True, False), "operational": (True, True)}
BATCH = {"baseline-host": ("baseline", "host-staged-reuse-v3", False),
         "wave-host": ("wave", "host-staged-reuse-v3", False),
         "wave-device": ("wave", "device-tp1-v3", False),
         "wave-device-sequences": ("wave", "device-tp1-v3", True)}
LABELS = {"baseline": "Baseline", "cache": "Admission cache", "operational": "Cache + operational",
          "baseline-host": "Baseline + host reuse", "wave-host": "Wave + host reuse",
          "wave-device": "Wave + device residual", "wave-device-sequences": "Wave + serial IPC groups"}
IDENTITIES = ("controller_sha256", "worker_sha256", "artifact_hsaco_id",
              "artifact_manifest_id", "artifact_handoff_id")
PALETTE = ("#257b75", "#ac4b60", "#5965a5", "#78642b")


def require(value, message):
    if not value:
        raise ValueError(message)


def observation(data, family, variant, report_sha):
    require(data["authority"] == "none" and data["precision"] == "BF16"
            and type(data["tensor_parallel"]) is int and data["tensor_parallel"] == 1
            and type(data["concurrent_requests"]) is int and data["concurrent_requests"] == 1,
            "plot requires BF16 single-request TP1")
    require(data["reference_sha256"] == validation.REFERENCE_SHA256, "plot reference identity")
    if family == "legacy":
        require(variant in LEGACY and data["schema"] == "FerricQwen3TargetDecodeComparisonV1", "legacy plot profile")
        require(data["all_reference_checks_passed"] is True and data["all_workers_exited"] is True
                and data["performance_qualified"] is False and data["benchmark_comparable"] is False
                and data["gpu_timestamps"] is False and data["overlap_measured"] is False,
                "legacy evidence scope")
        require(type(data["warmup_runs"]) is int and data["warmup_runs"] == 0
                and type(data["measured_runs"]) is int and data["measured_runs"] == 1
                and type(data["output_tokens_per_request"]) is int and data["output_tokens_per_request"] == 32
                and data["capacity"] == 64, "legacy matched workload")
        flags = LEGACY[variant]
        require(data["runtime_cache_admission"] is flags[0] and data["runtime_operational"] is flags[1],
                "legacy variant flags")
        for key in ("runtime_profile", "runtime_sequences", "runtime_ordered_batches", "runtime_rollover",
                    "kv_prefix_cache", "speculation"):
            require(data[key] is False, "unexpected legacy option")
        require(data["target_only"] is True and len(data["runs"]) == 1, "legacy target/run contract")
        run = data["runs"][0]
        require(run["warmup"] is False and run["reference_passed"] is True and run["run"] == 0,
                "legacy measured run")
        intervals = run["decode_intervals_seconds"]
        ttft = run["ttft_seconds"]
        generation = run["generation_seconds"]
        setup = data["setup_seconds"]
        rate = data["summary"]["post_first_tokens_per_second"]
        identities = {key: data[key] for key in IDENTITIES}
        revision = data["model_revision"]
        count = data["total_dispatches"]
        capture_sha = data["input_sha256"]["capture"]
        validator_sha = data["input_sha256"]["validator"]
        require(count == 19584 and run["kv_tokens_processed"] == 36, "legacy exact completed work")
        configuration = {"runtime_cache_admission": flags[0], "runtime_operational": flags[1],
                         "runtime_profiling": False, "dispatch_sequences": False,
                         "queue_rollover": False, "capacity": 64, "kv_prefix_cache": False}
    else:
        require(family == "batch" and variant in BATCH
                and data["schema"] == "FerricTargetBatchDecodeObservationV1", "batch plot profile")
        require(data["passed"] is True and data["reference_tokens_and_bytes_match"] is True
                and data["benchmark_qualified"] is False
                and data["warmup_requests"] == 0 and data["measured_requests"] == 1,
                "batch evidence scope")
        require(len(data["generated_tokens"]) == 32 and data["processed_kv_tokens"] == 36,
                "batch exact completed work")
        config = data["configuration"]
        profile = config["performance_profile"]
        projection, collective, sequences = BATCH[variant]
        require(profile["projection"] == projection and config["collective"] == collective
                and profile["dispatch_sequences"] is sequences, "batch variant flags")
        for key in ("runtime_cache_admission", "runtime_operational"):
            require(profile[key] is True, "batch runtime policy")
        require(profile["attention"] == "baseline" and profile["queue_rollover"] is False
                and profile["runtime_profiling"] is False, "batch execution profile")
        for key in ("prefix_cache", "output_head_pruning", "runtime_ordered_batches", "host_timing_enabled"):
            require(config[key] is False, "unexpected batch option")
        require(config["kernel_profile"] == "v3-wave" and config["batch_tokens"] == 1
                and config["prefill_chunk"] == 1 and config["context_tokens"] == 64
                and config["physical_pages"] == 4, "batch matched workload")
        timing = data["timing"]
        intervals = [value / 1e9 for value in timing["decode_intervals_ns"]]
        ttft = timing["admission_ttft_ns"] / 1e9
        generation = timing["generation_ns"] / 1e9
        setup = timing["setup_seconds"]
        rate = timing["post_first_tokens_per_second"]
        identities = {key: data["identities"][key] for key in IDENTITIES}
        revision = data["revision"]
        count = data["dispatches"]
        capture_sha = data["input_sha256"]["capture"]
        validator_sha = data["comparator_sha256"]
        require(count == 36 * (616 if collective == "device-tp1-v3" else 544), "batch packet count")
        configuration = {"runtime_cache_admission": True, "runtime_operational": True,
                         "runtime_profiling": False, "dispatch_sequences": sequences,
                         "queue_rollover": False, "projection": projection, "collective": collective,
                         "attention": "baseline", "context_tokens": 64, "physical_pages": 4,
                         "batch_tokens": 1, "prefill_chunk": 1, "prefix_cache": False,
                         "output_head_pruning": False, "runtime_ordered_batches": False}
    require(revision == validation.REVISION and len(intervals) == 31, "reference revision/interval count")
    for value in [*intervals, ttft, generation, setup, rate]:
        validation.finite(value, "plot observed duration/rate")
    for value in [*identities.values(), capture_sha, report_sha, validator_sha]:
        validation.digest(value)
    total = validation.finite(math.fsum(intervals), "plot decode duration")
    recomputed_rate = validation.finite(31 / total, "plot recomputed rate")
    validation.near(rate, recomputed_rate, "plot rate")
    validation.near(generation, ttft + total, "plot request duration")
    return {
        "schema": "FerricTarget8BPlotObservationV1", "authority": "none", "family": family,
        "variant": variant, "label": LABELS[variant], "model": "Qwen/Qwen3-8B",
        "model_revision": revision, "precision": "BF16", "concurrent_requests": 1, "tensor_parallel": 1,
        "measured_requests": 1, "warmup_requests": 0, "generated_tokens": 32,
        "processed_kv_tokens": 36, "completed_dispatches": count,
        "decode_intervals_seconds": intervals, "post_first_tokens_per_second": recomputed_rate,
        "tpot_seconds": total / 31, "ttft_seconds": ttft, "generation_seconds": generation,
        "setup_seconds": setup, "identities": identities,
        "configuration": configuration, "reference_tokens_and_bytes_match": True,
        "reference_sha256": validation.REFERENCE_SHA256, "capture_sha256": capture_sha,
        "validated_report_sha256": report_sha, "validator_sha256": validator_sha,
        "performance_qualified": False, "cpu_isolation_verified": False, "gpu_overlap_measured": False,
    }


def matched(observations):
    require(0 < len(observations) <= 4, "plot family size")
    first = observations[0]
    require(len({row["variant"] for row in observations}) == len(observations), "duplicate plotted variant")
    for row in observations:
        require(row["family"] == first["family"] and row["identities"] == first["identities"]
                and row["model_revision"] == first["model_revision"]
                and row["reference_sha256"] == first["reference_sha256"],
                "do not combine unmatched controllers, artifacts, models or references")


def interval_plot(path, rows, title):
    matched(rows)
    maximum = validation.finite(max(max(row["decode_intervals_seconds"]) * 1000 for row in rows) * 1.12,
                                "plot vertical extent")
    svg = plots.canvas(title, 490)
    plots.label(svg, 24, 52, "One unwarmed request per configuration; host intervals, not GPU durations or qualified benchmarks.")
    left, right, top, bottom = 88, 1060, 110, 410
    for tick in range(6):
        y = bottom - (bottom - top) * tick / 5
        plots.node(svg, "line", x1=left, x2=right, y1=y, y2=y, stroke="#dddddd")
        plots.label(svg, 18, y + 4, f"{maximum * tick / 5:.1f}")
    plots.label(svg, 18, 94, "ms")
    for token in (2, 8, 16, 24, 32):
        x = left + (right - left) * (token - 2) / 30
        plots.label(svg, x - 5, bottom + 24, str(token))
    plots.label(svg, 325, 463, "Output token ordinal (first token excluded); lower interval is better")
    for index, row in enumerate(rows):
        color = PALETTE[index]
        plots.node(svg, "line", x1=24 + index * 265, x2=43 + index * 265, y1=76, y2=76,
                   stroke=color, stroke_width=3)
        plots.label(svg, 50 + index * 265, 81, row["label"], 12)
        coordinates = [(left + (right - left) * ordinal / 30,
                        bottom - (value * 1000 / maximum) * (bottom - top))
                       for ordinal, value in enumerate(row["decode_intervals_seconds"])]
        plots.node(svg, "polyline", points=" ".join(f"{x:.3f},{y:.3f}" for x, y in coordinates),
                   fill="none", stroke=color, stroke_width=2)
        for ordinal, ((x, y), value) in enumerate(zip(coordinates, row["decode_intervals_seconds"]), 2):
            circle = plots.node(svg, "circle", cx=f"{x:.3f}", cy=f"{y:.3f}", r=2.5, fill=color)
            plots.node(circle, "title").text = f"{row['label']}: output {ordinal}, {value * 1000:.6f} ms"
    plots.save(path, svg)


def preflight(families):
    require(set(families) == {"legacy", "batch"}, "plot family roster")
    summaries = []
    for family, rows in families.items():
        if not rows:
            continue
        order = LEGACY if family == "legacy" else BATCH
        rows.sort(key=lambda row: list(order).index(row["variant"]))
        matched(rows)
        validation.finite(max(max(row["decode_intervals_seconds"]) * 1000 for row in rows) * 1.12,
                          "plot vertical extent")
        maximum_rate = max(row["post_first_tokens_per_second"] for row in rows)
        validation.finite(maximum_rate * 1.12, "rate chart extent")
        validation.finite(720 * maximum_rate, "rate chart scaled extent")
        base_rate = rows[0]["post_first_tokens_per_second"]
        for row in rows:
            validation.finite(row["post_first_tokens_per_second"] / base_rate, "observed base rate ratio")
        for before, after in zip(rows, rows[1:]):
            ratio = validation.finite(after["post_first_tokens_per_second"] / before["post_first_tokens_per_second"],
                                      "observed adjacent rate ratio")
            tpot_ratio = validation.finite(after["tpot_seconds"] / before["tpot_seconds"], "observed TPOT ratio")
            change = (tpot_ratio - 1) * 100
            require(math.isfinite(change), "observed signed TPOT change")
            summaries.append({"family": family, "from": before["variant"], "to": after["variant"],
                              "observed_rate_ratio": ratio, "observed_tpot_change_percent": change,
                              "causal_or_statistical_speedup_claim": False})
    return summaries


def generate(output, families):
    # Reject invalid derived values before creating any partially publishable
    # directory, table or SVG, not merely during final JSON serialization.
    summaries = preflight(families)
    output.mkdir(parents=True, exist_ok=False)
    for family, rows in families.items():
        if not rows:
            continue
        title = "Qwen3-8B " + ("legacy runtime" if family == "legacy" else "paged runtime")
        interval_plot(output / f"{family}-intervals.svg", rows, title + ": observed decode intervals")
        if len(rows) >= 2:
            plots.rates(output / f"{family}-rates.svg", [(row["label"], row["post_first_tokens_per_second"])
                                                        for row in rows], title + ": observed decode rates")
        for row in rows:
            with (output / f"{family}-{row['variant']}-report.json").open("x", encoding="utf-8") as stream:
                json.dump(row, stream, indent=2, sort_keys=True, allow_nan=False)
                stream.write("\n")
        with (output / f"{family}-intervals.csv").open("x", newline="", encoding="utf-8") as stream:
            writer = csv.writer(stream, lineterminator="\n")
            writer.writerow(["variant", "output_token_ordinal", "decode_interval_ms"])
            for row in rows:
                writer.writerows((row["variant"], index, f"{value * 1000:.9f}")
                                 for index, value in enumerate(row["decode_intervals_seconds"], 2))
        lines = ["| Configuration | TTFT (s) | Mean TPOT (s) | Post-first tokens/s | Observed rate / first | Setup (s) |",
                 "| --- | ---: | ---: | ---: | ---: | ---: |"]
        base_rate = rows[0]["post_first_tokens_per_second"]
        for row in rows:
            lines.append(f"| {row['label']} | {row['ttft_seconds']:.6f} | {row['tpot_seconds']:.6f} | "
                         f"{row['post_first_tokens_per_second']:.6f} | {row['post_first_tokens_per_second'] / base_rate:.3f}x | "
                         f"{row['setup_seconds']:.6f} |")
        (output / f"{family}-table.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (output / "observed-contrasts.json").write_text(json.dumps({
        "schema": "FerricTarget8BObservedContrastsV1", "authority": "none", "contrasts": summaries,
        "nonclaim": "Descriptive ratios of single unwarmed requests. No controlled causal contribution or additive optimization attribution."},
        indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    hashes = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}" for path in sorted(output.iterdir())]
    (output / "SHA256SUMS").write_text("\n".join(hashes) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--legacy", action="append", default=[], metavar="VARIANT=REPORT")
    parser.add_argument("--batch", action="append", default=[], metavar="VARIANT=REPORT")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    families = {"legacy": [], "batch": []}
    for family in families:
        for value in getattr(args, family):
            variant, separator, path = value.partition("=")
            require(separator and variant in (LEGACY if family == "legacy" else BATCH), "plot variant argument")
            raw = validation.read_bounded(path, 1024 * 1024)
            families[family].append(observation(validation.decode_json(raw), family, variant, hashlib.sha256(raw).hexdigest()))
    require(any(families.values()), "no observed reports supplied")
    generate(args.output, families)


if __name__ == "__main__":
    main()
