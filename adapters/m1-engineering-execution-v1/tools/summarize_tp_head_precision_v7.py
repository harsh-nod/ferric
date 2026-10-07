#!/usr/bin/env python3
"""Matched v7 host-wall summaries after exact v7 reference and sidecar validation."""

import argparse
import copy
import importlib.util
import json
from pathlib import Path


HERE = Path(__file__).resolve().parent


def module(name):
    spec = importlib.util.spec_from_file_location(name, HERE / (name + ".py"))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


V7 = module("compare_tp_head_precision_v7")
TIMING = module("host_timing_summary")
LEDGER = TIMING.LEDGER
CHECK = V7.CHECK
SOURCES = {
    "compare_tp_head_precision_v7.py": "3c69090b82fe578a25f4ef7f870b9cc2a634e41f9934f9714cc5fa6f9e07fdbc",
    "compare_tp_batch.py": "1be4f2b3b0f8d1c96ef0b5bf21388da232d6b23e5700c7bef6295117b6093b6a",
    "host_timing_summary.py": "057f4a6dfb929db44714f8192bb1e373184f4170ce3946dcc31be811637b2402",
    "performance_ledger.py": "6da3b95acda3332bfa27a1c51685dda316d5f188615b9b939554d5eef8c9085e",
}
PROFILES = {"bf16-control": ("bf16-v7-control", "baseline"),
            "fp32-baseline": ("fp32-v7", "baseline"), "fp32-mfma": ("fp32-v7", "mfma")}
PAIRS = {("bf16-control", "fp32-baseline"), ("fp32-baseline", "fp32-mfma")}
SCHEMA = "FerricTpHeadPrecisionTimingManifestV1"
RUN_FIELDS = {"id", "run_dir", "workload", "reference", "comparison", "comparison_sha256", "timing", "timing_sha256"}


def canonical_path(value):
    CHECK.require(type(value) is str and Path(value).is_absolute()
                  and Path(value).resolve(strict=True) == Path(value), "canonical retained path required")
    return Path(value)


def validate_manifest(value):
    for name, digest in SOURCES.items():
        V7.exact(CHECK.sha256(CHECK.read_bounded(HERE / name, 128 * 1024)), digest, "frozen v7 dependency " + name)
    CHECK.fields(value, {"schema", "warmup_policy", "variants", "pairs"}, "v7 timing manifest")
    CHECK.require(value["schema"] == SCHEMA and value["warmup_policy"] == "fresh-worker-no-warmup", "v7 timing policy")
    CHECK.require(type(value["variants"]) is list and 1 <= len(value["variants"]) <= 3, "v7 variant count")
    names, ids, paths, expectations, common = set(), set(), set(), {}, None
    for variant in value["variants"]:
        CHECK.fields(variant, {"name", "expectation", "expectation_sha256", "runs"}, "v7 timing variant")
        name = variant["name"]
        CHECK.require(name in PROFILES and name not in names, "v7 variant role")
        names.add(name)
        raw = CHECK.read_bounded(canonical_path(variant["expectation"]), 65536)
        CHECK.hash_value(variant["expectation_sha256"], "expectation digest")
        V7.exact(CHECK.sha256(raw), variant["expectation_sha256"], "external expectation file")
        expected = V7.expectation(CHECK.json_value(raw))
        V7.exact([expected["head_precision"], expected["performance_profile"]["projection"]],
                 list(PROFILES[name]), "named v7 profile")
        matched = copy.deepcopy(expected)
        del matched["head_precision"], matched["performance_profile"]["projection"]
        if common is None:
            common = matched
        V7.exact(matched, common, "unmatched controller/worker/images/workload/reference/roster/policy")
        expectations[name] = expected
        CHECK.require(type(variant["runs"]) is list and 1 <= len(variant["runs"]) <= 8, "v7 repetitions bound")
        for run in variant["runs"]:
            CHECK.fields(run, RUN_FIELDS, "v7 timed run")
            CHECK.require(type(run["id"]) is str and 1 <= len(run["id"]) <= 128 and run["id"] not in ids, "run identity")
            ids.add(run["id"])
            for key in ("comparison_sha256", "timing_sha256"):
                CHECK.hash_value(run[key], key)
            for key in ("run_dir", "comparison", "timing"):
                path = str(canonical_path(run[key]))
                CHECK.require(path not in paths, "reused run/receipt/sidecar path")
                paths.add(path)
            for key in ("workload", "reference"):
                canonical_path(run[key])
    CHECK.require(type(value["pairs"]) is list and len(value["pairs"]) <= 2, "v7 pair count")
    seen = set()
    for pair in value["pairs"]:
        CHECK.fields(pair, {"baseline", "candidate"}, "v7 pair")
        identity = pair["baseline"], pair["candidate"]
        CHECK.require(identity in PAIRS and identity not in seen and set(identity) <= names,
                      "pair must change only head precision or only projection under FP32")
        seen.add(identity)
    return expectations


