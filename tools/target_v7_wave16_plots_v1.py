#!/usr/bin/env python3
"""Revalidate and plot the separate v7-wave16 baseline/wave capture pair.

Raw input stays private. Every public report is regenerated with the unchanged
SHA-pinned comparator before any plot output directory is created.
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


CHECKER_SHA256 = "0b6ed462f59a17b948b5c85779e55f42eb1ffdfaebab61ca2b23fb162db474f1"
VARIANTS = ("mfma-device-baselineattention", "mfma-device-waveattention")
LABELS = ("Baseline attention", "Wave64 attention")


def load_checker():
    path = Path(__file__).with_name("target_v7_wave16_decode_v1.py")
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
    if hashlib.sha256(raw).hexdigest() != CHECKER_SHA256:
        raise ValueError("frozen v7 checker digest")
    module = types.ModuleType("_frozen_v7_wave16_plot_checker")
    module.__file__ = str(path)
    exec(compile(raw, str(path), "exec"), module.__dict__)
    return module


def revalidate(checker, directory, variant, reference):
    read = checker.batch.read_bounded
    report = checker.compare(read(directory / "capture.ndjson", 1048576),
                             read(directory / "exit-status.txt", 16),
                             read(directory / "workload.json", 65536), reference,
                             read(directory / "predeclared-expectation.json", 65536),
                             read(directory / "stderr.log", 65536))
    report["comparator_sha256"] = CHECKER_SHA256
    checker.batch.same(report["variant"], variant, "exact plot variant")
    raw = read(directory / "report-public.json", 1048576)
    checker.batch.same(checker.batch.json_value(raw), report, "regenerated public report")
    checker.batch.require(read(directory / "postcheck-status.txt", 256) ==
                          b"controller=0 idle=0 topology=0 inputs=0 plan=0\n", "successful harness postchecks")
    return report, raw


def rows_and_contrast(checker, reports):
    checker.batch.require(type(reports) is list and len(reports) == 2, "exact plot pair")
    rows = []
    for report, variant, label in zip(reports, VARIANTS, LABELS):
        checker.batch.same(report["schema"], "FerricTargetV7Wave16ObservationV1", "v7 schema")
        checker.batch.same(report["variant"], variant, "ordered v7 pair")
        checker.batch.same(report["comparator_sha256"], CHECKER_SHA256, "v7 checker pin")
        checker.batch.same(report["generated_tokens"], checker.batch.TOKENS, "full token roster")
        checker.batch.same(report["generated_utf8_bytes"], list("".join(checker.batch.PIECES).encode()), "full byte roster")
        checker.batch.same(report["dispatches"], 22176, "unchanged packet count")
        checker.batch.same(report["benchmark_qualified"], False, "one request is not qualified")
        for key, value in (("weight_precision", "BF16"), ("activation_precision", "BF16"), ("logits_precision", "FP32")):
            checker.batch.same(report[key], value, "explicit precision")
        timing = report["timing"]
        intervals = timing["decode_intervals_ns"]
        checker.batch.require(type(intervals) is list and len(intervals) == 31, "31 actual intervals")
        for value in intervals:
            checker.batch.integer(value, 1, 2**64 - 1, "exact u64 interval")
        rate = 31 * 1e9 / sum(intervals)
        checker.batch.same(rate, timing["post_first_tokens_per_second"], "recomputed rate")
        checker.batch.same(sum(intervals) / 31, timing["mean_tpot_ns"], "recomputed TPOT")
        rows.append({"family": "v7-wave16", "variant": variant, "label": label,
                     "identities": report["identities"], "head_artifact": report["head_artifact"],
                     "model_revision": report["revision"], "reference_sha256": report["reference_sha256"],
                     "decode_intervals_seconds": [value / 1e9 for value in intervals],
                     "post_first_tokens_per_second": rate})
    checker.batch.same(rows[0]["head_artifact"], rows[1]["head_artifact"], "matching head image")
    drawing.matched(rows)
    ratio = rows[1]["post_first_tokens_per_second"] / rows[0]["post_first_tokens_per_second"]
    contrast = {"schema": "FerricV7Wave16ObservedContrastV1", "observed_rate_ratio": ratio,
                "observed_tpot_change_percent": (1 / ratio - 1) * 100,
                "measured_requests_per_variant": 1, "warmup_requests": 0,
                "cpu_isolation_verified": False, "performance_qualified": False,
                "causal_or_stable_speedup_claim": False, "gpu_overlap_measured": False}
    checker.batch.require(all(math.isfinite(value) for value in (
        ratio, contrast["observed_tpot_change_percent"],
        max(max(row["decode_intervals_seconds"]) for row in rows) * 1120,
        max(row["post_first_tokens_per_second"] for row in rows) * 720)), "finite chart extents")
    return rows, contrast


def generate(output, checker, reports, raw_reports):
    rows, contrast = rows_and_contrast(checker, reports)
    encoded = json.dumps(contrast, indent=2, sort_keys=True, allow_nan=False) + "\n"
    output.mkdir(parents=True, exist_ok=False)
    drawing.interval_plot(output / "intervals.svg", rows, "Qwen3-8B: v7-head attention comparison")
    drawing.plots.rates(output / "rates.svg", [(row["label"], row["post_first_tokens_per_second"]) for row in rows],
                        "Qwen3-8B: observed v7-head attention rates")
    for name, raw in zip(("baseline-report.json", "wave-report.json"), raw_reports):
        with (output / name).open("xb") as stream:
            stream.write(raw)
    with (output / "intervals.csv").open("x", newline="", encoding="utf-8") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerow(["variant", "output_token_ordinal", "decode_interval_ms"])
        for row in rows:
            writer.writerows((row["variant"], index, f"{value * 1000:.9f}")
                             for index, value in enumerate(row["decode_intervals_seconds"], 2))
    (output / "observed-contrast.json").write_text(encoded, encoding="utf-8")
    sums = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}" for path in sorted(output.iterdir())]
    (output / "SHA256SUMS").write_text("\n".join(sums) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("baseline-capture", "wave-capture", "reference", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    checker = load_checker()
    reference = checker.batch.read_bounded(args.reference, 131072)
    pairs = [revalidate(checker, directory, variant, reference) for directory, variant in
             zip((args.baseline_capture, args.wave_capture), VARIANTS)]
    generate(args.output, checker, [pair[0] for pair in pairs], [pair[1] for pair in pairs])
    print("Two full captures independently revalidated; 62 actual intervals plotted; no GPU work.")


if __name__ == "__main__":
    main()
