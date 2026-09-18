#!/usr/bin/env python3
"""Plot validated one-request Target8B observations without mixing runtimes."""

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import sys
import types

import target_decode_profile_v1 as validation

# Reuse the existing bounded SVG writer, typography and rate-chart conventions.
sys.path.insert(0, str(Path(__file__).with_name("qwen-decode-report")))
import plots  # noqa: E402

LEGACY = {"baseline": (False, False), "cache": (True, False), "operational": (True, True)}
BATCH = {"baseline-host": ("baseline", "host-staged-reuse-v3", False),
         "baseline-device": ("baseline", "device-tp1-v3", False),
         "wave-host": ("wave", "host-staged-reuse-v3", False),
         "wave-device": ("wave", "device-tp1-v3", False),
         "wave-device-sequences": ("wave", "device-tp1-v3", True)}
HEAD = {"baseline-bf16-v7-control": ("baseline", "bf16-v7-control", "BF16"),
        "baseline-fp32-v7": ("baseline", "fp32-v7", "FP32"),
        "mfma-fp32-v7": ("mfma", "fp32-v7", "FP32")}
ORDERED = {"serial-control": False, "ordered-scalar-v3": True}
ORDERED_CHECKER_SHA256 = "8f06cc06288a84f347a56957454153ffa6416530561faed26a0e5ab6cd01b9b2"
ORDERED_CONTROLLER_SHA256 = "d24a93a99215054c74c35b0ba2778361a817f50376edf0bbc20ea576668cdcac"
FAMILIES = {"legacy": LEGACY, "batch": BATCH, "head": HEAD, "ordered": ORDERED}
LABELS = {"baseline": "Baseline", "cache": "Admission cache", "operational": "Cache + operational",
          "baseline-host": "Baseline + host reuse", "baseline-device": "Baseline + device residual",
          "wave-host": "Wave + host reuse",
          "wave-device": "Wave + device residual", "wave-device-sequences": "Wave + serial IPC groups",
          "baseline-bf16-v7-control": "Scalar + BF16 logits", "baseline-fp32-v7": "Scalar + FP32 logits",
          "mfma-fp32-v7": "MFMA + FP32 logits", "serial-control": "Serial control",
          "ordered-scalar-v3": "Ordered scalar groups"}
IDENTITIES = ("controller_sha256", "worker_sha256", "artifact_hsaco_id",
              "artifact_manifest_id", "artifact_handoff_id")
PALETTE = ("#257b75", "#ac4b60", "#5965a5", "#78642b")


def require(value, message):
    if not value:
        raise ValueError(message)