def load_run(run, expected, expectation_hash):
    prior_raw = CHECK.read_bounded(run["comparison"], 1024 * 1024)
    V7.exact(CHECK.sha256(prior_raw), run["comparison_sha256"], "pinned v7 comparison changed")
    fresh = V7.compare(Path(run["run_dir"]), Path(run["workload"]), Path(run["reference"]), expected)
    fresh.update(expectation_sha256=expectation_hash, comparator_sha256=SOURCES["compare_tp_head_precision_v7.py"])
    V7.exact(CHECK.json_value(prior_raw), fresh, "v7 comparison stale or not bound to current evidence")
    raw = CHECK.read_bounded(Path(run["run_dir"]) / "results.jsonl", 8 * 1024 * 1024)
    V7.exact(CHECK.sha256(raw), fresh["input_sha256"]["results.jsonl"], "v7 raw trace changed after qualification")
    records = [CHECK.json_value(line) for line in raw.splitlines()]
    sidecar = CHECK.read_bounded(run["timing"], 64 * 1024 * 1024)
    V7.exact(CHECK.sha256(sidecar), run["timing_sha256"], "pinned v7 timing changed")
    observations = TIMING.validate_sidecar(CHECK.json_value(sidecar), records, {**expected, "world": 1})
    aggregates = {}
    for row in observations:
        key = tuple(row[field] for field in TIMING.KEYS if field != "batch")
        total = aggregates.setdefault(key, {field: 0 for field in TIMING.TOTALS})
        for field in TIMING.TOTALS:
            total[field] = max(total[field], row[field]) if field == "max_ns" else total[field] + row[field]
    metrics = LEDGER.extract_metrics(records)
    metrics.update(id=run["id"], comparison_sha256=run["comparison_sha256"], timing_sha256=run["timing_sha256"],
                   input_sha256=fresh["input_sha256"], identities=fresh["identities"],
                   head_precision=fresh["head_precision"], fp32_head_workspace_bytes=fresh["fp32_head_workspace_bytes"],
                   batch_count=fresh["batch_count"], physical_token_rows=fresh["physical_token_rows"],
                   rank_dispatch_counts=fresh["rank_dispatch_counts"])
    return metrics, aggregates, {key: records[0][key] for key in LEDGER.EQUIVALENCE_FIELDS}


def aggregate(manifest):
    expectations = validate_manifest(manifest)
    variants, shared = [], None
    for variant in manifest["variants"]:
        runs, phases = [], []
        expected = expectations[variant["name"]]
        for run in variant["runs"]:
            metrics, observations, equivalence = load_run(run, expected, variant["expectation_sha256"])
            if shared is None:
                shared = equivalence
            V7.exact(equivalence, shared, "physical execution geometry differs")
            runs.append(metrics)
            phases.append(observations)
        requests = {}
        for name in CHECK.NAMES:
            values = [run["requests"][name] for run in runs]
            identity = {key: values[0][key] for key in ("slot", "generation", "state", "generated_tokens", "decode_interval_count")}
            for value in values:
                V7.exact({key: value[key] for key in identity}, identity, "request identity/gap count differs across repetitions")
            requests[name] = {**identity, "metrics": {key: LEDGER.statistics([value[key] for value in values])
                                                      for key in LEDGER.REQUEST_METRICS}}
        CHECK.require(all(set(phase) == set(phases[0]) for phase in phases), "timing observation roster differs across repetitions")
        observations = [{**dict(zip(("phase", "category", "label", "rank"), key, strict=True)),
                         "samples": [phase[key] for phase in phases], "n": len(phases),
                         "mean_elapsed_ns": sum(phase[key]["elapsed_ns"] for phase in phases) / len(phases)}
                        for key in sorted(phases[0], key=lambda key: tuple("" if part is None else str(part) for part in key))]
        variants.append({"name": variant["name"], "expected": expected, "repetitions": len(runs), "runs": runs,
                         "requests": requests, "host_observations": observations,
                         "metrics": {key: LEDGER.statistics([run[key] for run in runs]) for key in LEDGER.GLOBAL_METRICS}})
    by_name = {variant["name"]: variant for variant in variants}
    pairs = []
    for pair in manifest["pairs"]:
        baseline, candidate = (by_name[pair[key]] for key in ("baseline", "candidate"))
        base_phases = {tuple(row[key] for key in ("phase", "category", "label", "rank")): row for row in baseline["host_observations"]}
        ratios = []
        for row in candidate["host_observations"]:
            identity = tuple(row[key] for key in ("phase", "category", "label", "rank"))
            before = base_phases.get(identity)
            ratios.append({**dict(zip(("phase", "category", "label", "rank"), identity, strict=True)),
                           "candidate_over_baseline": row["mean_elapsed_ns"] / before["mean_elapsed_ns"]
                           if before and before["mean_elapsed_ns"] > 0 else None})
        pairs.append({**pair, "metrics": {key: LEDGER.relative(candidate["metrics"][key], baseline["metrics"][key],
                       key == "output_tokens_per_second") for key in LEDGER.GLOBAL_METRICS},
                      "requests": {name: {key: LEDGER.relative(candidate["requests"][name]["metrics"][key],
                       baseline["requests"][name]["metrics"][key], key == "output_tokens_per_second")
                       for key in LEDGER.REQUEST_METRICS} for name in CHECK.NAMES}, "host_phase_ratios": ratios})
    return {"schema": "FerricTpHeadPrecisionTimingSummaryV1", "authority": "none", "variants": variants, "pairs": pairs,
            "equivalence": shared, "warmup_policy": manifest["warmup_policy"], "generator_dependencies": SOURCES,
            "measurement": "host-wall-latency-not-gpu-duration", "sidecar_normalization": "none; original v7 Setup/Closed bind directly",
            "percentiles": "nearest rank; no request-identity pooling; small-n p95 is not a stable serving tail",
            "metric_definitions": {"ttft": "first output minus actual admission",
                "tpot": "per-request arithmetic mean of adjacent output gaps; null for the one-output cancellation",
                "throughput": "all eight outputs divided by last terminal event minus first admission; cancellation output included",
                "terminal_event": "last output when completed; cancelled_ns when cancelled"},
            "nonclaims": ["short fixed logical-tick workload, not steady-state serving or an SLO",
                          "nested phases and cross-rank transport observations overlap and are not additive",
                          "all variants include host instrumentation overhead; no GPU duration or syscall attribution",
                          "distinct v7 reports never automatically enter frozen BF16 ledgers"]}


