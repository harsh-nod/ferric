#!/usr/bin/env python3
"""Plot raw-capture-revalidated feature observations without changing identities.

CPU only. Exact pinned comparators and all seven cohort-specific postchecks
must pass before any public output directory is created.
"""

import argparse
import copy
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
    "paired-prefetch": {
        "checker": "target_mfma_paired_prefetch_v1.py",
        "sha256": "da77a498a9a429202bd247bd724e457732621716b5ad7f4f2e301cf14f5b41b2",
        "schema": "FerricTargetMfmaPairedPrefetchObservationV1",
        "variants": ("baseline-main", "paired-main"),
        "labels": ("Feature-off main image", "Paired-prefetch main image"),
        "title": "Qwen3-8B: image-level feature observation",
        "axis": "main-image",
        "identity_fields": ("artifact_hsaco_id", "artifact_manifest_id", "artifact_handoff_id"),
        "controller_cohort": "mfma-paired-prefetch-matched-v2-controller-v7",
        "postchecks": b"controller=0 idle=0 topology=0 inputs=0 plan=0 source=0 paired_source=0\n",
    },
    "preparation-worker": {
        "checker": "target_full_forward_preparation_v1.py",
        "sha256": "7e654a33f32a3d382638eafceada3c0db44aa46237533836a51302e6eea4483f",
        "schema": "FerricTargetFullForwardPreparationObservationV1",
        "variants": ("per-dispatch-checks", "transaction-checks"),
        "labels": ("Per-dispatch checks", "Transaction checks"),
        "title": "Qwen3-8B: worker preparation observation",
        "axis": "runtime-worker",
        "identity_fields": ("worker_sha256",),
        "controller_cohort": "full-forward-preparation-fence-v1-controller-v7",
        "postchecks": b"controller=0 idle=0 topology=0 inputs=0 plan=0 source=0 worker_source=0\n",
    },
}


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
        raise ValueError("frozen feature-ablation checker digest")
    module = types.ModuleType("_frozen_feature_ablation_plot_" + family.replace("-", "_"))
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
    checker.core.require(read(directory / "postcheck-status.txt", 256) == COHORTS[family]["postchecks"],
                         "all seven exact cohort postchecks")
    return report, raw


def expected_identities(checker, family, variant):
    if family == "paired-prefetch":
        return {"controller_sha256": checker.CONTROLLER_SHA256,
                "worker_sha256": checker.WORKER_SHA256, **checker.ARTIFACTS[variant]}
    return {"controller_sha256": checker.CONTROLLER_SHA256,
            "worker_sha256": checker.WORKERS[variant], **checker.ARTIFACT}


def matched_axis(checker, family, rows):
    """Require the declared difference; preserve every real identity in each row."""
    core = checker.core
    cohort = COHORTS[family]
    core.require(type(rows) is list and len(rows) == 2, "exact ablation row pair")
    for row, variant in zip(rows, cohort["variants"]):
        core.same(row["variant"], variant, "ordered ablation variant")
        core.same(row["family"], family, "one ablation family")
        core.same(row["identities"], expected_identities(checker, family, variant), "actual ablation identities")
        core.same(row["model_revision"], core.REVISION, "same model revision")
        core.same(row["reference_sha256"], core.REFERENCE_SHA256, "same reference")
    for key in core.IDENTITIES:
        if key in cohort["identity_fields"]:
            core.require(rows[0]["identities"][key] != rows[1]["identities"][key], "declared identity axis differs")
        else:
            core.same(rows[0]["identities"][key], rows[1]["identities"][key], "non-axis identity unchanged")