def ordered_metadata(data, variant):
    # Reuse only the exact reviewed checker bytes, never a stale module cache.
    path = Path(__file__).with_name("target_ordered_scalar_v3.py")
    source = validation.read_bounded(path, 131072)
    require(hashlib.sha256(source).hexdigest() == ORDERED_CHECKER_SHA256, "frozen ordered checker source")
    checker = types.ModuleType("_frozen_ordered_plot_contract")
    checker.__file__ = str(path)
    exec(compile(source, str(path), "exec"), checker.__dict__)
    same = checker.core.same
    ordered = ORDERED[variant]
    for key, expected in {
        "variant": variant, "model": "Qwen/Qwen3-8B", "profile": "single-unwarmed-request",
        "warmup_requests": 0, "measured_requests": 1, "processed_kv_tokens": 36, "dispatches": 22176,
        "generated_tokens": checker.core.TOKENS,
        "generated_utf8_bytes": list("".join(checker.core.PIECES).encode()),
        "runtime_profiling": False, "performance_qualified": False,
        "gpu_timestamps": False, "overlap_measured": False, "completion_polls_measured": False,
        "comparator_sha256": ORDERED_CHECKER_SHA256,
        "common_comparator_sha256": checker.CORE_SHA256,
        "counter_source_sha256": checker.COUNTER_SOURCE_SHA256,
        "identities": {"controller_sha256": ORDERED_CONTROLLER_SHA256,
                       "worker_sha256": checker.WORKER_SHA256, **checker.ARTIFACT},
    }.items():
        same(data[key], expected, "ordered plot " + key)
    require("runtime_counters" not in data and "derived_host_observations" not in data,
            "instrumented counters are not ordinary rate observations")
    config = data["configuration"]
    same(config, {
        "batch_tokens": 1, "prefill_chunk": 1, "context_tokens": 64, "physical_pages": 4,
        "prefix_cache": False, "output_head_pruning": False, "runtime_ordered_batches": ordered,
        "ordered_batch_profile": checker.ORDERED_PROFILE if ordered else None,
        "kernel_profile": "v3-wave", "collective": "device-tp1-v3", "host_timing_enabled": False,
        "target_only": True, "speculation": False, "head_precision": "BF16",
        "performance_profile": {"runtime_cache_admission": True, "runtime_operational": True,
                                "runtime_profiling": False, "dispatch_sequences": False,
                                "queue_rollover": False, "projection": "baseline", "attention": "baseline"},
    }, "exact ordered plot configuration")
    counts = checker.execution_counts({"runtime_ordered_batches": ordered})
    same(data["execution_counts"], counts, "packet and source-frontier accounting")
    timing = data["timing"]
    same(timing["decode_interval_count"], 31, "ordered interval count")
    require(type(timing["decode_intervals_ns"]) is list, "ordered interval list")
    for value in [*timing["decode_intervals_ns"], timing["admission_ttft_ns"], timing["generation_ns"]]:
        checker.core.integer(value, 1, 2**64 - 1, "ordered exact u64 timestamp")
    return {"execution_counts": counts, "weight_precision": "BF16", "activation_precision": "BF16",
            "logits_precision": "BF16", "completion_polls_measured": False}