def markdown(report):
    lines = ["# V7 Head-Precision Results", "", "Fixed short workload; host wall timing, not GPU duration or serving throughput.",
             "Repetitions remain explicit; small-n nearest-rank p95 is not a stable tail.", "",
             "| Variant | n | Output tok/s p50 | Window s p50 | Setup s p50 | Whole s p50 |", "|---|---:|---:|---:|---:|---:|"]
    for variant in report["variants"]:
        metrics = variant["metrics"]
        lines.append(f"| {variant['name']} | {variant['repetitions']} | " + " | ".join(
            f"{metrics[key]['p50']:.6f}" for key in ("output_tokens_per_second", "workload_window_seconds", "setup_seconds", "whole_seconds")) + " |")
    for name in CHECK.NAMES:
        lines += ["", "## " + name, "", "| Variant | n | Decode gaps/rep | TTFT s p50/p95 | TPOT s p50/p95 |", "|---|---:|---:|---:|---:|"]
        for variant in report["variants"]:
            request = variant["requests"][name]
            cells = ["n/a" if request["metrics"][key] is None else
                     f"{request['metrics'][key]['p50']:.6f}/{request['metrics'][key]['p95']:.6f}" for key in ("ttft_seconds", "tpot_seconds")]
            lines.append(f"| {variant['name']} | {variant['repetitions']} | {request['decode_interval_count']} | " + " | ".join(cells) + " |")
    for pair in report["pairs"]:
        lines += ["", f"## {pair['baseline']} to {pair['candidate']}", "",
                  f"Output throughput change: {pair['metrics']['output_tokens_per_second']['p50']['improvement_percent']:+.2f}%."]
    lines += ["", "## Named Host Spans", "", "These are selected matching phase spans, not the sum of all nested observations.", "",
              "| Variant | Phase | Mean wall seconds |", "|---|---|---:|"]
    for variant in report["variants"]:
        for row in variant["host_observations"]:
            if row["category"] == "span" and row["rank"] is None and row["phase"] == row["label"]:
                lines.append(f"| {variant['name']} | {row['phase']} | {row['mean_elapsed_ns'] / 1e9:.6f} |")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("manifest", "json-output", "markdown-output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    raw = CHECK.read_bounded(args.manifest, 4 * 1024 * 1024)
    report = aggregate(CHECK.json_value(raw))
    report.update(manifest_sha256=CHECK.sha256(raw), generator_sha256=CHECK.sha256(CHECK.read_bounded(Path(__file__), 128 * 1024)))
    with args.json_output.open("x", encoding="utf-8") as output:
        json.dump(report, output, indent=2, sort_keys=True, allow_nan=False)
        output.write("\n")
    with args.markdown_output.open("x", encoding="utf-8") as output:
        output.write(markdown(report))


if __name__ == "__main__":
    main()