def rows_and_contrast(checker, family, reports):
    cohort = COHORTS[family]
    core = checker.core
    core.require(type(reports) is list and len(reports) == 2, "exact complete ablation pair")
    rows = []
    for index, (report, variant, label) in enumerate(zip(reports, cohort["variants"], cohort["labels"])):
        expected = {
            "schema": cohort["schema"], "variant": variant,
            "controller_cohort": cohort["controller_cohort"], "comparator_sha256": cohort["sha256"],
            "authority": "none", "model": "Qwen/Qwen3-8B", "revision": core.REVISION,
            "reference_sha256": core.REFERENCE_SHA256, "passed": True,
            "reference_tokens_and_bytes_match": True, "tensor_parallel": 1, "concurrent_requests": 1,
            "warmup_requests": 0, "measured_requests": 1, "profile": "single-unwarmed-request",
            "processed_kv_tokens": 36, "dispatches": 22176, "generated_tokens": core.TOKENS,
            "generated_utf8_bytes": list("".join(core.PIECES).encode()),
            "weight_precision": "BF16", "activation_precision": "BF16", "logits_precision": "FP32",
            "benchmark_qualified": False, "performance_qualified": False, "runtime_profiling": False,
            "gpu_timestamps": False, "overlap_measured": False,
            "completion_polls_measured": False, "persistent_kernel": False,
            "common_comparator_sha256": checker.CORE_SHA256,
            "identities": expected_identities(checker, family, variant),
            "execution_counts": checker.execution_counts({"runtime_full_forward": True}),
            "head_artifact": checker.HEAD_ARTIFACT, "head_precision": "fp32-v7",
            "fp32_head_workspace_bytes": 9_723_904,
        }
        if family == "paired-prefetch":
            expected.update(
                paired_source_manifest_sha256=checker.PAIRED_SOURCE_MANIFEST_SHA256,
                main_canonical_descriptor_sha256=checker.MAIN_CANONICAL_DESCRIPTORS[variant],
                comparison_scope="matched-source-and-tools-image-feature-ablation",
                codegen_catalog_sha256=checker.CODEGEN_CATALOG_SHA256,
                changed_kernel_roots=checker.CHANGED_KERNEL_ROOTS,
                secondary_codegen_roots=checker.SECONDARY_CODEGEN_ROOTS,
                pure_prefetch_causal_gain_claimed=False,
                paired_prefetch_feature={"default_enabled": False, "selected": index == 1,
                                         "observed_rows": 1, "source_tp1_row_range": [1, 16],
                                         "multirow_gpu_validated": False})
        else:
            expected.update(worker_source_manifest_sha256=checker.WORKER_SOURCE_MANIFEST_SHA256,
                            transaction_preparation_fence=index == 1, currentness_check_count_measured=False)
        for key, value in expected.items():
            core.same(report[key], value, "exact ablation plot " + key)
        config = {
            "batch_tokens": 1, "prefill_chunk": 1, "context_tokens": 64, "physical_pages": 4,
            "prefix_cache": False, "output_head_pruning": False, "runtime_ordered_batches": False,
            "kernel_profile": "v3-mfma", "collective": "device-tp1-v3", "host_timing_enabled": False,
            "target_only": True, "speculation": False, "head_precision": "fp32-v7",
            "runtime_full_forward": True, "full_forward_profile": checker.PROFILE,
            "performance_profile": {"runtime_cache_admission": True, "runtime_operational": True,
                                    "runtime_profiling": False, "dispatch_sequences": False,
                                    "queue_rollover": False, "projection": "mfma", "attention": "wave"},
        }
        core.same(report["configuration"], config, "identical full-forward wave configuration")
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
        rows.append({"family": family, "variant": variant, "label": label,
                     "identities": copy.deepcopy(report["identities"]), "model_revision": report["revision"],
                     "reference_sha256": report["reference_sha256"],
                     "decode_intervals_seconds": [value / 1e9 for value in intervals],
                     "post_first_tokens_per_second": rate})
    matched_axis(checker, family, rows)
    ratio = rows[1]["post_first_tokens_per_second"] / rows[0]["post_first_tokens_per_second"]
    contrast = {
        "schema": "FerricFeatureAblationObservedContrastV1", "family": family,
        "title": cohort["title"], "differing_axis": cohort["axis"],
        "differing_identity_fields": list(cohort["identity_fields"]),
        "identities_by_variant": {row["variant"]: copy.deepcopy(row["identities"]) for row in rows},
        "head_artifact": copy.deepcopy(checker.HEAD_ARTIFACT),
        "controller_cohort": cohort["controller_cohort"], "observed_rate_ratio": ratio,
        "observed_tpot_change_percent": (1 / ratio - 1) * 100,
        "measured_requests_per_variant": 1, "warmup_requests": 0,
        "weight_precision": "BF16", "activation_precision": "BF16", "logits_precision": "FP32",
        "packets_per_request_each": 22176,
        "source_completion_frontiers": {variant: 36 for variant in cohort["variants"]},
        "frontier_count_scope": "source-derived schedule; not measured polls or GPU event count",
        "cpu_isolation_verified": False, "performance_qualified": False,
        "causal_or_stable_speedup_claim": False, "gpu_overlap_measured": False, "persistent_gpu_kernel": False,
        "target_700_tokens_per_second_claimed": False,
    }
    if family == "paired-prefetch":
        for key in ("paired_source_manifest_sha256", "comparison_scope", "codegen_catalog_sha256",
                    "changed_kernel_roots", "secondary_codegen_roots", "pure_prefetch_causal_gain_claimed"):
            contrast[key] = copy.deepcopy(reports[0][key])
        contrast["main_canonical_descriptors_by_variant"] = copy.deepcopy(checker.MAIN_CANONICAL_DESCRIPTORS)
        contrast["paired_prefetch_feature_by_variant"] = {
            report["variant"]: copy.deepcopy(report["paired_prefetch_feature"]) for report in reports}
    else:
        contrast.update(worker_source_manifest_sha256=checker.WORKER_SOURCE_MANIFEST_SHA256,
                        currentness_check_count_measured=False)
    core.require(all(math.isfinite(value) for value in (
        ratio, contrast["observed_tpot_change_percent"],
        max(max(row["decode_intervals_seconds"]) for row in rows) * 1120,
        max(row["post_first_tokens_per_second"] for row in rows) * 720)), "finite chart extents")
    return rows, contrast