def observation(data, family, variant, report_sha):
    cohort_metadata = {}
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
        is_head = family == "head"
        is_ordered = family == "ordered"
        schemas = {"batch": "FerricTargetBatchDecodeObservationV1", "head": "FerricTargetHeadDecodeObservationV1",
                   "ordered": "FerricTargetOrderedScalarObservationV1"}
        require(family in schemas and variant in FAMILIES[family]
                and data["schema"] == schemas[family], "paged plot profile")
        require(data["passed"] is True and data["reference_tokens_and_bytes_match"] is True
                and data["benchmark_qualified"] is False
                and data["warmup_requests"] == 0 and data["measured_requests"] == 1,
                "batch evidence scope")
        require(len(data["generated_tokens"]) == 32 and data["processed_kv_tokens"] == 36,
                "batch exact completed work")
        config = data["configuration"]
        profile = config["performance_profile"]
        if is_head:
            import target_head_decode_v1 as head_validation

            projection, precision, logits = HEAD[variant]
            collective, sequences = "host-staged-reuse-v3", False
            require(data["model"] == "Qwen/Qwen3-8B" and data["profile"] == "single-unwarmed-request",
                    "head model and observation profile")
            require(type(data["warmup_requests"]) is int and type(data["measured_requests"]) is int
                    and type(data["processed_kv_tokens"]) is int and type(data["dispatches"]) is int,
                    "head exact request and dispatch count types")
            for key in ("generated_tokens", "generated_utf8_bytes"):
                require(type(data[key]) is list and all(type(value) is int for value in data[key]),
                        "head exact integer token/byte types")
            require(data["generated_tokens"] == head_validation.batch.TOKENS
                    and data["generated_utf8_bytes"] == list("".join(head_validation.batch.PIECES).encode()),
                    "head frozen reference tokens and bytes")
            require(data["variant"] == variant and data["head_precision"] == precision
                    and data["logits_precision"] == logits and data["weight_precision"] == "BF16"
                    and data["activation_precision"] == "BF16", "explicit head precision contract")
            require(data["shared_checker_sha256"] == head_validation.BASE_CHECKER_SHA256,
                    "head shared checker pin")
            require(set(data["identities"]) == set(IDENTITIES), "head main identity roster")
            for key in ("batch_tokens", "prefill_chunk", "context_tokens", "physical_pages"):
                require(type(config[key]) is int, "head exact geometry types")
            timing = data["timing"]
            require(type(timing["decode_interval_count"]) is int and timing["decode_interval_count"] == 31
                    and type(timing["decode_intervals_ns"]) is list, "head exact interval roster")
            for value in [*timing["decode_intervals_ns"], timing["admission_ttft_ns"], timing["generation_ns"]]:
                require(type(value) is int and 0 < value < 2**64, "head exact positive u64 timestamps")
            require(config["target_only"] is True and config["speculation"] is False, "head target-only contract")
            workspace = data["fp32_head_workspace_bytes"]
            require(type(workspace) is int and workspace == (9_723_904 if logits == "FP32" else 0),
                    "head exact workspace")
            head_ids = data["head_artifact"]
            require(type(head_ids) is dict and set(head_ids) ==
                    {"artifact_hsaco_id", "artifact_manifest_id", "artifact_handoff_id"}, "head artifact roster")
            for value in head_ids.values():
                validation.digest(value)
            cohort_metadata = {"head_artifact": dict(head_ids), "head_precision": precision,
                               "weight_precision": "BF16", "activation_precision": "BF16",
                               "logits_precision": logits, "fp32_head_workspace_bytes": workspace}
        elif is_ordered:
            cohort_metadata = ordered_metadata(data, variant)
            projection, collective, sequences = "baseline", "device-tp1-v3", False
        else:
            projection, collective, sequences = BATCH[variant]
        require(profile["projection"] == projection and config["collective"] == collective
                and profile["dispatch_sequences"] is sequences, "batch variant flags")
        for key in ("runtime_cache_admission", "runtime_operational"):
            require(profile[key] is True, "batch runtime policy")
        require(profile["attention"] == "baseline" and profile["queue_rollover"] is False
                and profile["runtime_profiling"] is False, "batch execution profile")
        for key in ("prefix_cache", "output_head_pruning", "host_timing_enabled"):
            require(config[key] is False, "unexpected batch option")
        ordered = ORDERED[variant] if is_ordered else False
        require(config["runtime_ordered_batches"] is ordered, "explicit ordered submission mode")
        require(config["kernel_profile"] == ("v3-mfma" if is_head else "v3-wave") and config["batch_tokens"] == 1
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
                         "output_head_pruning": False, "runtime_ordered_batches": ordered}
        if is_head:
            configuration.update(kernel_profile="v3-mfma", target_only=True, speculation=False)
        elif is_ordered:
            configuration.update(kernel_profile="v3-wave", target_only=True, speculation=False,
                                 head_precision="BF16", ordered_batch_profile=config["ordered_batch_profile"])
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
        **cohort_metadata,
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
        if first["family"] == "head":
            require(row["head_artifact"] == first["head_artifact"], "do not combine unmatched head artifacts")


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
    require(set(families) in ({"legacy", "batch"}, {"legacy", "batch", "head"}, set(FAMILIES)),
            "plot family roster")
    require(any(families.values()), "no observed reports supplied")
    summaries = []
    for family, rows in families.items():
        if not rows:
            continue
        order = FAMILIES[family]
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
        title = "Qwen3-8B " + {"legacy": "legacy runtime", "batch": "paged runtime",
                              "head": "v7 head (BF16 weights/activations)",
                              "ordered": "ordered scalar (BF16, no GPU-overlap claim)"}[family]
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
    parser.add_argument("--head", action="append", default=[], metavar="VARIANT=REPORT")
    parser.add_argument("--ordered", action="append", default=[], metavar="VARIANT=REPORT")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    families = {family: [] for family in FAMILIES}
    for family in families:
        for value in getattr(args, family):
            variant, separator, path = value.partition("=")
            require(separator and variant in FAMILIES[family], "plot variant argument")
            raw = validation.read_bounded(path, 1024 * 1024)
            families[family].append(observation(validation.decode_json(raw), family, variant, hashlib.sha256(raw).hexdigest()))
    require(any(families.values()), "no observed reports supplied")
    generate(args.output, families)


if __name__ == "__main__":
    main()