def interval_plot(path, checker, family, rows, title):
    # drawing.interval_plot requires all identities equal. This renderer uses
    # the declared, checked axis without changing or dropping real identities.
    matched_axis(checker, family, rows)
    maximum = max(max(row["decode_intervals_seconds"]) * 1000 for row in rows) * 1.12
    svg = drawing.plots.canvas(title, 490)
    drawing.plots.label(svg, 24, 52, "One unwarmed request per variant; unqualified host intervals, not GPU durations.")
    left, right, top, bottom = 88, 1060, 110, 410
    for tick in range(6):
        y = bottom - (bottom - top) * tick / 5
        drawing.plots.node(svg, "line", x1=left, x2=right, y1=y, y2=y, stroke="#dddddd")
        drawing.plots.label(svg, 18, y + 4, f"{maximum * tick / 5:.1f}")
    drawing.plots.label(svg, 18, 94, "ms")
    for token in (2, 8, 16, 24, 32):
        x = left + (right - left) * (token - 2) / 30
        drawing.plots.label(svg, x - 5, bottom + 24, str(token))
    drawing.plots.label(svg, 325, 463, "Output token ordinal (first token excluded); lower interval is better")
    for index, row in enumerate(rows):
        color = drawing.PALETTE[index]
        drawing.plots.node(svg, "line", x1=24 + index * 300, x2=43 + index * 300, y1=76, y2=76,
                           stroke=color, stroke_width=3)
        drawing.plots.label(svg, 50 + index * 300, 81, row["label"], 12)
        coordinates = [(left + (right - left) * ordinal / 30,
                        bottom - (value * 1000 / maximum) * (bottom - top))
                       for ordinal, value in enumerate(row["decode_intervals_seconds"])]
        drawing.plots.node(svg, "polyline", points=" ".join(f"{x:.3f},{y:.3f}" for x, y in coordinates),
                           fill="none", stroke=color, stroke_width=2)
        for ordinal, ((x, y), value) in enumerate(zip(coordinates, row["decode_intervals_seconds"]), 2):
            circle = drawing.plots.node(svg, "circle", cx=f"{x:.3f}", cy=f"{y:.3f}", r=2.5, fill=color)
            drawing.plots.node(circle, "title").text = f"{row['label']}: output {ordinal}, {value * 1000:.6f} ms"
    drawing.plots.save(path, svg)


def generate(output, checker, family, reports, raw_reports):
    rows, contrast = rows_and_contrast(checker, family, reports)
    checker.core.require(type(raw_reports) is list and len(raw_reports) == 2, "exact source reports")
    for report, raw in zip(reports, raw_reports):
        checker.core.same(checker.core.json_value(raw), report, "only exact public reports may be written")
    encoded = json.dumps(contrast, indent=2, sort_keys=True, allow_nan=False) + "\n"
    output.mkdir(parents=True, exist_ok=False)
    title = COHORTS[family]["title"]
    interval_plot(output / "intervals.svg", checker, family, rows, title)
    drawing.plots.rates(output / "rates.svg", [(row["label"], row["post_first_tokens_per_second"]) for row in rows], title)
    for name, raw in zip(("control-report.json", "candidate-report.json"), raw_reports):
        with (output / name).open("xb") as stream:
            stream.write(raw)
    with (output / "intervals.csv").open("x", newline="", encoding="utf-8") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerow(["variant", "output_token_ordinal", "decode_interval_ms"])
        for row in rows:
            writer.writerows((row["variant"], index, f"{value * 1000:.9f}")
                             for index, value in enumerate(row["decode_intervals_seconds"], 2))
    lines = ["| Variant | TTFT (s) | Mean TPOT (s) | Post-first tokens/s | Observed rate / control | Setup (s) |",
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


def run(family, control_capture, candidate_capture, reference_path, output):
    checker = load_checker(family)
    reference = checker.core.read_bounded(reference_path, 131072)
    pairs = [revalidate(checker, family, directory, variant, reference) for directory, variant in
             zip((control_capture, candidate_capture), COHORTS[family]["variants"])]
    generate(output, checker, family, [pair[0] for pair in pairs], [pair[1] for pair in pairs])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--family", choices=tuple(COHORTS), required=True)
    for name in ("control-capture", "candidate-capture", "reference", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    run(args.family, args.control_capture, args.candidate_capture, args.reference, args.output)
    print("Both captures independently revalidated; 62 actual intervals plotted; no GPU work.")


if __name__ == "__main__":
    main()
